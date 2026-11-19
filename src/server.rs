use crate::api::ApiRouter;
use crate::auth::{AuthContext, Role};
use crate::db::DbHandle;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub static_dir: PathBuf,
    pub db_dir: PathBuf,
}

pub struct HttpServer {
    config: ServerConfig,
    db: DbHandle,
    meet_signals: crate::api::meet::MeetSignalingHandle,
    is_running: Arc<AtomicBool>,
}

impl HttpServer {
    pub fn new(config: ServerConfig, db: DbHandle) -> Self {
        Self {
            config,
            db,
            meet_signals: Arc::new(std::sync::Mutex::new(
                crate::api::meet::MeetSignalingState::new(),
            )),
            is_running: Arc::new(AtomicBool::new(true)),
        }
    }

    pub fn run(&self) -> std::io::Result<()> {
        let addr = format!("{}:{}", self.config.host, self.config.port);
        let listener = TcpListener::bind(&addr)?;
        println!("🚀 BijiOS Server berjalan di http://{}", addr);

        for stream in listener.incoming() {
            if !self.is_running.load(Ordering::Relaxed) {
                break;
            }

            match stream {
                Ok(stream) => {
                    let db = Arc::clone(&self.db);
                    let meet_signals = Arc::clone(&self.meet_signals);
                    let static_dir = self.config.static_dir.clone();
                    let db_dir = self.config.db_dir.clone();

                    thread::spawn(move || {
                        if let Err(e) = Self::handle_connection(
                            stream,
                            &db,
                            &meet_signals,
                            &static_dir,
                            &db_dir,
                        ) {
                            eprintln!("Error handling connection: {}", e);
                        }
                    });
                }
                Err(e) => {
                    eprintln!("Connection failed: {}", e);
                }
            }
        }

        Ok(())
    }

    fn handle_connection(
        mut stream: TcpStream,
        db: &DbHandle,
        meet_signals: &crate::api::meet::MeetSignalingHandle,
        static_dir: &Path,
        db_dir: &Path,
    ) -> std::io::Result<()> {
        let mut reader = BufReader::new(&stream);
        let mut request_line = String::new();

        if reader.read_line(&mut request_line)? == 0 {
            return Ok(());
        }

        let parts: Vec<&str> = request_line.split_whitespace().collect();
        if parts.len() < 2 {
            return Ok(());
        }

        let method = parts[0];
        let full_path = parts[1];

        // Parse path and query
        let (path, query) = match full_path.split_once('?') {
            Some((p, q)) => (p, q),
            None => (full_path, ""),
        };

        // Read headers
        let mut headers = Vec::new();
        let mut content_length: usize = 0;
        let mut auth_token: Option<String> = None;
        let mut tenant_header: Option<String> = None;

        loop {
            let mut line = String::new();
            if reader.read_line(&mut line)? == 0 || line == "\r\n" || line == "\n" {
                break;
            }
            let trimmed = line.trim();
            if let Some((k, v)) = trimmed.split_once(':') {
                let k_lower = k.trim().to_lowercase();
                let val = v.trim();
                if k_lower == "content-length" {
                    content_length = val.parse().unwrap_or(0);
                } else if k_lower == "authorization" {
                    let token_str = if let Some(token) = val.strip_prefix("Bearer ") {
                        token.trim()
                    } else if let Some(token) = val.strip_prefix("bearer ") {
                        token.trim()
                    } else {
                        val.trim()
                    };
                    auth_token = Some(token_str.to_string());
                } else if k_lower == "x-tenant-id" {
                    tenant_header = Some(val.to_string());
                }
                headers.push((k_lower, val.to_string()));
            }
        }

        // Also check query param ?token= or ?auth=
        if auth_token.is_none() {
            for param in query.split('&') {
                if let Some((k, v)) = param.split_once('=') {
                    if k == "token" || k == "auth" {
                        auth_token = Some(v.to_string());
                    }
                }
            }
        }

        // Handle CORS preflight
        if method == "OPTIONS" {
            let resp = "HTTP/1.1 204 No Content\r\n\
                Access-Control-Allow-Origin: *\r\n\
                Access-Control-Allow-Methods: GET, POST, PUT, DELETE, OPTIONS\r\n\
                Access-Control-Allow-Headers: Authorization, Content-Type, X-Tenant-Id\r\n\
                Access-Control-Max-Age: 86400\r\n\
                Content-Length: 0\r\n\r\n";
            stream.write_all(resp.as_bytes())?;
            return Ok(());
        }

        // Read request body
        let mut body = String::new();
        if content_length > 0 {
            let mut buf = vec![0u8; content_length];
            reader.read_exact(&mut buf)?;
            body = String::from_utf8_lossy(&buf).to_string();
        }

        // Determine AuthContext (Strict Session Validation)
        let maybe_ctx = Self::resolve_auth(auth_token.as_deref(), tenant_header.as_deref(), db);

        // Check if API route
        if path.starts_with("/api") {
            let is_public = path == "/api/auth/login"
                || path.starts_with("/api/shares/public/")
                || path == "/api/health"
                || path == "/api/system/metrics"
                || path == "/api/os/status"
                || path == "/api/os/power"
                || (path.starts_with("/api/files/") && path.ends_with("/download"))
                || (path.starts_with("/api/users/") && path.ends_with("/avatar"));

            if !is_public && maybe_ctx.is_none() {
                let resp = "HTTP/1.1 401 Unauthorized\r\n\
                    Content-Type: application/json\r\n\
                    Access-Control-Allow-Origin: *\r\n\
                    Access-Control-Allow-Methods: GET, POST, PUT, DELETE, OPTIONS\r\n\
                    Access-Control-Allow-Headers: Authorization, Content-Type, X-Tenant-Id\r\n\
                    Connection: close\r\n\r\n\
                    {\"error\":\"Unauthorized. Valid session token required.\"}";
                let _ = stream.write_all(resp.as_bytes());
                return Ok(());
            }

            let guest_ctx = AuthContext::guest();
            let ctx = maybe_ctx.as_ref().unwrap_or(&guest_ctx);

            // Check for physical file download
            if path.starts_with("/api/files/") && path.ends_with("/download") {
                let file_id = &path["/api/files/".len()..path.len() - "/download".len()];
                let db_lock = db.read().unwrap();
                if let Some(f) = db_lock.files.iter().find(|x| x.id == file_id) {
                    if maybe_ctx.is_some() && ctx.role != Role::SuperAdmin && f.tenant_id != ctx.tenant_id {
                        let resp = "HTTP/1.1 403 Forbidden\r\n\
                            Content-Type: application/json\r\n\
                            Access-Control-Allow-Origin: *\r\n\
                            Connection: close\r\n\r\n\
                            {\"error\":\"Forbidden. File is in a different tenant.\"}";
                        let _ = stream.write_all(resp.as_bytes());
                        return Ok(());
                    }

                    let storage_base = db_dir.parent().unwrap_or(db_dir).join("storage");
                    let direct_path = storage_base.join(&f.tenant_id).join(&f.owner_id).join(&f.name);
                    let actual_path = if direct_path.exists() {
                        direct_path
                    } else {
                        let mut found = direct_path.clone();
                        let user_dir = storage_base.join(&f.tenant_id).join(&f.owner_id);
                        if let Ok(entries) = fs::read_dir(&user_dir) {
                            for e in entries.flatten() {
                                let p = e.path();
                                if p.is_dir() && p.join(&f.name).exists() {
                                    found = p.join(&f.name);
                                    break;
                                }
                            }
                        }
                        found
                    };

                    let (bytes_opt, is_inline) = if actual_path.exists() && actual_path.is_file() {
                        if let Ok(raw_bytes) = fs::read(&actual_path) {
                            if (f.extension == "xlsx" || f.name.to_lowercase().ends_with(".xlsx")) && !raw_bytes.starts_with(b"PK\x03\x04") {
                                if let Ok(text) = String::from_utf8(raw_bytes.clone()) {
                                    let pkg = crate::xlsx::XlsxEngine::create_xlsx_package(&text);
                                    let _ = fs::write(&actual_path, &pkg);
                                    (Some(pkg), false)
                                } else {
                                    (Some(raw_bytes), false)
                                }
                            } else if (f.extension == "pptx" || f.name.to_lowercase().ends_with(".pptx")) && !raw_bytes.starts_with(b"PK\x03\x04") {
                                if let Ok(text) = String::from_utf8(raw_bytes.clone()) {
                                    let pkg = crate::pptx::PptxEngine::create_pptx_package(&text);
                                    let _ = fs::write(&actual_path, &pkg);
                                    (Some(pkg), false)
                                } else {
                                    (Some(raw_bytes), false)
                                }
                            } else {
                                (Some(raw_bytes), f.mime_type.starts_with("image/") || f.mime_type == "application/pdf")
                            }
                        } else {
                            (None, false)
                        }
                    } else if f.content.starts_with("data:") {
                        if let Some(comma) = f.content.find(',') {
                            (crate::api::files::FilesApi::decode_base64(&f.content[comma + 1..]), true)
                        } else {
                            (None, false)
                        }
                    } else if !f.content.is_empty() {
                        if (f.extension == "xlsx" || f.name.to_lowercase().ends_with(".xlsx")) && !f.content.as_bytes().starts_with(b"PK\x03\x04") {
                            (Some(crate::xlsx::XlsxEngine::create_xlsx_package(&f.content)), false)
                        } else if (f.extension == "pptx" || f.name.to_lowercase().ends_with(".pptx")) && !f.content.as_bytes().starts_with(b"PK\x03\x04") {
                            (Some(crate::pptx::PptxEngine::create_pptx_package(&f.content)), false)
                        } else {
                            (Some(f.content.as_bytes().to_vec()), f.mime_type.starts_with("image/") || f.mime_type == "application/pdf")
                        }
                    } else {
                        (None, false)
                    };

                    if let Some(bytes) = bytes_opt {
                        let disp = if is_inline { "inline" } else { "attachment" };
                        let headers = format!(
                            "HTTP/1.1 200 OK\r\n\
                            Content-Type: {}\r\n\
                            Content-Disposition: {}; filename=\"{}\"\r\n\
                            Content-Length: {}\r\n\
                            Access-Control-Allow-Origin: *\r\n\
                            Connection: close\r\n\r\n",
                            f.mime_type, disp, f.name, bytes.len()
                        );
                        let _ = stream.write_all(headers.as_bytes());
                        let _ = stream.write_all(&bytes);
                        return Ok(());
                    }
                }
            }

            // Check for user avatar image
            if path.starts_with("/api/users/") && path.ends_with("/avatar") {
                let user_id = &path["/api/users/".len()..path.len() - "/avatar".len()];
                let avatar_path = std::path::PathBuf::from("/home/dokuhub/apps/dokuhub/data/storage/avatars")
                    .join(format!("{}.png", user_id));
                if avatar_path.exists() {
                    if let Ok(bytes) = std::fs::read(&avatar_path) {
                        let headers = format!(
                            "HTTP/1.1 200 OK\r\n\
                            Content-Type: image/png\r\n\
                            Content-Disposition: inline; filename=\"avatar_{}.png\"\r\n\
                            Content-Length: {}\r\n\
                            Cache-Control: public, max-age=86400\r\n\
                            Access-Control-Allow-Origin: *\r\n\
                            Connection: close\r\n\r\n",
                            user_id, bytes.len()
                        );
                        let _ = stream.write_all(headers.as_bytes());
                        let _ = stream.write_all(&bytes);
                        return Ok(());
                    }
                }
                let resp = "HTTP/1.1 404 Not Found\r\n\
                    Content-Type: application/json\r\n\
                    Access-Control-Allow-Origin: *\r\n\
                    Connection: close\r\n\r\n\
                    {\"error\":\"Avatar image not found\"}";
                let _ = stream.write_all(resp.as_bytes());
                return Ok(());
            }

            // Export XLSX on the fly (returns ISO/IEC 29500 OpenXML binary)
            if method == "POST" && path == "/api/files/export_xlsx" {
                let xlsx_bytes = crate::xlsx::XlsxEngine::create_xlsx_package(&body);
                let title = if let Some(j) = crate::api::json_helper::parse_json(&body) {
                    j.get_str("title").unwrap_or("Spreadsheet.xlsx").to_string()
                } else {
                    "Spreadsheet.xlsx".to_string()
                };
                let filename = if title.to_lowercase().ends_with(".xlsx") {
                    title
                } else {
                    format!("{}.xlsx", title)
                };
                let headers = format!(
                    "HTTP/1.1 200 OK\r\n\
                    Content-Type: application/vnd.openxmlformats-officedocument.spreadsheetml.sheet\r\n\
                    Content-Disposition: attachment; filename=\"{}\"\r\n\
                    Content-Length: {}\r\n\
                    Access-Control-Allow-Origin: *\r\n\
                    Access-Control-Allow-Methods: GET, POST, PUT, DELETE, OPTIONS\r\n\
                    Access-Control-Allow-Headers: Authorization, Content-Type, X-Tenant-Id\r\n\
                    Connection: close\r\n\r\n",
                    filename, xlsx_bytes.len()
                );
                let _ = stream.write_all(headers.as_bytes());
                let _ = stream.write_all(&xlsx_bytes);
                return Ok(());
            }

            // Export PPTX on the fly (returns ISO/IEC 29500 OpenXML PowerPoint binary)
            if method == "POST" && path == "/api/files/export_pptx" {
                let pptx_bytes = crate::pptx::PptxEngine::create_pptx_package(&body);
                let title = if let Some(j) = crate::api::json_helper::parse_json(&body) {
                    j.get_str("title").unwrap_or("Presentation.pptx").to_string()
                } else {
                    "Presentation.pptx".to_string()
                };
                let filename = if title.to_lowercase().ends_with(".pptx") {
                    title
                } else {
                    format!("{}.pptx", title)
                };
                let headers = format!(
                    "HTTP/1.1 200 OK\r\n\
                    Content-Type: application/vnd.openxmlformats-officedocument.presentationml.presentation\r\n\
                    Content-Disposition: attachment; filename=\"{}\"\r\n\
                    Content-Length: {}\r\n\
                    Access-Control-Allow-Origin: *\r\n\
                    Access-Control-Allow-Methods: GET, POST, PUT, DELETE, OPTIONS\r\n\
                    Access-Control-Allow-Headers: Authorization, Content-Type, X-Tenant-Id\r\n\
                    Connection: close\r\n\r\n",
                    filename, pptx_bytes.len()
                );
                let _ = stream.write_all(headers.as_bytes());
                let _ = stream.write_all(&pptx_bytes);
                return Ok(());
            }

            let client_ip = headers
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case("x-forwarded-for"))
                .and_then(|(_, v)| v.split(',').next())
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|| {
                    stream
                        .peer_addr()
                        .map(|a| a.ip().to_string())
                        .unwrap_or_else(|_| "127.0.0.1".to_string())
                });

            if let Some((code, resp_body, content_type)) = ApiRouter::route(
                method,
                path,
                query,
                &body,
                ctx,
                db,
                meet_signals,
                db_dir,
                &client_ip,
            ) {
                let status_text = match code {
                    200 => "OK",
                    201 => "Created",
                    204 => "No Content",
                    400 => "Bad Request",
                    401 => "Unauthorized",
                    403 => "Forbidden",
                    404 => "Not Found",
                    _ => "Internal Server Error",
                };

                let headers = format!(
                    "HTTP/1.1 {} {}\r\n\
                    Content-Type: {}\r\n\
                    Content-Length: {}\r\n\
                    Access-Control-Allow-Origin: *\r\n\
                    Access-Control-Allow-Methods: GET, POST, PUT, DELETE, OPTIONS\r\n\
                    Access-Control-Allow-Headers: Authorization, Content-Type, X-Tenant-Id\r\n\
                    Connection: close\r\n\r\n",
                    code,
                    status_text,
                    content_type,
                    resp_body.len()
                );

                stream.write_all(headers.as_bytes())?;
                stream.write_all(resp_body.as_bytes())?;
                return Ok(());
            }
        }

        // Serve Static Files
        Self::serve_static(path, static_dir, &mut stream)?;
        Ok(())
    }

    fn resolve_auth(token: Option<&str>, tenant: Option<&str>, db: &DbHandle) -> Option<AuthContext> {
        let db_lock = db.read().unwrap();

        if let Some(tok) = token {
            let tok = tok.trim();
            // Supports "dkh_token:{user_id}:{tenant_id}" and legacy "dkh_token_{user_id}_{tenant_id}"
            let clean = tok
                .strip_prefix("dkh_token:")
                .or_else(|| tok.strip_prefix("dkh_token_"));

            if let Some(rest) = clean {
                let user_id = if let Some((u, _)) = rest.split_once(':') {
                    u
                } else {
                    rest
                };

                if let Some(user) = db_lock
                    .users
                    .iter()
                    .find(|u| u.id == user_id || rest.starts_with(&u.id))
                {
                    if user.status == "suspended" {
                        return None;
                    }

                    let active_tenant = if user.role == Role::SuperAdmin {
                        tenant
                            .map(|t| t.to_string())
                            .unwrap_or_else(|| user.tenant_id.clone())
                    } else {
                        user.tenant_id.clone()
                    };

                    return Some(AuthContext {
                        user_id: user.id.clone(),
                        tenant_id: active_tenant,
                        email: user.email.clone(),
                        name: user.name.clone(),
                        role: user.role,
                    });
                }
            }
        }

        None
    }

    fn serve_static(path: &str, static_dir: &Path, stream: &mut TcpStream) -> std::io::Result<()> {
        let clean_path = path.trim_start_matches('/');
        let target_path = if clean_path.is_empty() || clean_path.starts_with("share/") {
            static_dir.join("index.html")
        } else {
            let candidate = static_dir.join(clean_path);
            if candidate.is_file() {
                candidate
            } else {
                static_dir.join("index.html")
            }
        };

        if !target_path.exists() {
            let not_found = "HTTP/1.1 404 Not Found\r\nContent-Length: 13\r\n\r\n404 Not Found";
            stream.write_all(not_found.as_bytes())?;
            return Ok(());
        }

        let content = match fs::read(&target_path) {
            Ok(c) => c,
            Err(e) => {
                let err_resp = format!("HTTP/1.1 500 Internal Error\r\nContent-Length: {}\r\n\r\n{}", e.to_string().len(), e);
                stream.write_all(err_resp.as_bytes())?;
                return Ok(());
            }
        };

        let mime = Self::get_mime_type(&target_path);
        let header = format!(
            "HTTP/1.1 200 OK\r\n\
            Content-Type: {}\r\n\
            Content-Length: {}\r\n\
            Cache-Control: no-cache\r\n\
            Access-Control-Allow-Origin: *\r\n\
            Connection: close\r\n\r\n",
            mime,
            content.len()
        );

        stream.write_all(header.as_bytes())?;
        stream.write_all(&content)?;
        Ok(())
    }

    fn get_mime_type(path: &Path) -> &'static str {
        match path.extension().and_then(|ext| ext.to_str()).unwrap_or("") {
            "html" => "text/html; charset=utf-8",
            "js" | "mjs" => "application/javascript; charset=utf-8",
            "css" => "text/css; charset=utf-8",
            "json" => "application/json; charset=utf-8",
            "svg" => "image/svg+xml",
            "png" => "image/png",
            "jpg" | "jpeg" => "image/jpeg",
            "bmp" => "image/bmp",
            "webp" => "image/webp",
            "gif" => "image/gif",
            "ico" => "image/x-icon",
            "pdf" => "application/pdf",
            "woff2" => "font/woff2",
            "woff" => "font/woff",
            "ttf" => "font/ttf",
            "toon" => "text/plain; charset=utf-8",
            _ => "text/plain; charset=utf-8",
        }
    }
}

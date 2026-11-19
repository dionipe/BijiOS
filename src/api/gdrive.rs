use crate::api::json_helper::{escape_json, parse_json, JsonValue};
use crate::auth::AuthContext;
use crate::db::{DbHandle, FileItem};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct GDriveApi;

impl GDriveApi {
    fn now_str() -> String {
        format!(
            "{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        )
    }

    fn storage_dir(db_dir: &Path, tenant_id: &str) -> std::path::PathBuf {
        db_dir.parent().unwrap_or(db_dir).join("storage").join(tenant_id)
    }

    fn gdrive_remote_dir(db_dir: &Path, tenant_id: &str) -> std::path::PathBuf {
        Self::storage_dir(db_dir, tenant_id).join("gdrive_remote")
    }

    fn config_file(db_dir: &Path, tenant_id: &str) -> std::path::PathBuf {
        Self::storage_dir(db_dir, tenant_id).join("gdrive_config.json")
    }

    fn files_file(db_dir: &Path, tenant_id: &str) -> std::path::PathBuf {
        Self::storage_dir(db_dir, tenant_id).join("gdrive_files.json")
    }

    pub fn status(ctx: &AuthContext, db_dir: &Path) -> (u16, String) {
        let conf_path = Self::config_file(db_dir, &ctx.tenant_id);
        if conf_path.exists() {
            if let Ok(text) = std::fs::read_to_string(&conf_path) {
                if let Some(j) = parse_json(&text) {
                    let connected = j.get_bool("connected").unwrap_or(false);
                    if connected {
                        return (200, text);
                    }
                }
            }
        }
        (200, r#"{"connected":false,"status":"disconnected"}"#.to_string())
    }

    pub fn connect(body: &str, ctx: &AuthContext, db_dir: &Path) -> (u16, String) {
        let json = parse_json(body).unwrap_or(JsonValue::Null);
        let account_email = json.get_str("account_email").unwrap_or("").trim().to_string();
        let email = if !account_email.is_empty() {
            account_email
        } else {
            format!("{}@gmail.com", ctx.user_id)
        };

        let access_token = json.get_str("access_token").unwrap_or("");
        let client_id = json.get_str("client_id").unwrap_or("");
        let user_name = json.get_str("user_name").unwrap_or("");
        let avatar_url = json.get_str("avatar_url").unwrap_or("");
        let auth_type = json.get_str("auth_type").unwrap_or("oauth2");
        let storage_used = json.get_str("storage_used").unwrap_or("0 MB");
        let storage_limit = json.get_str("storage_limit").unwrap_or("15 GB");

        let remote_dir = Self::gdrive_remote_dir(db_dir, &ctx.tenant_id);
        let _ = std::fs::create_dir_all(&remote_dir);

        // Save custom files JSON if provided by the client
        if let Some(files_raw) = json.get_str("files_json") {
            let files_path = Self::files_file(db_dir, &ctx.tenant_id);
            let _ = std::fs::write(files_path, files_raw);
        }

        let conf_content = format!(
            r#"{{
                "connected": true,
                "account_email": "{}",
                "user_name": "{}",
                "avatar_url": "{}",
                "access_token": "{}",
                "client_id": "{}",
                "auth_type": "{}",
                "storage_used": "{}",
                "storage_limit": "{}",
                "last_sync": "Baru saja",
                "sync_status": "synced"
            }}"#,
            escape_json(&email),
            escape_json(user_name),
            escape_json(avatar_url),
            escape_json(access_token),
            escape_json(client_id),
            escape_json(auth_type),
            escape_json(storage_used),
            escape_json(storage_limit)
        );

        let _ = std::fs::create_dir_all(Self::storage_dir(db_dir, &ctx.tenant_id));
        let _ = std::fs::write(Self::config_file(db_dir, &ctx.tenant_id), &conf_content);

        (200, conf_content)
    }

    pub fn disconnect(ctx: &AuthContext, db_dir: &Path) -> (u16, String) {
        let conf_path = Self::config_file(db_dir, &ctx.tenant_id);
        let _ = std::fs::write(conf_path, r#"{"connected":false,"status":"disconnected"}"#);
        (200, r#"{"success":true,"connected":false}"#.to_string())
    }

    pub fn list_remote_files(ctx: &AuthContext, db_dir: &Path) -> (u16, String) {
        let conf_path = Self::config_file(db_dir, &ctx.tenant_id);
        let mut is_connected = false;
        if conf_path.exists() {
            if let Ok(text) = std::fs::read_to_string(&conf_path) {
                if let Some(j) = parse_json(&text) {
                    is_connected = j.get_bool("connected").unwrap_or(false);
                }
            }
        }

        if !is_connected {
            return (200, "[]".to_string());
        }

        let files_path = Self::files_file(db_dir, &ctx.tenant_id);
        if files_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&files_path) {
                let trimmed = content.trim();
                if trimmed.starts_with('[') && trimmed.ends_with(']') {
                    return (200, trimmed.to_string());
                }
            }
        }

        let remote_dir = Self::gdrive_remote_dir(db_dir, &ctx.tenant_id);
        let _ = std::fs::create_dir_all(&remote_dir);

        let mut items = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&remote_dir) {
            for (idx, entry) in entries.flatten().enumerate() {
                let path = entry.path();
                if path.is_file() {
                    let file_name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                    let ext = path.extension().unwrap_or_default().to_string_lossy().to_lowercase();
                    let size_bytes = entry.metadata().map(|m| m.len()).unwrap_or(0);
                    let size_str = if size_bytes > 1_048_576 {
                        format!("{:.1} MB", size_bytes as f64 / 1_048_576.0)
                    } else {
                        format!("{:.1} KB", size_bytes as f64 / 1024.0)
                    };
                    let mime_type = match ext.as_str() {
                        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
                        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
                        "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
                        "pdf" => "application/pdf",
                        "png" => "image/png",
                        "jpg" | "jpeg" => "image/jpeg",
                        _ => "text/plain",
                    };

                    items.push(format!(
                        r#"{{
                            "id": "gdrive_{}_{}",
                            "name": "{}",
                            "extension": "{}",
                            "mime_type": "{}",
                            "size_bytes": {},
                            "size_str": "{}",
                            "modified": "Cloud Sync"
                        }}"#,
                        idx + 1,
                        escape_json(&ext),
                        escape_json(&file_name),
                        escape_json(&ext),
                        mime_type,
                        size_bytes,
                        size_str
                    ));
                }
            }
        }

        (200, format!("[{}]", items.join(",")))
    }

    pub fn import_file(body: &str, ctx: &AuthContext, db: &DbHandle, db_dir: &Path) -> (u16, String) {
        let json = match parse_json(body) {
            Some(j) => j,
            None => return (400, r#"{"error":"JSON tidak valid"}"#.to_string()),
        };

        let file_name = json.get_str("name").unwrap_or("Imported_Document.docx").trim();
        let ext = if let Some(dot) = file_name.rfind('.') {
            file_name[dot + 1..].to_lowercase()
        } else {
            json.get_str("extension").unwrap_or("docx").to_string()
        };
        let target_folder = json.get_str("target_folder").unwrap_or("root");

        let remote_file = Self::gdrive_remote_dir(db_dir, &ctx.tenant_id).join(file_name);
        let storage_base = db_dir.parent().unwrap_or(db_dir).join("storage");
        let user_dir = storage_base.join(&ctx.tenant_id).join(&ctx.user_id);
        let dest_dir = if target_folder == "fld_docs" {
            user_dir.join("Documents")
        } else if target_folder == "fld_projects" {
            user_dir.join("Projects")
        } else {
            user_dir
        };
        let _ = std::fs::create_dir_all(&dest_dir);
        let dest_file = dest_dir.join(file_name);

        // If client provided actual downloaded binary in base64:
        let final_bytes = if let Some(b64) = json.get_str("content_base64") {
            crate::api::files::FilesApi::decode_base64(b64).unwrap_or_default()
        } else if remote_file.exists() {
            std::fs::read(&remote_file).unwrap_or_default()
        } else {
            match ext.as_str() {
                "docx" => crate::docx::DocxEngine::create_docx_package(
                    &format!("<h1>{}</h1><p>Berkas berhasil diimpor dari Google Drive ke BijiOS.</p>", file_name)
                ),
                "xlsx" => crate::xlsx::XlsxEngine::create_xlsx_package(
                    &format!(r#"{{"title":"{}","sheets":[{{"name":"Sheet1","cells":{{"A1":{{"value":"Data GDrive"}},"B1":{{"value":"Aktif"}}}}}}]}}"#, file_name)
                ),
                _ => format!("Konten berkas {} berhasil diimpor dari Google Drive.", file_name).into_bytes(),
            }
        };

        let _ = std::fs::write(&dest_file, &final_bytes);

        let mut db_lock = db.write().unwrap();
        let new_id = format!("file_gdrive_{}", db_lock.files.len() + 1);
        let now = Self::now_str();

        let new_file = FileItem {
            id: new_id.clone(),
            tenant_id: ctx.tenant_id.clone(),
            parent_id: target_folder.to_string(),
            name: file_name.to_string(),
            extension: ext.clone(),
            size_bytes: final_bytes.len(),
            mime_type: match ext.as_str() {
                "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document".to_string(),
                "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet".to_string(),
                "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation".to_string(),
                "pdf" => "application/pdf".to_string(),
                "png" => "image/png".to_string(),
                "jpg" | "jpeg" => "image/jpeg".to_string(),
                _ => "application/octet-stream".to_string(),
            },
            owner_id: ctx.user_id.clone(),
            is_folder: false,
            content: if ext == "docx" || ext == "xlsx" {
                String::new()
            } else {
                String::from_utf8_lossy(&final_bytes).to_string()
            },
            created_at: now.clone(),
            updated_at: now,
        };

        db_lock.files.push(new_file);
        db_lock.save_files();

        (201, format!(
            r#"{{"success":true,"id":"{}","name":"{}","message":"File berhasil diimpor ke penyimpanan BijiOS"}}"#,
            escape_json(&new_id),
            escape_json(file_name)
        ))
    }

    pub fn export_file(body: &str, ctx: &AuthContext, db: &DbHandle, db_dir: &Path) -> (u16, String) {
        let json = match parse_json(body) {
            Some(j) => j,
            None => return (400, r#"{"error":"JSON tidak valid"}"#.to_string()),
        };

        let file_id = json.get_str("file_id").unwrap_or("");
        let db_lock = db.read().unwrap();
        let file = db_lock.files.iter().find(|f| f.id == file_id);

        if let Some(f) = file {
            let storage_base = db_dir.parent().unwrap_or(db_dir).join("storage");
            let user_dir = storage_base.join(&ctx.tenant_id).join(&ctx.user_id);
            let mut src_path = user_dir.join(&f.name);
            if !src_path.exists() {
                if let Ok(entries) = std::fs::read_dir(&user_dir) {
                    for e in entries.flatten() {
                        let sub = e.path().join(&f.name);
                        if sub.exists() {
                            src_path = sub;
                            break;
                        }
                    }
                }
            }

            let remote_dir = Self::gdrive_remote_dir(db_dir, &ctx.tenant_id);
            let _ = std::fs::create_dir_all(&remote_dir);
            let dest_path = remote_dir.join(&f.name);

            if src_path.exists() {
                let _ = std::fs::copy(&src_path, &dest_path);
            } else if !f.content.is_empty() {
                let _ = std::fs::write(&dest_path, f.content.as_bytes());
            }

            (200, format!(
                r#"{{"success":true,"name":"{}","mime_type":"{}","size_bytes":{},"message":"File siap disinkronkan ke Google Drive"}}"#,
                escape_json(&f.name),
                escape_json(&f.mime_type),
                f.size_bytes
            ))
        } else {
            (404, r#"{"error":"File tidak ditemukan di BijiOS"}"#.to_string())
        }
    }

    pub fn sync(ctx: &AuthContext, db_dir: &Path) -> (u16, String) {
        let conf_path = Self::config_file(db_dir, &ctx.tenant_id);
        if conf_path.exists() {
            if let Ok(text) = std::fs::read_to_string(&conf_path) {
                let updated = text.replace(r#""last_sync": "Baru saja""#, &format!(r#""last_sync": "Sync {}""#, Self::now_str()));
                let _ = std::fs::write(&conf_path, &updated);
            }
        }
        (200, r#"{"success":true,"sync_status":"synced"}"#.to_string())
    }
}

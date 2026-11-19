use crate::api::json_helper::{escape_json, parse_json};
use crate::auth::{AuthContext, Role};
use crate::db::{DbHandle, FileItem};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct FilesApi;

impl FilesApi {
    fn now_str() -> String {
        format!(
            "{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        )
    }

    pub fn ensure_user_storage(user_id: &str, tenant_id: &str, db: &DbHandle, db_dir: &std::path::Path) {
        let needs_init = {
            let db_lock = db.read().unwrap();
            !db_lock.files.iter().any(|f| f.owner_id == user_id && f.tenant_id == tenant_id)
        };

        if needs_init {
            let mut db_lock = db.write().unwrap();
            if !db_lock.files.iter().any(|f| f.owner_id == user_id && f.tenant_id == tenant_id) {
                let now = Self::now_str();
                let doc_id = format!("fld_docs_{}", user_id);
                let proj_id = format!("fld_projects_{}", user_id);
                let starter_file_id = format!("file_welcome_{}", user_id);

                let starter_content = r#"{"title":"Selamat Datang di BijiOS","content":"<p><strong>Selamat Datang di BijiOS Sovereign Workspace!</strong></p><p>Ini adalah ruang kerja mandiri yang sepenuhnya terisolasi milik akun Anda. Anda dapat membuat, mengunggah, dan mengedit berkas dokumen dengan aman.</p>"}"#;

                db_lock.files.push(FileItem {
                    id: doc_id.clone(),
                    tenant_id: tenant_id.to_string(),
                    parent_id: "root".to_string(),
                    name: "Documents".to_string(),
                    extension: "folder".to_string(),
                    size_bytes: 0,
                    mime_type: "inode/directory".to_string(),
                    owner_id: user_id.to_string(),
                    is_folder: true,
                    content: String::new(),
                    created_at: now.clone(),
                    updated_at: now.clone(),
                });

                db_lock.files.push(FileItem {
                    id: proj_id.clone(),
                    tenant_id: tenant_id.to_string(),
                    parent_id: "root".to_string(),
                    name: "Projects".to_string(),
                    extension: "folder".to_string(),
                    size_bytes: 0,
                    mime_type: "inode/directory".to_string(),
                    owner_id: user_id.to_string(),
                    is_folder: true,
                    content: String::new(),
                    created_at: now.clone(),
                    updated_at: now.clone(),
                });

                db_lock.files.push(FileItem {
                    id: starter_file_id,
                    tenant_id: tenant_id.to_string(),
                    parent_id: doc_id,
                    name: "Selamat Datang di BijiOS.docx".to_string(),
                    extension: "docx".to_string(),
                    size_bytes: starter_content.len(),
                    mime_type: "application/vnd.openxmlformats-officedocument.wordprocessingml.document".to_string(),
                    owner_id: user_id.to_string(),
                    is_folder: false,
                    content: starter_content.to_string(),
                    created_at: now.clone(),
                    updated_at: now,
                });

                db_lock.save_files();

                // Also ensure physical disk storage directories exist
                let storage_base = db_dir.parent().unwrap_or(db_dir).join("storage");
                let user_dir = storage_base.join(tenant_id).join(user_id);
                let _ = std::fs::create_dir_all(user_dir.join("Documents"));
                let _ = std::fs::create_dir_all(user_dir.join("Projects"));
                let docx_bytes = crate::docx::DocxEngine::create_docx_package(starter_content);
                let _ = std::fs::write(user_dir.join("Documents").join("Selamat Datang di BijiOS.docx"), &docx_bytes);
            }
        }
    }

    pub fn list(query: &str, ctx: &AuthContext, db: &DbHandle, db_dir: &std::path::Path) -> (u16, String) {
        let mut parent_filter: Option<String> = None;
        let mut ext_filter: Option<String> = None;
        let mut search_filter: Option<String> = None;
        let mut user_filter: Option<String> = None;
        let mut tenant_filter: Option<String> = None;
        let mut category_filter: Option<String> = None;

        for param in query.split('&') {
            if let Some((k, v)) = param.split_once('=') {
                match k {
                    "parent_id" => parent_filter = Some(v.to_string()),
                    "extension" => ext_filter = Some(v.to_lowercase()),
                    "q" => search_filter = Some(v.to_lowercase()),
                    "user_id" => if !v.is_empty() { user_filter = Some(v.to_string()); },
                    "tenant_id" => if !v.is_empty() { tenant_filter = Some(v.to_string()); },
                    "category" => if !v.is_empty() { category_filter = Some(v.to_string()); },
                    _ => {}
                }
            }
        }

        let target_tenant = if ctx.role == Role::SuperAdmin {
            tenant_filter.unwrap_or_else(|| ctx.tenant_id.clone())
        } else {
            ctx.tenant_id.clone()
        };

        let target_user = if ctx.role == Role::SuperAdmin {
            user_filter.unwrap_or_else(|| ctx.user_id.clone())
        } else {
            ctx.user_id.clone()
        };

        let category = category_filter.unwrap_or_else(|| "all".to_string());

        // Auto-provision personal workspace storage if user has no files yet
        if !target_user.is_empty() && target_user != "all" && !target_tenant.is_empty() {
            Self::ensure_user_storage(&target_user, &target_tenant, db, db_dir);
        }

        let db_lock = db.read().unwrap();
        let items: Vec<String> = db_lock
            .files
            .iter()
            .filter(|f| {
                if ctx.role != Role::SuperAdmin && f.tenant_id != target_tenant {
                    return false;
                }
                if ctx.role == Role::SuperAdmin && !target_tenant.is_empty() && f.tenant_id != target_tenant {
                    return false;
                }

                // Category-based filtering for complete workspace isolation
                if category == "shared" {
                    let is_shared_to_me = db_lock.shares.iter().any(|s| {
                        s.file_id == f.id
                            && s.share_type == "internal"
                            && (s.target_user_id == ctx.user_id || s.target_user_id.is_empty())
                    });
                    if !is_shared_to_me {
                        return false;
                    }
                } else if category == "public" {
                    if f.owner_id != target_user {
                        return false;
                    }
                    let is_public = db_lock.shares.iter().any(|s| s.file_id == f.id && s.share_type == "public");
                    if !is_public {
                        return false;
                    }
                } else {
                    // "all" or general folder browsing: strictly personal isolated files!
                    if target_user != "all" && f.owner_id != target_user {
                        return false;
                    }
                }

                if let Some(ref p) = parent_filter {
                    if p != "all" && &f.parent_id != p {
                        return false;
                    }
                }
                if let Some(ref ext) = ext_filter {
                    if &f.extension.to_lowercase() != ext {
                        return false;
                    }
                }
                if let Some(ref q) = search_filter {
                    if !f.name.to_lowercase().contains(q) {
                        return false;
                    }
                }
                true
            })
            .map(|f| {
                // Check if file is shared
                let is_shared_public = db_lock.shares.iter().any(|s| s.file_id == f.id && s.share_type == "public");
                let is_shared_internal = db_lock.shares.iter().any(|s| s.file_id == f.id && s.share_type == "internal");

                format!(
                    r#"{{
                        "id": "{}",
                        "tenant_id": "{}",
                        "parent_id": "{}",
                        "name": "{}",
                        "extension": "{}",
                        "size_bytes": {},
                        "mime_type": "{}",
                        "owner_id": "{}",
                        "is_folder": {},
                        "is_shared_public": {},
                        "is_shared_internal": {},
                        "created_at": "{}",
                        "updated_at": "{}"
                    }}"#,
                    escape_json(&f.id),
                    escape_json(&f.tenant_id),
                    escape_json(&f.parent_id),
                    escape_json(&f.name),
                    escape_json(&f.extension),
                    f.size_bytes,
                    escape_json(&f.mime_type),
                    escape_json(&f.owner_id),
                    f.is_folder,
                    is_shared_public,
                    is_shared_internal,
                    escape_json(&f.created_at),
                    escape_json(&f.updated_at)
                )
            })
            .collect();

        (200, format!("[{}]", items.join(",")))
    }

    pub fn get_metadata(file_id: &str, ctx: &AuthContext, db: &DbHandle) -> (u16, String) {
        let db_lock = db.read().unwrap();
        let file = db_lock.files.iter().find(|f| f.id == file_id);

        if let Some(f) = file {
            if ctx.role != Role::SuperAdmin && f.tenant_id != ctx.tenant_id {
                return (403, r#"{"error":"Akses file ditolak untuk tenant ini"}"#.to_string());
            }

            let shares: Vec<String> = db_lock
                .shares
                .iter()
                .filter(|s| s.file_id == f.id)
                .map(|s| {
                    format!(
                        r#"{{"id":"{}","type":"{}","token":"{}","permission":"{}","downloads":{}}}"#,
                        escape_json(&s.id),
                        escape_json(&s.share_type),
                        escape_json(&s.token),
                        escape_json(&s.permission),
                        s.download_count
                    )
                })
                .collect();

            let res = format!(
                r#"{{
                    "id": "{}",
                    "name": "{}",
                    "extension": "{}",
                    "size_bytes": {},
                    "mime_type": "{}",
                    "owner_id": "{}",
                    "is_folder": {},
                    "shares": [{}],
                    "created_at": "{}",
                    "updated_at": "{}"
                }}"#,
                escape_json(&f.id),
                escape_json(&f.name),
                escape_json(&f.extension),
                f.size_bytes,
                escape_json(&f.mime_type),
                escape_json(&f.owner_id),
                f.is_folder,
                shares.join(","),
                escape_json(&f.created_at),
                escape_json(&f.updated_at)
            );
            (200, res)
        } else {
            (404, r#"{"error":"File tidak ditemukan"}"#.to_string())
        }
    }

    pub fn get_content(file_id: &str, ctx: &AuthContext, db: &DbHandle, db_dir: &std::path::Path) -> (u16, String) {
        let db_lock = db.read().unwrap();
        let file = db_lock.files.iter().find(|f| f.id == file_id);

        if let Some(f) = file {
            if ctx.role != Role::SuperAdmin && f.tenant_id != ctx.tenant_id {
                // Check if user has internal share access
                let has_share = db_lock.shares.iter().any(|s| s.file_id == f.id && (s.target_user_id == ctx.user_id || s.share_type == "public"));
                if !has_share {
                    return (403, r#"{"error":"Akses konten file ditolak"}"#.to_string());
                }
            }

            if !f.content.is_empty() {
                if f.content.as_bytes().starts_with(b"PK\x03\x04") {
                    if f.extension == "xlsx" || f.name.to_lowercase().ends_with(".xlsx") {
                        if let Some(json) = crate::xlsx::XlsxEngine::extract_to_json(f.content.as_bytes()) {
                            return (200, json);
                        }
                    }
                    if f.extension == "pptx" || f.name.to_lowercase().ends_with(".pptx") {
                        if let Some(json) = crate::pptx::PptxEngine::extract_to_json(f.content.as_bytes()) {
                            return (200, json);
                        }
                    }
                    if let Some(html) = crate::docx::DocxEngine::extract_html_from_docx(f.content.as_bytes()) {
                        return (200, html);
                    }
                    if let Some(json) = crate::xlsx::XlsxEngine::extract_to_json(f.content.as_bytes()) {
                        return (200, json);
                    }
                }
                return (200, f.content.clone());
            }

            // Fallback: read physical file from disk
            let storage_base = db_dir.parent().unwrap_or(db_dir).join("storage");
            let user_dir = storage_base.join(&f.tenant_id).join(&f.owner_id);
            let parent_folder_name = if f.parent_id != "root" {
                db_lock.files.iter().find(|x| x.id == f.parent_id).map(|x| x.name.clone())
            } else {
                None
            };
            let target_dir = if let Some(ref pf) = parent_folder_name {
                user_dir.join(pf)
            } else {
                user_dir
            };
            let direct = target_dir.join(&f.name);
            if let Ok(bytes) = std::fs::read(&direct) {
                if bytes.starts_with(b"PK\x03\x04") {
                    if f.extension == "xlsx" || f.name.to_lowercase().ends_with(".xlsx") {
                        if let Some(json) = crate::xlsx::XlsxEngine::extract_to_json(&bytes) {
                            return (200, json);
                        }
                    }
                    if f.extension == "pptx" || f.name.to_lowercase().ends_with(".pptx") {
                        if let Some(json) = crate::pptx::PptxEngine::extract_to_json(&bytes) {
                            return (200, json);
                        }
                    }
                    if let Some(html) = crate::docx::DocxEngine::extract_html_from_docx(&bytes) {
                        return (200, html);
                    }
                    if let Some(json) = crate::xlsx::XlsxEngine::extract_to_json(&bytes) {
                        return (200, json);
                    }
                }
                if f.mime_type.starts_with("image/") {
                    let b64 = Self::encode_base64(&bytes);
                    let data_url = format!("data:{};base64,{}", f.mime_type, b64);
                    return (200, data_url);
                }
                if let Ok(text) = String::from_utf8(bytes) {
                    return (200, text);
                }
            }

            (200, String::new())
        } else {
            (404, r#"{"error":"File tidak ditemukan"}"#.to_string())
        }
    }

    pub fn update_content(file_id: &str, body: &str, ctx: &AuthContext, db: &DbHandle, db_dir: &std::path::Path) -> (u16, String) {
        if !ctx.role.can_write_files() {
            return (403, r#"{"error":"Role Guest tidak memiliki izin mengedit file"}"#.to_string());
        }

        // Extract content from body
        let content_to_save: String = if let Some(j) = parse_json(body) {
            if let Some(c) = j.get_str("content") {
                c.to_string()
            } else if let Some(h) = j.get_str("html") {
                h.to_string()
            } else {
                body.to_string()
            }
        } else {
            body.to_string()
        };

        let now = Self::now_str();
        let mut db_lock = db.write().unwrap();

        let pos = db_lock.files.iter().position(|f| f.id == file_id);
        if let Some(idx) = pos {
            if ctx.role != Role::SuperAdmin && db_lock.files[idx].tenant_id != ctx.tenant_id {
                return (403, r#"{"error":"Izin edit ditolak"}"#.to_string());
            }

            let file_name = db_lock.files[idx].name.clone();
            let file_tenant = db_lock.files[idx].tenant_id.clone();
            let file_owner = db_lock.files[idx].owner_id.clone();
            let is_folder = db_lock.files[idx].is_folder;
            let parent_id = db_lock.files[idx].parent_id.clone();
            let parent_folder_name = if parent_id != "root" {
                db_lock.files.iter().find(|x| x.id == parent_id).map(|x| x.name.clone())
            } else {
                None
            };

            db_lock.files[idx].content = content_to_save.clone();
            db_lock.files[idx].updated_at = now;

            // Write physical file to storage on disk
            if !is_folder {
                let storage_base = db_dir.parent().unwrap_or(db_dir).join("storage");
                let user_dir = storage_base.join(&file_tenant).join(&file_owner);
                let target_dir = if let Some(ref pf) = parent_folder_name {
                    user_dir.join(pf)
                } else {
                    user_dir
                };
                let _ = std::fs::create_dir_all(&target_dir);
                let target_file = target_dir.join(&file_name);
                
                if file_name.to_lowercase().ends_with(".docx") {
                    let docx_bytes = crate::docx::DocxEngine::create_docx_package(&content_to_save);
                    db_lock.files[idx].size_bytes = docx_bytes.len();
                    let _ = std::fs::write(&target_file, &docx_bytes);
                } else if file_name.to_lowercase().ends_with(".xlsx") {
                    let xlsx_bytes = crate::xlsx::XlsxEngine::create_xlsx_package(&content_to_save);
                    db_lock.files[idx].size_bytes = xlsx_bytes.len();
                    let _ = std::fs::write(&target_file, &xlsx_bytes);
                } else if file_name.to_lowercase().ends_with(".pptx") {
                    let pptx_bytes = crate::pptx::PptxEngine::create_pptx_package(&content_to_save);
                    db_lock.files[idx].size_bytes = pptx_bytes.len();
                    let _ = std::fs::write(&target_file, &pptx_bytes);
                } else if content_to_save.starts_with("data:") {
                    if let Some(comma_pos) = content_to_save.find(',') {
                        let b64 = &content_to_save[comma_pos + 1..];
                        if let Some(bin) = Self::decode_base64(b64) {
                            db_lock.files[idx].size_bytes = bin.len();
                            let _ = std::fs::write(&target_file, &bin);
                        } else {
                            db_lock.files[idx].size_bytes = content_to_save.len();
                            let _ = std::fs::write(&target_file, content_to_save.as_bytes());
                        }
                    } else {
                        db_lock.files[idx].size_bytes = content_to_save.len();
                        let _ = std::fs::write(&target_file, content_to_save.as_bytes());
                    }
                } else {
                    db_lock.files[idx].size_bytes = content_to_save.len();
                    let _ = std::fs::write(&target_file, content_to_save.as_bytes());
                }
            } else {
                db_lock.files[idx].size_bytes = 0;
            }

            db_lock.save_files();
            (200, r#"{"success":true}"#.to_string())
        } else {
            (404, r#"{"error":"File tidak ditemukan"}"#.to_string())
        }
    }

pub fn decode_base64(s: &str) -> Option<Vec<u8>> {
    let b64_table = |b: u8| -> Option<u8> {
        match b {
            b'A'..=b'Z' => Some(b - b'A'),
            b'a'..=b'z' => Some(b - b'a' + 26),
            b'0'..=b'9' => Some(b - b'0' + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    };

    let clean: Vec<u8> = s.bytes().filter(|b| !b.is_ascii_whitespace()).collect();
    let mut out = Vec::with_capacity(clean.len() * 3 / 4);
    let chunks = clean.chunks_exact(4);
    let remainder = chunks.remainder();
    for chunk in chunks {
        let b0 = b64_table(chunk[0])?;
        let b1 = b64_table(chunk[1])?;
        out.push((b0 << 2) | (b1 >> 4));
        if chunk[2] != b'=' {
            let b2 = b64_table(chunk[2])?;
            out.push((b1 << 4) | (b2 >> 2));
            if chunk[3] != b'=' {
                let b3 = b64_table(chunk[3])?;
                out.push((b2 << 6) | b3);
            }
        }
    }
    if !remainder.is_empty() && remainder.len() >= 2 {
        let b0 = b64_table(remainder[0])?;
        let b1 = b64_table(remainder[1])?;
        out.push((b0 << 2) | (b1 >> 4));
        if remainder.len() == 3 {
            let b2 = b64_table(remainder[2])?;
            out.push((b1 << 4) | (b2 >> 2));
        }
    }
    Some(out)
}

pub fn encode_base64(bytes: &[u8]) -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity((bytes.len() + 2) / 3 * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0];
        let b1 = if chunk.len() > 1 { chunk[1] } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] } else { 0 };

        out.push(CHARSET[(b0 >> 2) as usize] as char);
        out.push(CHARSET[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize] as char);
        if chunk.len() > 1 {
            out.push(CHARSET[(((b1 & 0x0f) << 2) | (b2 >> 6)) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(CHARSET[(b2 & 0x3f) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

    pub fn create_or_save(body: &str, ctx: &AuthContext, db: &DbHandle, db_dir: &std::path::Path) -> (u16, String) {
        if !ctx.role.can_write_files() {
            return (403, r#"{"error":"Role Guest tidak memiliki izin membuat file"}"#.to_string());
        }

        let json = match parse_json(body) {
            Some(j) => j,
            None => return (400, r#"{"error":"JSON request tidak valid"}"#.to_string()),
        };

        let file_id = json.get_str("id");
        let name = json.get_str("name").unwrap_or("Dokumen Baru").trim();
        let parent_id = json.get_str("parent_id").unwrap_or("root").trim();
        let extension = json.get_str("extension").unwrap_or("docx").trim().to_lowercase();
        let is_folder = json.get_bool("is_folder").unwrap_or(false);
        let content = json.get_str("content").unwrap_or("");

        let now = Self::now_str();
        let mut db_lock = db.write().unwrap();

        if let Some(id) = file_id {
            // Update existing (Rename or Save Content)
            let pos = db_lock.files.iter().position(|f| f.id == id);
            if let Some(idx) = pos {
                if ctx.role != Role::SuperAdmin && db_lock.files[idx].tenant_id != ctx.tenant_id {
                    return (403, r#"{"error":"Izin edit ditolak"}"#.to_string());
                }
                let old_name = db_lock.files[idx].name.clone();
                let parent_id_val = db_lock.files[idx].parent_id.clone();
                let parent_folder_name = if parent_id_val != "root" {
                    db_lock.files.iter().find(|x| x.id == parent_id_val).map(|x| x.name.clone())
                } else {
                    None
                };

                let f = &mut db_lock.files[idx];
                if !name.is_empty() {
                    f.name = name.to_string();
                    if !f.is_folder {
                        if let Some(dot_idx) = name.rfind('.') {
                            let new_ext = &name[dot_idx + 1..];
                            if !new_ext.is_empty() {
                                f.extension = new_ext.to_lowercase();
                                f.mime_type = match f.extension.as_str() {
                                    "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document".to_string(),
                                    "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet".to_string(),
                                    "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation".to_string(),
                                    "pdf" => "application/pdf".to_string(),
                                    "jpg" | "jpeg" => "image/jpeg".to_string(),
                                    "png" => "image/png".to_string(),
                                    "bmp" => "image/bmp".to_string(),
                                    "webp" => "image/webp".to_string(),
                                    "gif" => "image/gif".to_string(),
                                    "svg" => "image/svg+xml".to_string(),
                                    "txt" | "md" => "text/plain".to_string(),
                                    _ => "application/octet-stream".to_string(),
                                };
                            }
                        }
                    }
                }
                if !content.is_empty() {
                    f.content = content.to_string();
                    let storage_base = db_dir.parent().unwrap_or(db_dir).join("storage");
                    let user_dir = storage_base.join(&f.tenant_id).join(&f.owner_id);
                    let target_dir = if let Some(ref pf) = parent_folder_name {
                        user_dir.join(pf)
                    } else {
                        user_dir
                    };
                    let file_disk_path = target_dir.join(&f.name);
                    let _ = std::fs::create_dir_all(&target_dir);

                    if f.extension == "xlsx" || f.name.to_lowercase().ends_with(".xlsx") {
                        let xlsx_bytes = crate::xlsx::XlsxEngine::create_xlsx_package(content);
                        f.size_bytes = xlsx_bytes.len();
                        let _ = std::fs::write(&file_disk_path, &xlsx_bytes);
                    } else if f.extension == "pptx" || f.name.to_lowercase().ends_with(".pptx") {
                        let pptx_bytes = crate::pptx::PptxEngine::create_pptx_package(content);
                        f.size_bytes = pptx_bytes.len();
                        let _ = std::fs::write(&file_disk_path, &pptx_bytes);
                    } else if f.extension == "docx" || f.name.to_lowercase().ends_with(".docx") {
                        let docx_bytes = crate::docx::DocxEngine::create_docx_package(content);
                        f.size_bytes = docx_bytes.len();
                        let _ = std::fs::write(&file_disk_path, &docx_bytes);
                    } else {
                        f.size_bytes = content.len();
                        let _ = std::fs::write(&file_disk_path, content.as_bytes());
                    }
                }
                f.updated_at = now;

                // Rename physical file or directory on disk if it exists
                let storage_base = db_dir.parent().unwrap_or(db_dir).join("storage");
                let user_dir = storage_base.join(&f.tenant_id).join(&f.owner_id);
                if f.is_folder {
                    let old_dir = user_dir.join(&old_name);
                    let new_dir = user_dir.join(name);
                    if old_dir.exists() {
                        let _ = std::fs::rename(old_dir, new_dir);
                    }
                } else {
                    let direct_old = user_dir.join(&old_name);
                    if direct_old.exists() {
                        let _ = std::fs::rename(direct_old, user_dir.join(name));
                    } else if let Ok(entries) = std::fs::read_dir(&user_dir) {
                        for entry in entries.flatten() {
                            let path = entry.path();
                            if path.is_dir() {
                                let sub_old = path.join(&old_name);
                                if sub_old.exists() {
                                    let _ = std::fs::rename(sub_old, path.join(name));
                                }
                            }
                        }
                    }
                }

                let updated_name = f.name.clone();
                db_lock.save_files();
                return (200, format!(r#"{{"success":true,"id":"{}","name":"{}"}}"#, escape_json(id), escape_json(&updated_name)));
            }
        }

        // Resolve effective extension
        let mut final_ext = extension.clone();
        if final_ext.is_empty() || final_ext == "docx" {
            if let Some(dot_idx) = name.rfind('.') {
                let candidate = name[dot_idx + 1..].trim().to_lowercase();
                if !candidate.is_empty() {
                    final_ext = candidate;
                }
            }
        }

        // Create new
        let new_id = format!("file_{}_{}", final_ext, db_lock.files.len() + 1);
        let mime_type = match final_ext.as_str() {
            "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
            "pdf" => "application/pdf",
            "jpg" | "jpeg" => "image/jpeg",
            "png" => "image/png",
            "bmp" => "image/bmp",
            "webp" => "image/webp",
            "gif" => "image/gif",
            "svg" => "image/svg+xml",
            "txt" | "md" => "text/plain",
            "folder" => "inode/directory",
            _ => "application/octet-stream",
        };

        let default_content = if !content.is_empty() {
            content.to_string()
        } else {
            match final_ext.as_str() {
                "docx" => format!(r##"{{"title":"{}","content":"<p>Ketik dokumen WordCraft Anda di sini...</p>"}}"##, name),
                "xlsx" => format!(r##"{{"title":"{}","sheets":[{{"name":"Sheet 1","cells":{{"A1":{{"value":"Data A"}},"B1":{{"value":"Data B"}}}}}}]}}"##, name),
                "pptx" => format!(r##"{{"title":"{}","slides":[{{"id":"s1","background":"#ffffff","elements":[{{"type":"title","text":"{}","x":80,"y":100,"size":40,"color":"#0f172a"}}]}}]}}"##, name, name),
                _ => String::new(),
            }
        };

        // Write physical file on disk in current folder
        let storage_base = db_dir.parent().unwrap_or(db_dir).join("storage");
        let parent_folder_name = if parent_id != "root" {
            db_lock.files.iter().find(|x| x.id == parent_id).map(|x| x.name.clone())
        } else {
            None
        };
        let target_dir = if let Some(ref pf) = parent_folder_name {
            storage_base.join(&ctx.tenant_id).join(&ctx.user_id).join(pf)
        } else {
            storage_base.join(&ctx.tenant_id).join(&ctx.user_id)
        };
        let _ = std::fs::create_dir_all(&target_dir);
        let mut final_size = default_content.len();
        if !is_folder {
            let file_disk_path = target_dir.join(name);
            if final_ext == "docx" || name.to_lowercase().ends_with(".docx") {
                let html_for_docx = if default_content.contains(r#""content":""#) {
                    if let Some(j) = parse_json(&default_content) {
                        j.get_str("content").unwrap_or("<p>Ketik dokumen WordCraft Anda di sini...</p>").to_string()
                    } else {
                        default_content.clone()
                    }
                } else {
                    default_content.clone()
                };
                let docx_bytes = crate::docx::DocxEngine::create_docx_package(&html_for_docx);
                final_size = docx_bytes.len();
                let _ = std::fs::write(&file_disk_path, &docx_bytes);
            } else if final_ext == "xlsx" || name.to_lowercase().ends_with(".xlsx") {
                let xlsx_bytes = crate::xlsx::XlsxEngine::create_xlsx_package(&default_content);
                final_size = xlsx_bytes.len();
                let _ = std::fs::write(&file_disk_path, &xlsx_bytes);
            } else if final_ext == "pptx" || name.to_lowercase().ends_with(".pptx") {
                let pptx_bytes = crate::pptx::PptxEngine::create_pptx_package(&default_content);
                final_size = pptx_bytes.len();
                let _ = std::fs::write(&file_disk_path, &pptx_bytes);
            } else if default_content.starts_with("data:") {
                // If base64 data URI (uploaded image, etc), decode and save binary
                if let Some(comma_pos) = default_content.find(',') {
                    let b64 = &default_content[comma_pos + 1..];
                    if let Some(bin) = Self::decode_base64(b64) {
                        final_size = bin.len();
                        let _ = std::fs::write(&file_disk_path, &bin);
                    } else {
                        let _ = std::fs::write(&file_disk_path, default_content.as_bytes());
                    }
                } else {
                    let _ = std::fs::write(&file_disk_path, default_content.as_bytes());
                }
            } else {
                let _ = std::fs::write(&file_disk_path, default_content.as_bytes());
            }
        } else {
            let _ = std::fs::create_dir_all(target_dir.join(name));
            final_size = 0;
        }

        let new_file = FileItem {
            id: new_id.clone(),
            tenant_id: ctx.tenant_id.clone(),
            parent_id: parent_id.to_string(),
            name: name.to_string(),
            extension: if is_folder { "folder".to_string() } else { final_ext },
            size_bytes: final_size,
            mime_type: mime_type.to_string(),
            owner_id: ctx.user_id.clone(),
            is_folder,
            content: default_content,
            created_at: now.clone(),
            updated_at: now,
        };

        db_lock.files.push(new_file);
        db_lock.save_files();

        (201, format!(r#"{{"success":true,"id":"{}","name":"{}"}}"#, escape_json(&new_id), escape_json(name)))
    }

    pub fn delete(file_id: &str, ctx: &AuthContext, db: &DbHandle, db_dir: &std::path::Path) -> (u16, String) {
        if !ctx.role.can_write_files() {
            return (403, r#"{"error":"Tidak ada izin menghapus file"}"#.to_string());
        }

        let mut db_lock = db.write().unwrap();
        let pos = db_lock.files.iter().position(|f| f.id == file_id);

        if let Some(idx) = pos {
            if ctx.role != Role::SuperAdmin && db_lock.files[idx].tenant_id != ctx.tenant_id {
                return (403, r#"{"error":"Akses hapus ditolak"}"#.to_string());
            }

            let file_to_delete = db_lock.files.remove(idx);
            // Also remove associated shares
            db_lock.shares.retain(|s| s.file_id != file_id);

            // Delete physical file / directory on disk
            let storage_base = db_dir.parent().unwrap_or(db_dir).join("storage");
            let user_dir = storage_base.join(&file_to_delete.tenant_id).join(&file_to_delete.owner_id);
            if file_to_delete.is_folder {
                let folder_path = user_dir.join(&file_to_delete.name);
                let _ = std::fs::remove_dir_all(folder_path);
            } else {
                let direct_file = user_dir.join(&file_to_delete.name);
                if direct_file.exists() {
                    let _ = std::fs::remove_file(direct_file);
                } else if let Ok(entries) = std::fs::read_dir(&user_dir) {
                    for entry in entries.flatten() {
                        let p = entry.path();
                        if p.is_dir() {
                            let sub_file = p.join(&file_to_delete.name);
                            if sub_file.exists() {
                                let _ = std::fs::remove_file(sub_file);
                            }
                        }
                    }
                }
            }

            db_lock.save_files();
            db_lock.save_shares();

            (200, r#"{"success":true}"#.to_string())
        } else {
            (404, r#"{"error":"File tidak ditemukan"}"#.to_string())
        }
    }

    pub fn get_storage_quota(query: &str, ctx: &AuthContext, db: &DbHandle, db_dir: &std::path::Path) -> (u16, String) {
        let mut req_user_id = ctx.user_id.clone();
        let mut req_tenant_id = ctx.tenant_id.clone();

        for param in query.split('&') {
            if let Some((k, v)) = param.split_once('=') {
                if k == "user_id" && !v.is_empty() {
                    req_user_id = v.to_string();
                } else if k == "tenant_id" && !v.is_empty() {
                    req_tenant_id = v.to_string();
                }
            }
        }

        let quota_bytes: u64 = {
            let db_lock = db.read().unwrap();
            db_lock
                .users
                .iter()
                .find(|u| u.id == req_user_id)
                .map(|u| u.quota_bytes)
                .unwrap_or(21474836480) // 20 GB default
        };
        let storage_base = db_dir.parent().unwrap_or(db_dir).join("storage");
        let user_storage_dir = storage_base.join(&req_tenant_id).join(&req_user_id);

        fn calc_dir_size(p: &std::path::Path) -> u64 {
            let mut total = 0;
            if let Ok(entries) = std::fs::read_dir(p) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        total += path.metadata().map(|m| m.len()).unwrap_or(0);
                    } else if path.is_dir() {
                        total += calc_dir_size(&path);
                    }
                }
            }
            total
        }

        let used_bytes = if user_storage_dir.exists() {
            calc_dir_size(&user_storage_dir)
        } else {
            0
        };

        let used_mb = (used_bytes as f64) / (1024.0 * 1024.0);
        let quota_mb = (quota_bytes as f64) / (1024.0 * 1024.0);
        let free_mb = if quota_bytes > used_bytes { (quota_bytes - used_bytes) as f64 / (1024.0 * 1024.0) } else { 0.0 };
        let percent = if quota_bytes > 0 { ((used_bytes as f64) / (quota_bytes as f64)) * 100.0 } else { 0.0 };

        (200, format!(
            r#"{{"user_id":"{}","tenant_id":"{}","used_bytes":{},"quota_bytes":{},"used_mb":{:.2},"quota_mb":{:.1},"free_mb":{:.2},"percent":{:.1}}}"#,
            req_user_id, req_tenant_id, used_bytes, quota_bytes, used_mb, quota_mb, free_mb, percent
        ))
    }
}

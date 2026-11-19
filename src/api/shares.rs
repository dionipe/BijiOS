use crate::api::json_helper::{escape_json, parse_json, JsonValue};
use crate::auth::{AuthContext, Role};
use crate::db::{DbHandle, ShareItem};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct SharesApi;

impl SharesApi {
    fn now_secs() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    pub fn list(ctx: &AuthContext, db: &DbHandle) -> (u16, String) {
        let db_lock = db.read().unwrap();
        let cur_dept = db_lock
            .users
            .iter()
            .find(|u| u.id == ctx.user_id)
            .map(|u| u.department.clone())
            .unwrap_or_default();

        let items: Vec<String> = db_lock
            .shares
            .iter()
            .filter(|s| {
                if ctx.role == Role::SuperAdmin {
                    return true;
                }
                if s.tenant_id != ctx.tenant_id {
                    return false;
                }
                if ctx.role == Role::Admin {
                    return true;
                }
                if ctx.role == Role::Supervisor {
                    // Supervisor sees shares they requested, shares assigned to them, or shares from their department
                    let req_dept = db_lock
                        .users
                        .iter()
                        .find(|u| u.id == s.requested_by)
                        .map(|u| u.department.as_str())
                        .unwrap_or("");
                    return s.requested_by == ctx.user_id
                        || s.approver_id == ctx.user_id
                        || (!cur_dept.is_empty() && req_dept.eq_ignore_ascii_case(&cur_dept))
                        || s.target_user_id == ctx.user_id;
                }
                // Staff sees their own share requests or approved shares targeted to them
                s.requested_by == ctx.user_id
                    || (s.target_user_id == ctx.user_id && s.approval_status == "approved")
            })
            .map(|s| {
                let file_name = db_lock
                    .files
                    .iter()
                    .find(|f| f.id == s.file_id)
                    .map(|f| f.name.as_str())
                    .unwrap_or("Unknown File");

                let target_user_name = if !s.target_email.is_empty() {
                    s.target_email.as_str()
                } else {
                    db_lock
                        .users
                        .iter()
                        .find(|u| u.id == s.target_user_id)
                        .map(|u| u.name.as_str())
                        .unwrap_or(if s.share_type == "public" {
                            "Publik / Siapapun dengan Link"
                        } else {
                            "Anggota Internal"
                        })
                };

                let requester = db_lock.users.iter().find(|u| u.id == s.requested_by);
                let requester_name = requester.map(|u| u.name.as_str()).unwrap_or(&s.requested_by);
                let requester_dept = requester.map(|u| u.department.as_str()).unwrap_or("");

                let approver_name = db_lock
                    .users
                    .iter()
                    .find(|u| u.id == s.approver_id)
                    .map(|u| u.name.as_str())
                    .unwrap_or(if s.share_type == "public" { "Otomatis (Publik)" } else { "-" });

                let can_approve = s.approval_status == "pending"
                    && ctx.role.can_approve_shares()
                    && (ctx.role == Role::SuperAdmin
                        || ctx.role == Role::Admin
                        || s.approver_id == ctx.user_id
                        || (!cur_dept.is_empty() && requester_dept.eq_ignore_ascii_case(&cur_dept)));

                format!(
                    r#"{{
                        "id": "{}",
                        "file_id": "{}",
                        "file_name": "{}",
                        "share_type": "{}",
                        "target_user_id": "{}",
                        "target_user_name": "{}",
                        "target_email": "{}",
                        "token": "{}",
                        "permission": "{}",
                        "has_password": {},
                        "download_count": {},
                        "requested_by": "{}",
                        "requester_name": "{}",
                        "requester_department": "{}",
                        "approver_id": "{}",
                        "approver_name": "{}",
                        "approval_status": "{}",
                        "request_note": "{}",
                        "approval_note": "{}",
                        "can_approve": {}
                    }}"#,
                    escape_json(&s.id),
                    escape_json(&s.file_id),
                    escape_json(file_name),
                    escape_json(&s.share_type),
                    escape_json(&s.target_user_id),
                    escape_json(target_user_name),
                    escape_json(&s.target_email),
                    escape_json(&s.token),
                    escape_json(&s.permission),
                    s.password.is_some(),
                    s.download_count,
                    escape_json(&s.requested_by),
                    escape_json(requester_name),
                    escape_json(requester_dept),
                    escape_json(&s.approver_id),
                    escape_json(approver_name),
                    escape_json(&s.approval_status),
                    escape_json(&s.request_note),
                    escape_json(&s.approval_note),
                    can_approve
                )
            })
            .collect();

        (200, format!("[{}]", items.join(",")))
    }

    pub fn create(body: &str, ctx: &AuthContext, db: &DbHandle) -> (u16, String) {
        if !ctx.role.can_create_public_shares() {
            return (403, r#"{"error":"Role Anda tidak diizinkan membuat tautan berbagi"}"#.to_string());
        }

        let json = match parse_json(body) {
            Some(j) => j,
            None => return (400, r#"{"error":"JSON tidak valid"}"#.to_string()),
        };

        let file_id = match json.get_str("file_id") {
            Some(id) => id.trim(),
            None => return (400, r#"{"error":"file_id wajib disertakan"}"#.to_string()),
        };

        let share_type = json.get_str("share_type").unwrap_or("public").trim().to_lowercase();
        let target_user_id = json.get_str("target_user_id").unwrap_or("").trim().to_string();
        let target_email = json.get_str("target_email").unwrap_or("").trim().to_string();
        let raw_perm = json.get_str("permission").unwrap_or("view").trim().to_lowercase();
        let permission = match raw_perm.as_str() {
            "edit" | "editor" => "edit",
            "comment" | "commenter" => "comment",
            _ => "view",
        };
        let password = json.get_str("password").map(|p| p.trim().to_string()).filter(|p| !p.is_empty());
        let expires_at = json.get_str("expires_at").map(|e| e.trim().to_string()).filter(|e| !e.is_empty());
        let request_note = json.get_str("request_note").unwrap_or("").trim().to_string();

        let mut db_lock = db.write().unwrap();
        if !db_lock.files.iter().any(|f| f.id == file_id && (ctx.role == Role::SuperAdmin || f.tenant_id == ctx.tenant_id)) {
            return (404, r#"{"error":"File tidak ditemukan di workspace ini"}"#.to_string());
        }

        // Determine Approval Flow:
        // - Public share: NO approval flow needed (immediately approved).
        // - Internal / External share requested by Staff (Role::Member): requires Supervisor approval -> "pending".
        // - Internal / External share requested by Supervisor / Admin / SuperAdmin: auto-approved.
        let needs_approval = share_type != "public" && ctx.role == Role::Member;
        let (approval_status, approver_id) = if !needs_approval {
            ("approved".to_string(), ctx.user_id.clone())
        } else {
            // Resolve staff's direct supervisor -> department supervisor -> tenant admin
            let requester = db_lock.users.iter().find(|u| u.id == ctx.user_id);
            let direct_sup = requester.map(|u| u.supervisor_id.clone()).unwrap_or_default();
            let req_dept = requester.map(|u| u.department.clone()).unwrap_or_default();

            let resolved_approver = if !direct_sup.is_empty() && db_lock.users.iter().any(|u| u.id == direct_sup) {
                direct_sup
            } else if !req_dept.is_empty() {
                db_lock
                    .users
                    .iter()
                    .find(|u| u.tenant_id == ctx.tenant_id && u.role == Role::Supervisor && u.department.eq_ignore_ascii_case(&req_dept))
                    .or_else(|| db_lock.users.iter().find(|u| u.tenant_id == ctx.tenant_id && u.role == Role::Admin))
                    .map(|u| u.id.clone())
                    .unwrap_or_default()
            } else {
                db_lock
                    .users
                    .iter()
                    .find(|u| u.tenant_id == ctx.tenant_id && matches!(u.role, Role::Supervisor | Role::Admin))
                    .map(|u| u.id.clone())
                    .unwrap_or_default()
            };

            ("pending".to_string(), resolved_approver)
        };

        let prefix = match share_type.as_str() {
            "public" => "pub",
            "external" => "ext",
            _ => "int",
        };
        let random_token = format!(
            "dkh-share-{}-{:x}",
            prefix,
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() % 0xFFFFFF
        );

        let new_id = format!("shr_{}_{}", Self::now_secs() % 100000, db_lock.shares.len() + 1);
        let new_share = ShareItem {
            id: new_id.clone(),
            tenant_id: ctx.tenant_id.clone(),
            file_id: file_id.to_string(),
            share_type: share_type.clone(),
            target_user_id,
            token: random_token.clone(),
            permission: permission.to_string(),
            password,
            expires_at,
            download_count: 0,
            requested_by: ctx.user_id.clone(),
            approver_id: approver_id.clone(),
            approval_status: approval_status.clone(),
            request_note,
            approval_note: String::new(),
            target_email,
        };

        db_lock.shares.push(new_share);
        db_lock.save_shares();

        (
            201,
            format!(
                r#"{{"success":true,"id":"{}","token":"{}","share_url":"/share/{}","approval_status":"{}","needs_approval":{},"permission":"{}"}}"#,
                escape_json(&new_id),
                escape_json(&random_token),
                escape_json(&random_token),
                escape_json(&approval_status),
                needs_approval,
                escape_json(permission)
            ),
        )
    }

    pub fn review_approval(
        share_id: &str,
        approve: bool,
        body: &str,
        ctx: &AuthContext,
        db: &DbHandle,
    ) -> (u16, String) {
        if !ctx.role.can_approve_shares() {
            return (403, r#"{"error":"Hanya Supervisor, Admin Tenant, atau SuperAdmin yang dapat melakukan approval"}"#.to_string());
        }

        let note = parse_json(body)
            .and_then(|j| j.get_str("approval_note").or_else(|| j.get_str("note")).map(|s| s.trim().to_string()))
            .unwrap_or_default();

        let mut db_lock = db.write().unwrap();
        let idx = match db_lock.shares.iter().position(|s| s.id == share_id) {
            Some(i) => i,
            None => return (404, r#"{"error":"Pengajuan share tidak ditemukan"}"#.to_string()),
        };

        let share_tenant = db_lock.shares[idx].tenant_id.clone();
        if ctx.role != Role::SuperAdmin && share_tenant != ctx.tenant_id {
            return (403, r#"{"error":"Tidak diizinkan menyetujui share lintas tenant"}"#.to_string());
        }

        if ctx.role == Role::Supervisor {
            let cur_dept = db_lock
                .users
                .iter()
                .find(|u| u.id == ctx.user_id)
                .map(|u| u.department.clone())
                .unwrap_or_default();
            let req_id = db_lock.shares[idx].requested_by.clone();
            let req_dept = db_lock
                .users
                .iter()
                .find(|u| u.id == req_id)
                .map(|u| u.department.clone())
                .unwrap_or_default();
            let assigned = db_lock.shares[idx].approver_id == ctx.user_id;
            let same_dept = !cur_dept.is_empty() && cur_dept.eq_ignore_ascii_case(&req_dept);
            if !assigned && !same_dept {
                return (403, r#"{"error":"Supervisor hanya dapat menyetujui pengajuan dari Staff di bagiannya"}"#.to_string());
            }
        }

        db_lock.shares[idx].approval_status = if approve { "approved".to_string() } else { "rejected".to_string() };
        db_lock.shares[idx].approver_id = ctx.user_id.clone();
        db_lock.shares[idx].approval_note = note;
        let token = db_lock.shares[idx].token.clone();
        let status = db_lock.shares[idx].approval_status.clone();
        db_lock.save_shares();

        (
            200,
            format!(
                r#"{{"success":true,"id":"{}","approval_status":"{}","token":"{}"}}"#,
                escape_json(share_id),
                escape_json(&status),
                escape_json(&token)
            ),
        )
    }

    pub fn get_public(token: &str, password_attempt: Option<&str>, db: &DbHandle) -> (u16, String) {
        let mut db_lock = db.write().unwrap();
        let share_pos = db_lock.shares.iter().position(|s| s.token == token);

        if let Some(idx) = share_pos {
            let share = db_lock.shares[idx].clone();

            // Enforce approval flow for non-public shares
            if share.share_type != "public" && share.approval_status != "approved" {
                let msg = if share.approval_status == "rejected" {
                    "Pengajuan berbagi dokumen ini telah ditolak oleh Supervisor."
                } else {
                    "Pengajuan berbagi dokumen ini masih menunggu persetujuan (Approval) Supervisor."
                };
                return (403, format!(r#"{{"error":"{}","approval_status":"{}"}}"#, msg, escape_json(&share.approval_status)));
            }

            if let Some(ref required_pw) = share.password {
                if let Some(attempt) = password_attempt {
                    if attempt != required_pw {
                        return (401, r#"{"error":"Kata sandi salah","require_password":true}"#.to_string());
                    }
                } else {
                    return (401, r#"{"error":"Dokumen ini dilindungi kata sandi","require_password":true}"#.to_string());
                }
            }

            db_lock.shares[idx].download_count += 1;
            db_lock.save_shares();

            if let Some(file) = db_lock.files.iter().find(|f| f.id == share.file_id) {
                let res = format!(
                    r#"{{
                        "success": true,
                        "share_id": "{}",
                        "share_type": "{}",
                        "permission": "{}",
                        "approval_status": "{}",
                        "file": {{
                            "id": "{}",
                            "name": "{}",
                            "extension": "{}",
                            "size_bytes": {},
                            "mime_type": "{}",
                            "permission": "{}",
                            "updated_at": "{}"
                        }},
                        "content": {}
                    }}"#,
                    escape_json(&share.id),
                    escape_json(&share.share_type),
                    escape_json(&share.permission),
                    escape_json(&share.approval_status),
                    escape_json(&file.id),
                    escape_json(&file.name),
                    escape_json(&file.extension),
                    file.size_bytes,
                    escape_json(&file.mime_type),
                    escape_json(&share.permission),
                    escape_json(&file.updated_at),
                    if file.content.trim().starts_with('{') || file.content.trim().starts_with('[') {
                        file.content.clone()
                    } else {
                        format!("\"{}\"", escape_json(&file.content))
                    }
                );
                return (200, res);
            }
        }

        (404, r#"{"error":"Tautan berbagi tidak valid atau telah dicabut"}"#.to_string())
    }

    pub fn list_comments(share_id: &str, db_dir: &Path) -> (u16, String) {
        let path = db_dir.join("share_comments.jsonl");
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        let mut items = Vec::new();
        for line in content.lines() {
            if let Some(j) = parse_json(line.trim()) {
                if j.get_str("share_id").unwrap_or("") == share_id {
                    items.push(line.trim().to_string());
                }
            }
        }
        (200, format!("[{}]", items.join(",")))
    }

    pub fn add_comment(share_id: &str, body: &str, ctx: &AuthContext, db: &DbHandle, db_dir: &Path) -> (u16, String) {
        let json = parse_json(body).unwrap_or(JsonValue::Null);
        let text = json.get_str("comment").or_else(|| json.get_str("text")).unwrap_or("").trim();
        if text.is_empty() {
            return (400, r#"{"error":"Komentar tidak boleh kosong"}"#.to_string());
        }

        let db_lock = db.read().unwrap();
        let share = match db_lock.shares.iter().find(|s| s.id == share_id || s.token == share_id) {
            Some(s) => s,
            None => return (404, r#"{"error":"Tautan berbagi tidak ditemukan"}"#.to_string()),
        };

        if share.share_type != "public" && share.approval_status != "approved" {
            return (403, r#"{"error":"Share belum disetujui Supervisor"}"#.to_string());
        }
        if share.permission == "view" && !ctx.role.can_approve_shares() {
            return (403, r#"{"error":"Hak akses View Only tidak diizinkan menambahkan catatan/komentar"}"#.to_string());
        }

        let author = json.get_str("author").unwrap_or(&ctx.name).trim();
        let ts = Self::now_secs();
        let record = format!(
            r#"{{"id":"cmt_{}","share_id":"{}","author":"{}","role":"{}","text":"{}","timestamp":{}}}"#,
            ts,
            escape_json(&share.id),
            escape_json(author),
            escape_json(ctx.role.as_str()),
            escape_json(text),
            ts
        );

        let path = db_dir.join("share_comments.jsonl");
        if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&path) {
            let _ = writeln!(f, "{}", record);
        }

        (201, record)
    }

    pub fn delete(id: &str, ctx: &AuthContext, db: &DbHandle) -> (u16, String) {
        let mut db_lock = db.write().unwrap();
        let prev = db_lock.shares.len();
        db_lock.shares.retain(|s| !(s.id == id && (ctx.role == Role::SuperAdmin || s.tenant_id == ctx.tenant_id)));
        if db_lock.shares.len() < prev {
            db_lock.save_shares();
            (200, r#"{"success":true}"#.to_string())
        } else {
            (404, r#"{"error":"Tautan tidak ditemukan"}"#.to_string())
        }
    }
}

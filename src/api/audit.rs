use crate::api::json_helper::{escape_json, parse_json, JsonValue};
use crate::auth::{AuthContext, Role};
use crate::db::DbHandle;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static AUDIT_SEQ: AtomicU64 = AtomicU64::new(1);

pub struct AuditApi;

impl AuditApi {
    fn now_secs() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    fn audit_file(db_dir: &Path) -> PathBuf {
        db_dir.join("audit_trail.jsonl")
    }

    /// Append a structured audit trail event to `audit_trail.jsonl`
    #[allow(clippy::too_many_arguments)]
    pub fn record(
        db_dir: &Path,
        tenant_id: &str,
        user_id: &str,
        user_name: &str,
        role: &str,
        category: &str,
        action: &str,
        method: &str,
        resource: &str,
        detail: &str,
        status_code: u16,
        severity: &str,
        ip_address: &str,
    ) {
        let ts = Self::now_secs();
        let seq = AUDIT_SEQ.fetch_add(1, Ordering::Relaxed);
        let id = format!("aud_{}_{}", ts, seq);

        let line = format!(
            r#"{{"id":"{}","timestamp":{},"tenant_id":"{}","user_id":"{}","user_name":"{}","role":"{}","category":"{}","action":"{}","method":"{}","resource":"{}","detail":"{}","status_code":{},"severity":"{}","ip_address":"{}"}}"#,
            escape_json(&id),
            ts,
            escape_json(tenant_id),
            escape_json(user_id),
            escape_json(user_name),
            escape_json(role),
            escape_json(category),
            escape_json(action),
            escape_json(method),
            escape_json(resource),
            escape_json(detail),
            status_code,
            escape_json(severity),
            escape_json(ip_address),
        );

        let path = Self::audit_file(db_dir);
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&path) {
            let _ = writeln!(file, "{}", line);
        }
    }

    /// Resolve display name of a user from DbHandle
    pub fn resolve_user_name(user_id: &str, db: &DbHandle) -> String {
        if let Ok(lock) = db.read() {
            if let Some(u) = lock.users.iter().find(|u| u.id == user_id) {
                return u.name.clone();
            }
        }
        user_id.to_string()
    }

    /// Automatic classification & recording for API requests routed via ApiRouter
    #[allow(clippy::too_many_arguments)]
    pub fn auto_record_request(
        method: &str,
        sub_path: &str,
        body: &str,
        status_code: u16,
        ctx: &AuthContext,
        db: &DbHandle,
        db_dir: &Path,
        ip_address: &str,
    ) {
        // Skip noisy polling / read-only list endpoints to keep audit trail high-signal
        if sub_path.starts_with("/audit") {
            return;
        }
        if method == "GET" && (sub_path.starts_with("/meet/signals") || sub_path == "/storage/quota" || sub_path == "/settings/system-stats" || sub_path == "/auth/me") {
            return;
        }
        // Only track mutating operations + login + sensitive reads
        if method == "GET" && !sub_path.starts_with("/shares/public/") {
            return;
        }

        // Skip high-frequency WebRTC ICE candidate signals
        if sub_path == "/meet/signal" {
            if let Some(j) = parse_json(body) {
                let sig_type = j.get_str("type").unwrap_or("");
                if sig_type == "candidate" || sig_type == "offer" || sig_type == "answer" {
                    return;
                }
            }
        }

        let mut user_id = ctx.user_id.clone();
        let mut user_name = Self::resolve_user_name(&ctx.user_id, db);
        let role = ctx.role.as_str().to_string();
        let tenant_id = ctx.tenant_id.clone();

        let (category, action, detail, severity) = if sub_path == "/auth/login" {
            let email = parse_json(body)
                .and_then(|j| j.get_str("email").map(|s| s.to_string()))
                .unwrap_or_else(|| "unknown".to_string());
            user_name = email.clone();
            if status_code == 200 {
                ("AUTH", "LOGIN_SUCCESS", format!("Login berhasil untuk akun {}", email), "info")
            } else {
                user_id = "anonymous".to_string();
                ("AUTH", "LOGIN_FAILED", format!("Percobaan login gagal untuk akun {}", email), "warning")
            }
        } else if sub_path.starts_with("/users") {
            if sub_path.ends_with("/password") {
                ("AUTH", "PASSWORD_CHANGE", format!("Perubahan kata sandi pada {}", sub_path), "warning")
            } else if method == "POST" {
                let target = parse_json(body).and_then(|j| j.get_str("email").map(|s| s.to_string())).unwrap_or_default();
                ("ADMIN", "USER_CREATE", format!("Menambahkan pengguna baru {}", target), "info")
            } else if method == "DELETE" {
                ("ADMIN", "USER_DELETE", format!("Menghapus pengguna {}", sub_path), "critical")
            } else {
                ("ADMIN", "USER_UPDATE", format!("Memperbarui data pengguna {}", sub_path), "info")
            }
        } else if sub_path.starts_with("/tenants") {
            if method == "POST" {
                ("ADMIN", "TENANT_CREATE", format!("Membuat tenant workspace baru"), "info")
            } else if method == "DELETE" {
                ("ADMIN", "TENANT_DELETE", format!("Menghapus tenant {}", sub_path), "critical")
            } else {
                ("ADMIN", "TENANT_UPDATE", format!("Memperbarui tenant {}", sub_path), "info")
            }
        } else if sub_path.starts_with("/files") {
            if method == "DELETE" {
                ("FILES", "FILE_DELETE", format!("Menghapus berkas {}", sub_path), "warning")
            } else if sub_path.ends_with("/content") {
                ("FILES", "FILE_EDIT_CONTENT", format!("Menyimpan perubahan isi berkas {}", sub_path), "info")
            } else if method == "POST" {
                let fname = parse_json(body).and_then(|j| j.get_str("name").map(|s| s.to_string())).unwrap_or_else(|| "file".to_string());
                ("FILES", "FILE_CREATE", format!("Membuat / mengunggah berkas '{}'", fname), "info")
            } else {
                ("FILES", "FILE_UPDATE", format!("Memperbarui metadata berkas {}", sub_path), "info")
            }
        } else if sub_path.starts_with("/shares") {
            if sub_path.ends_with("/approve") {
                ("SHARES", "SHARE_APPROVED", format!("Menyetujui pengajuan share dokumen {}", sub_path), "info")
            } else if sub_path.ends_with("/reject") {
                ("SHARES", "SHARE_REJECTED", format!("Menolak pengajuan share dokumen {}", sub_path), "warning")
            } else if sub_path.ends_with("/comments") && method == "POST" {
                ("SHARES", "SHARE_COMMENT", format!("Menambahkan catatan/komentar pada share {}", sub_path), "info")
            } else if method == "GET" {
                ("SHARES", "PUBLIC_SHARE_ACCESS", format!("Akses tautan berbagi publik {}", sub_path), "info")
            } else if method == "DELETE" {
                ("SHARES", "SHARE_REVOKE", format!("Mencabut tautan berbagi {}", sub_path), "warning")
            } else {
                ("SHARES", "SHARE_REQUEST", "Mengajukan / membuat tautan berbagi berkas".to_string(), "info")
            }
        } else if sub_path.starts_with("/notes") {
            if method == "DELETE" {
                ("NOTES", "NOTE_DELETE", format!("Menghapus catatan {}", sub_path), "warning")
            } else {
                let title = parse_json(body).and_then(|j| j.get_str("title").map(|s| s.to_string())).unwrap_or_default();
                ("NOTES", "NOTE_SAVE", format!("Menyimpan catatan '{}'", title), "info")
            }
        } else if sub_path.starts_with("/meet") {
            if sub_path == "/meet/rooms" && method == "POST" {
                let title = parse_json(body).and_then(|j| j.get_str("title").map(|s| s.to_string())).unwrap_or_default();
                ("MEET", "MEET_ROOM_CREATE", format!("Membuat ruang rapat '{}'", title), "info")
            } else {
                let sig_type = parse_json(body).and_then(|j| j.get_str("type").map(|s| s.to_string())).unwrap_or_default();
                let room = parse_json(body).and_then(|j| j.get_str("room_code").map(|s| s.to_string())).unwrap_or_default();
                ("MEET", "MEET_ACTIVITY", format!("Aktivitas rapat ({}) di ruang {}", sig_type, room), "info")
            }
        } else if sub_path.starts_with("/gdrive") {
            if sub_path == "/gdrive/connect" {
                let email = parse_json(body).and_then(|j| j.get_str("account_email").map(|s| s.to_string())).unwrap_or_default();
                ("GDRIVE", "GDRIVE_CONNECT", format!("Menghubungkan Google Drive ({})", email), "info")
            } else if sub_path == "/gdrive/disconnect" {
                ("GDRIVE", "GDRIVE_DISCONNECT", "Memutuskan sambungan Google Drive".to_string(), "warning")
            } else if sub_path == "/gdrive/import" {
                let name = parse_json(body).and_then(|j| j.get_str("name").map(|s| s.to_string())).unwrap_or_default();
                ("GDRIVE", "GDRIVE_IMPORT", format!("Mengimpor '{}' dari Google Drive", name), "info")
            } else if sub_path == "/gdrive/export" {
                ("GDRIVE", "GDRIVE_EXPORT", "Mengunggah berkas ke Google Drive".to_string(), "info")
            } else {
                ("GDRIVE", "GDRIVE_SYNC", "Sinkronisasi Google Drive".to_string(), "info")
            }
        } else {
            ("SYSTEM", "API_ACTION", format!("{} {}", method, sub_path), "info")
        };

        let final_severity = if status_code >= 400 && severity == "info" {
            "warning"
        } else {
            severity
        };

        Self::record(
            db_dir,
            &tenant_id,
            &user_id,
            &user_name,
            &role,
            category,
            action,
            method,
            sub_path,
            &detail,
            status_code,
            final_severity,
            ip_address,
        );
    }

    /// POST /api/audit/log - Record custom client-side desktop activity
    pub fn log_client_event(
        body: &str,
        ctx: &AuthContext,
        db: &DbHandle,
        db_dir: &Path,
        ip_address: &str,
    ) -> (u16, String) {
        let json = parse_json(body).unwrap_or(JsonValue::Null);
        let category = json.get_str("category").unwrap_or("DESKTOP");
        let action = json.get_str("action").unwrap_or("USER_ACTIVITY");
        let resource = json.get_str("resource").unwrap_or("desktop");
        let detail = json.get_str("detail").unwrap_or("");
        let severity = json.get_str("severity").unwrap_or("info");
        let user_name = Self::resolve_user_name(&ctx.user_id, db);

        Self::record(
            db_dir,
            &ctx.tenant_id,
            &ctx.user_id,
            &user_name,
            ctx.role.as_str(),
            category,
            action,
            "EVENT",
            resource,
            detail,
            200,
            severity,
            ip_address,
        );

        (201, r#"{"success":true}"#.to_string())
    }

    /// GET /api/audit/logs - List audit logs with optional filtering
    pub fn list_logs(query: &str, ctx: &AuthContext, db_dir: &Path) -> (u16, String) {
        let mut filter_cat = "";
        let mut filter_sev = "";
        let mut search = String::new();
        let mut limit: usize = 250;

        for pair in query.split('&') {
            if let Some((k, v)) = pair.split_once('=') {
                match k {
                    "category" => filter_cat = v,
                    "severity" => filter_sev = v,
                    "search" => search = v.replace("%20", " ").to_lowercase(),
                    "limit" => limit = v.parse().unwrap_or(250).min(1000),
                    _ => {}
                }
            }
        }

        let path = Self::audit_file(db_dir);
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        let is_super = ctx.role == Role::SuperAdmin;

        let mut rows: Vec<String> = Vec::new();
        let mut total_count: usize = 0;
        let mut warning_count: usize = 0;
        let mut critical_count: usize = 0;
        let mut today_count: usize = 0;
        let now = Self::now_secs();
        let day_start = now.saturating_sub(86400);

        for line in content.lines().rev() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if let Some(j) = parse_json(trimmed) {
                let t_id = j.get_str("tenant_id").unwrap_or("");
                if !is_super && t_id != ctx.tenant_id {
                    continue;
                }

                total_count += 1;
                let sev = j.get_str("severity").unwrap_or("info");
                if sev == "warning" {
                    warning_count += 1;
                } else if sev == "critical" {
                    critical_count += 1;
                }
                let ts = j.get_number("timestamp").unwrap_or(0.0) as u64;
                if ts >= day_start {
                    today_count += 1;
                }

                if !filter_cat.is_empty() && filter_cat != "ALL" {
                    let cat = j.get_str("category").unwrap_or("");
                    if !cat.eq_ignore_ascii_case(filter_cat) {
                        continue;
                    }
                }
                if !filter_sev.is_empty() && filter_sev != "ALL" && !sev.eq_ignore_ascii_case(filter_sev) {
                    continue;
                }
                if !search.is_empty() && !trimmed.to_lowercase().contains(&search) {
                    continue;
                }

                if rows.len() < limit {
                    rows.push(trimmed.to_string());
                }
            }
        }

        let resp = format!(
            r#"{{"stats":{{"total":{},"today":{},"warning":{},"critical":{}}},"logs":[{}]}}"#,
            total_count,
            today_count,
            warning_count,
            critical_count,
            rows.join(",")
        );
        (200, resp)
    }

    /// GET /api/audit/export - Export audit logs as CSV
    pub fn export_csv(ctx: &AuthContext, db_dir: &Path) -> (u16, String) {
        let path = Self::audit_file(db_dir);
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        let is_super = ctx.role == Role::SuperAdmin;

        let mut csv = String::from("id,timestamp,tenant_id,user_id,user_name,role,category,action,method,resource,detail,status_code,severity,ip_address\n");
        for line in content.lines().rev() {
            if let Some(j) = parse_json(line.trim()) {
                let t_id = j.get_str("tenant_id").unwrap_or("");
                if !is_super && t_id != ctx.tenant_id {
                    continue;
                }
                let esc = |s: &str| format!("\"{}\"", s.replace('"', "\"\""));
                csv.push_str(&format!(
                    "{},{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
                    esc(j.get_str("id").unwrap_or("")),
                    j.get_number("timestamp").unwrap_or(0.0) as u64,
                    esc(t_id),
                    esc(j.get_str("user_id").unwrap_or("")),
                    esc(j.get_str("user_name").unwrap_or("")),
                    esc(j.get_str("role").unwrap_or("")),
                    esc(j.get_str("category").unwrap_or("")),
                    esc(j.get_str("action").unwrap_or("")),
                    esc(j.get_str("method").unwrap_or("")),
                    esc(j.get_str("resource").unwrap_or("")),
                    esc(j.get_str("detail").unwrap_or("")),
                    j.get_number("status_code").unwrap_or(200.0) as u16,
                    esc(j.get_str("severity").unwrap_or("info")),
                    esc(j.get_str("ip_address").unwrap_or("")),
                ));
            }
        }
        (200, csv)
    }

    /// DELETE /api/audit/logs - Clear audit trail (SuperAdmin only)
    pub fn clear_logs(ctx: &AuthContext, db: &DbHandle, db_dir: &Path, ip_address: &str) -> (u16, String) {
        if ctx.role != Role::SuperAdmin {
            return (403, r#"{"error":"Hanya SuperAdmin yang diizinkan menghapus log audit"}"#.to_string());
        }
        let path = Self::audit_file(db_dir);
        let _ = std::fs::write(&path, "");
        let user_name = Self::resolve_user_name(&ctx.user_id, db);
        Self::record(
            db_dir,
            &ctx.tenant_id,
            &ctx.user_id,
            &user_name,
            ctx.role.as_str(),
            "ADMIN",
            "AUDIT_LOGS_CLEARED",
            "DELETE",
            "/api/audit/logs",
            "Seluruh riwayat audit trail dibersihkan oleh SuperAdmin",
            200,
            "critical",
            ip_address,
        );
        (200, r#"{"success":true}"#.to_string())
    }
}

use crate::api::json_helper::{escape_json, parse_json};
use crate::auth::{AuthContext, Role};
use crate::db::{DbHandle, User};
use std::fs;
use std::process::Command;

pub struct AuthApi;

impl AuthApi {
    /// Discover Linux users from /etc/passwd (1000 <= uid < 60000) and ensure they exist in BijiOS DB
    pub fn sync_linux_system_users(db: &DbHandle) {
        let Ok(passwd) = fs::read_to_string("/etc/passwd") else { return };
        let mut db_lock = db.write().unwrap();
        let default_tenant = db_lock
            .tenants
            .first()
            .map(|t| t.id.clone())
            .unwrap_or_else(|| "tenant_idbsd".to_string());

        let mut added = false;
        for line in passwd.lines() {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() < 7 {
                continue;
            }
            let username = parts[0].trim();
            let Ok(uid) = parts[2].trim().parse::<u32>() else { continue };
            let shell = parts[6].trim();
            if !(1000..60000).contains(&uid)
                || shell.ends_with("nologin")
                || shell.ends_with("false")
            {
                continue;
            }

            let exists = db_lock.users.iter().any(|u| {
                u.name.eq_ignore_ascii_case(username)
                    || u.email.split('@').next().unwrap_or("").eq_ignore_ascii_case(username)
            });
            if !exists {
                let initials: String = username
                    .chars()
                    .take(2)
                    .collect::<String>()
                    .to_uppercase();
                db_lock.users.push(User {
                    id: format!("usr_pam_{}_{}", username, uid),
                    tenant_id: default_tenant.clone(),
                    email: format!("{}@bijios.local", username),
                    name: username.to_string(),
                    password: "pam_managed_account".to_string(),
                    role: Role::SuperAdmin,
                    status: "active".to_string(),
                    created_at: "2026-01-01".to_string(),
                    avatar: initials,
                    color: "#007aff".to_string(),
                    quota_bytes: 32212254720,
                    department: "BijiOS System".to_string(),
                    supervisor_id: String::new(),
                });
                added = true;
            }
        }
        if added {
            db_lock.save_users();
        }
    }

    /// Authenticate username & password against Linux PAM (libpam.so.0)
    pub fn verify_linux_pam(username: &str, password: &str) -> bool {
        let clean_user = username.split('@').next().unwrap_or(username).trim();
        if clean_user.is_empty() || password.is_empty() {
            return false;
        }

        let py_script = r#"
import ctypes, os, sys
user = os.environ.get("PAM_USER", "").encode("utf-8")
pw = os.environ.get("PAM_PASS", "").encode("utf-8")
try:
    libpam = ctypes.CDLL("libpam.so.0")
    libc = ctypes.CDLL("libc.so.6")
except Exception:
    sys.exit(2)

class PamMessage(ctypes.Structure):
    _fields_ = [("msg_style", ctypes.c_int), ("msg", ctypes.c_char_p)]

class PamResponse(ctypes.Structure):
    _fields_ = [("resp", ctypes.c_char_p), ("resp_retcode", ctypes.c_int)]

CONV_FUNC = ctypes.CFUNCTYPE(
    ctypes.c_int, ctypes.c_int,
    ctypes.POINTER(ctypes.POINTER(PamMessage)),
    ctypes.POINTER(ctypes.POINTER(PamResponse)),
    ctypes.c_void_p
)

class PamConv(ctypes.Structure):
    _fields_ = [("conv", CONV_FUNC), ("appdata_ptr", ctypes.c_void_p)]

calloc = libc.calloc
calloc.restype = ctypes.c_void_p
calloc.argtypes = [ctypes.c_size_t, ctypes.c_size_t]
strdup = libc.strdup
strdup.restype = ctypes.c_void_p
strdup.argtypes = [ctypes.c_char_p]

def conv_cb(n_msgs, msg_list, resp_ptr, appdata):
    arr_ptr = calloc(n_msgs, ctypes.sizeof(PamResponse))
    if not arr_ptr:
        return 19
    resp_arr = ctypes.cast(arr_ptr, ctypes.POINTER(PamResponse))
    for i in range(n_msgs):
        m = msg_list[i].contents
        if m.msg_style in (1, 2):
            dup = strdup(pw)
            resp_arr[i].resp = ctypes.cast(dup, ctypes.c_char_p)
            resp_arr[i].resp_retcode = 0
    resp_ptr[0] = resp_arr
    return 0

cb = CONV_FUNC(conv_cb)
conv = PamConv(cb, None)
handle = ctypes.c_void_p()
rc = libpam.pam_start(b"login", user, ctypes.byref(conv), ctypes.byref(handle))
if rc != 0:
    sys.exit(1)
rc = libpam.pam_authenticate(handle, 0)
libpam.pam_end(handle, rc)
sys.exit(0 if rc == 0 else 1)
"#;

        if let Ok(status) = Command::new("python3")
            .arg("-c")
            .arg(py_script)
            .env("PAM_USER", clean_user)
            .env("PAM_PASS", password)
            .status()
        {
            if status.success() {
                return true;
            }
        }
        false
    }

    pub fn login(body: &str, db: &DbHandle) -> (u16, String) {
        Self::sync_linux_system_users(db);
        let json = match parse_json(body) {
            Some(j) => j,
            None => return (400, r#"{"error":"Format JSON tidak valid"}"#.to_string()),
        };

        let identity = json
            .get_str("email")
            .or_else(|| json.get_str("username"))
            .unwrap_or("")
            .trim();
        let password = json.get_str("password").unwrap_or("").trim();

        if identity.is_empty() || password.is_empty() {
            return (400, r#"{"error":"Email/Username dan password wajib diisi"}"#.to_string());
        }

        let pam_ok = Self::verify_linux_pam(identity, password);
        let identity_short = identity.split('@').next().unwrap_or(identity);

        let db_lock = db.read().unwrap();
        let user = db_lock.users.iter().find(|u| {
            let id_match = u.email.eq_ignore_ascii_case(identity)
                || u.name.eq_ignore_ascii_case(identity)
                || u.name.eq_ignore_ascii_case(identity_short)
                || u.id == identity;
            id_match && (pam_ok || u.password == password)
        });

        if let Some(u) = user {
            if u.status == "suspended" {
                return (403, r#"{"error":"Akun Anda sedang dinonaktifkan / disuspend"}"#.to_string());
            }

            let tenant = db_lock.tenants.iter().find(|t| t.id == u.tenant_id);
            let tenant_name = tenant.map(|t| t.name.as_str()).unwrap_or("Workspace");
            let token = format!("dkh_token:{}:{}", u.id, u.tenant_id);

            let res = format!(
                r#"{{
                    "success": true,
                    "token": "{}",
                    "user": {{
                        "id": "{}",
                        "name": "{}",
                        "email": "{}",
                        "role": "{}",
                        "tenant_id": "{}",
                        "tenant_name": "{}",
                        "avatar": "{}",
                        "color": "{}",
                        "storage_quota_bytes": {},
                        "quota_bytes": {}
                    }}
                }}"#,
                escape_json(&token),
                escape_json(&u.id),
                escape_json(&u.name),
                escape_json(&u.email),
                u.role.as_str(),
                escape_json(&u.tenant_id),
                escape_json(tenant_name),
                escape_json(&u.avatar),
                escape_json(&u.color),
                u.quota_bytes,
                u.quota_bytes
            );
            (200, res)
        } else {
            (401, r#"{"error":"Kredensial tidak valid. Silakan periksa username/email dan password Anda."}"#.to_string())
        }
    }

    pub fn me(ctx: &AuthContext, db: &DbHandle) -> (u16, String) {
        let db_lock = db.read().unwrap();
        let tenant = db_lock.tenants.iter().find(|t| t.id == ctx.tenant_id);
        let tenant_name = tenant.map(|t| t.name.as_str()).unwrap_or("Unknown");
        let user = db_lock.users.iter().find(|u| u.id == ctx.user_id);
        let avatar = user.map(|u| u.avatar.as_str()).unwrap_or("");
        let color = user.map(|u| u.color.as_str()).unwrap_or("#007aff");
        let quota_bytes = user.map(|u| u.quota_bytes).unwrap_or(21474836480);

        let res = format!(
            r#"{{
                "id": "{}",
                "name": "{}",
                "email": "{}",
                "role": "{}",
                "tenant_id": "{}",
                "tenant_name": "{}",
                "avatar": "{}",
                "color": "{}",
                "storage_quota_bytes": {},
                "quota_bytes": {}
            }}"#,
            escape_json(&ctx.user_id),
            escape_json(&ctx.name),
            escape_json(&ctx.email),
            ctx.role.as_str(),
            escape_json(&ctx.tenant_id),
            escape_json(tenant_name),
            escape_json(avatar),
            escape_json(color),
            quota_bytes,
            quota_bytes
        );
        (200, res)
    }

    pub fn list_tenants(ctx: &AuthContext, db: &DbHandle) -> (u16, String) {
        let db_lock = db.read().unwrap();
        let items: Vec<String> = db_lock
            .tenants
            .iter()
            .filter(|t| ctx.role == Role::SuperAdmin || t.id == ctx.tenant_id)
            .map(|t| {
                let user_count = db_lock.users.iter().filter(|u| u.tenant_id == t.id).count();
                let file_count = db_lock.files.iter().filter(|f| f.tenant_id == t.id).count();
                format!(
                    r#"{{"id":"{}","name":"{}","slug":"{}","plan":"{}","user_count":{},"file_count":{}}}"#,
                    escape_json(&t.id),
                    escape_json(&t.name),
                    escape_json(&t.slug),
                    escape_json(&t.plan),
                    user_count,
                    file_count
                )
            })
            .collect();
        (200, format!("[{}]", items.join(",")))
    }

    pub fn create_tenant(body: &str, ctx: &AuthContext, db: &DbHandle) -> (u16, String) {
        if ctx.role != Role::SuperAdmin {
            return (403, r#"{"error":"Hanya SuperAdmin yang dapat membuat tenant baru"}"#.to_string());
        }

        let json = match parse_json(body) {
            Some(j) => j,
            None => return (400, r#"{"error":"Format JSON tidak valid"}"#.to_string()),
        };

        let name = json.get_str("name").unwrap_or("").trim();
        let slug = json.get_str("slug").unwrap_or("").trim();
        let plan = json.get_str("plan").unwrap_or("Enterprise").trim();

        if name.is_empty() {
            return (400, r#"{"error":"Nama organisasi/tenant wajib diisi"}"#.to_string());
        }

        let generated_slug = if slug.is_empty() {
            name.to_lowercase()
                .chars()
                .map(|c| if c.is_alphanumeric() { c } else { '-' })
                .collect::<String>()
        } else {
            slug.to_string()
        };

        let mut db_lock = db.write().unwrap();
        if db_lock.tenants.iter().any(|t| t.slug == generated_slug) {
            return (400, r#"{"error":"Slug tenant sudah terdaftar"}"#.to_string());
        }

        let new_id = format!("tenant_{}", generated_slug.replace('-', "_"));
        let now = format!(
            "{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        );

        let new_tenant = crate::db::Tenant {
            id: new_id.clone(),
            name: name.to_string(),
            slug: generated_slug,
            plan: plan.to_string(),
            created_at: now,
        };

        db_lock.tenants.push(new_tenant);
        db_lock.save_tenants();

        (201, format!(r#"{{"success":true,"id":"{}","message":"Tenant berhasil dibuat"}}"#, escape_json(&new_id)))
    }

    pub fn update_tenant(id: &str, body: &str, ctx: &AuthContext, db: &DbHandle) -> (u16, String) {
        if ctx.role != Role::SuperAdmin {
            return (403, r#"{"error":"Hanya SuperAdmin yang dapat mengedit tenant"}"#.to_string());
        }

        let json = match parse_json(body) {
            Some(j) => j,
            None => return (400, r#"{"error":"Format JSON tidak valid"}"#.to_string()),
        };

        let mut db_lock = db.write().unwrap();
        let idx = match db_lock.tenants.iter().position(|t| t.id == id) {
            Some(i) => i,
            None => return (404, r#"{"error":"Tenant tidak ditemukan"}"#.to_string()),
        };

        if let Some(name) = json.get_str("name") {
            if !name.trim().is_empty() {
                db_lock.tenants[idx].name = name.trim().to_string();
            }
        }
        if let Some(slug) = json.get_str("slug") {
            if !slug.trim().is_empty() {
                db_lock.tenants[idx].slug = slug.trim().to_string();
            }
        }
        if let Some(plan) = json.get_str("plan") {
            if !plan.trim().is_empty() {
                db_lock.tenants[idx].plan = plan.trim().to_string();
            }
        }

        db_lock.save_tenants();
        (200, r#"{"success":true,"message":"Tenant berhasil diperbarui"}"#.to_string())
    }

    pub fn delete_tenant(id: &str, ctx: &AuthContext, db: &DbHandle) -> (u16, String) {
        if ctx.role != Role::SuperAdmin {
            return (403, r#"{"error":"Hanya SuperAdmin yang dapat menghapus tenant"}"#.to_string());
        }

        if id == ctx.tenant_id {
            return (400, r#"{"error":"Tidak dapat menghapus tenant yang sedang aktif digunakan"}"#.to_string());
        }

        let mut db_lock = db.write().unwrap();
        let user_count = db_lock.users.iter().filter(|u| u.tenant_id == id).count();
        if user_count > 0 {
            return (
                400,
                format!(
                    r#"{{"error":"Tidak dapat menghapus tenant karena masih memiliki {} pengguna terdaftar"}}"#,
                    user_count
                ),
            );
        }

        let pos = match db_lock.tenants.iter().position(|t| t.id == id) {
            Some(p) => p,
            None => return (404, r#"{"error":"Tenant tidak ditemukan"}"#.to_string()),
        };

        db_lock.tenants.remove(pos);
        db_lock.save_tenants();

        (200, r#"{"success":true,"message":"Tenant berhasil dihapus"}"#.to_string())
    }

    pub fn list_users(ctx: &AuthContext, db: &DbHandle) -> (u16, String) {
        Self::sync_linux_system_users(db);
        let db_lock = db.read().unwrap();
        let users: Vec<String> = db_lock
            .users
            .iter()
            .filter(|u| {
                if ctx.role == Role::SuperAdmin {
                    true
                } else {
                    u.tenant_id == ctx.tenant_id
                }
            })
            .map(|u| {
                let used_bytes: usize = db_lock
                    .files
                    .iter()
                    .filter(|f| f.owner_id == u.id && !f.is_folder)
                    .map(|f| f.size_bytes)
                    .sum();
                let quota_mb = (u.quota_bytes as f64) / (1024.0 * 1024.0);
                let used_mb = (used_bytes as f64) / (1024.0 * 1024.0);
                let percent = if quota_mb > 0.0 { (used_mb / quota_mb) * 100.0 } else { 0.0 };
                let supervisor_name = db_lock
                    .users
                    .iter()
                    .find(|sup| sup.id == u.supervisor_id)
                    .map(|sup| sup.name.as_str())
                    .unwrap_or("");

                format!(
                    r#"{{"id":"{}","tenant_id":"{}","name":"{}","email":"{}","role":"{}","status":"{}","avatar":"{}","color":"{}","department":"{}","supervisor_id":"{}","supervisor_name":"{}","storage_quota_bytes":{},"quota_bytes":{},"storage_used_mb":{:.2},"storage_quota_mb":{:.0},"storage_percent":{:.1}}}"#,
                    escape_json(&u.id),
                    escape_json(&u.tenant_id),
                    escape_json(&u.name),
                    escape_json(&u.email),
                    u.role.as_str(),
                    escape_json(&u.status),
                    escape_json(&u.avatar),
                    escape_json(&u.color),
                    escape_json(&u.department),
                    escape_json(&u.supervisor_id),
                    escape_json(supervisor_name),
                    u.quota_bytes,
                    u.quota_bytes,
                    used_mb,
                    quota_mb,
                    percent
                )
            })
            .collect();
        (200, format!("[{}]", users.join(",")))
    }

    pub fn invite_user(body: &str, ctx: &AuthContext, db: &DbHandle) -> (u16, String) {
        if !ctx.role.can_manage_users() {
            return (403, r#"{"error":"Hanya Admin atau SuperAdmin yang dapat mengundang user"}"#.to_string());
        }

        let json = match parse_json(body) {
            Some(j) => j,
            None => return (400, r#"{"error":"JSON tidak valid"}"#.to_string()),
        };

        let email = json.get_str("email").unwrap_or("").trim();
        let name = json.get_str("name").unwrap_or("").trim();
        let role_str = json.get_str("role").unwrap_or("Member").trim();
        let target_tenant = if ctx.role == Role::SuperAdmin {
            json.get_str("tenant_id").unwrap_or(&ctx.tenant_id).trim()
        } else {
            &ctx.tenant_id
        };

        if email.is_empty() || name.is_empty() {
            return (400, r#"{"error":"Nama dan email harus diisi"}"#.to_string());
        }

        let mut db_lock = db.write().unwrap();
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let clean_prefix: String = name
            .to_lowercase()
            .chars()
            .filter(|c| c.is_alphanumeric())
            .take(8)
            .collect();
        let prefix = if clean_prefix.is_empty() { "user".to_string() } else { clean_prefix };
        let mut new_id = format!("usr_{}_{}", prefix, timestamp % 10000);
        let mut counter = 1;
        while db_lock.users.iter().any(|u| u.id == new_id) {
            new_id = format!("usr_{}_{}_{}", prefix, timestamp % 10000, counter);
            counter += 1;
        }

        let quota_bytes: u64 = json.get_str("storage_quota_bytes")
            .or_else(|| json.get_str("quota_bytes"))
            .and_then(|s| s.parse().ok())
            .unwrap_or(10737418240);

        let department = json.get_str("department").unwrap_or("").trim().to_string();
        let supervisor_id = json.get_str("supervisor_id").unwrap_or("").trim().to_string();

        let new_user = crate::db::User {
            id: new_id.clone(),
            tenant_id: target_tenant.to_string(),
            email: email.to_string(),
            name: name.to_string(),
            password: "password123".to_string(),
            role: Role::from_str(role_str),
            status: "active".to_string(),
            created_at: format!("{}", timestamp),
            avatar: String::new(),
            color: "#007aff".to_string(),
            quota_bytes,
            department,
            supervisor_id,
        };

        db_lock.users.push(new_user);
        db_lock.save_users();

        (201, format!(r#"{{"success":true,"id":"{}"}}"#, escape_json(&new_id)))
    }

    pub fn update_user(user_id: &str, body: &str, ctx: &AuthContext, db: &DbHandle) -> (u16, String) {
        let json = match parse_json(body) {
            Some(j) => j,
            None => return (400, r#"{"error":"JSON tidak valid"}"#.to_string()),
        };

        let is_self = ctx.user_id == user_id;
        let is_admin = ctx.role.can_manage_users();

        if !is_self && !is_admin {
            return (403, r#"{"error":"Tidak memiliki izin memperbarui profil pengguna ini"}"#.to_string());
        }

        let mut db_lock = db.write().unwrap();
        let user_idx = match db_lock.users.iter().position(|u| u.id == user_id) {
            Some(idx) => idx,
            None => return (404, r#"{"error":"Pengguna tidak ditemukan"}"#.to_string()),
        };

        // If admin (not superadmin), ensure target user belongs to same tenant
        if ctx.role == Role::Admin && db_lock.users[user_idx].tenant_id != ctx.tenant_id {
            return (403, r#"{"error":"Admin hanya dapat mengelola pengguna di organisasi sendiri"}"#.to_string());
        }

        if let Some(name) = json.get_str("name") {
            if !name.trim().is_empty() {
                db_lock.users[user_idx].name = name.trim().to_string();
            }
        }

        if let Some(email) = json.get_str("email") {
            if !email.trim().is_empty() {
                db_lock.users[user_idx].email = email.trim().to_string();
            }
        }

        // Only Admin or SuperAdmin can change role and status
        if is_admin {
            if let Some(role_str) = json.get_str("role") {
                // Admin cannot make someone SuperAdmin
                let target_role = Role::from_str(role_str.trim());
                if ctx.role != Role::SuperAdmin && target_role == Role::SuperAdmin {
                    return (403, r#"{"error":"Hanya SuperAdmin yang dapat menetapkan role SuperAdmin"}"#.to_string());
                }
                db_lock.users[user_idx].role = target_role;
            }
            if let Some(status) = json.get_str("status") {
                db_lock.users[user_idx].status = status.trim().to_string();
            }
            if let Some(dept) = json.get_str("department") {
                db_lock.users[user_idx].department = dept.trim().to_string();
            }
            if let Some(sup_id) = json.get_str("supervisor_id") {
                db_lock.users[user_idx].supervisor_id = sup_id.trim().to_string();
            }
            // SuperAdmin can change user's tenant
            if ctx.role == Role::SuperAdmin {
                if let Some(tid) = json.get_str("tenant_id") {
                    if !tid.trim().is_empty() {
                        db_lock.users[user_idx].tenant_id = tid.trim().to_string();
                    }
                }
            }
        }

        // Direct password change in user update (if provided)
        if let Some(pass) = json.get_str("password") {
            if !pass.trim().is_empty() {
                db_lock.users[user_idx].password = pass.trim().to_string();
            }
        }

        // Handle Avatar update
        if let Some(av) = json.get_str("avatar") {
            let av = av.trim();
            if av.starts_with("data:image") {
                if let Some(comma_pos) = av.find(',') {
                    let b64 = &av[comma_pos + 1..];
                    if let Some(bytes) = crate::api::files::FilesApi::decode_base64(b64) {
                        let _ = std::fs::create_dir_all("/home/dokuhub/apps/dokuhub/data/storage/avatars");
                        let avatar_path = format!("/home/dokuhub/apps/dokuhub/data/storage/avatars/{}.png", user_id);
                        let _ = std::fs::write(&avatar_path, &bytes);
                        let cache_bust = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
                        db_lock.users[user_idx].avatar = format!("/api/users/{}/avatar?v={}", user_id, cache_bust);
                    } else {
                        db_lock.users[user_idx].avatar = av.to_string();
                    }
                } else {
                    db_lock.users[user_idx].avatar = av.to_string();
                }
            } else {
                db_lock.users[user_idx].avatar = av.to_string();
            }
        }

        // Handle Color update
        if let Some(col) = json.get_str("color") {
            if !col.trim().is_empty() {
                db_lock.users[user_idx].color = col.trim().to_string();
            }
        }

        // Handle Quota update (1GB, 5GB, 10GB, 20GB, 30GB or Custom in bytes)
        if let Some(q_str) = json.get_str("storage_quota_bytes").or_else(|| json.get_str("quota_bytes")) {
            if let Ok(q) = q_str.parse::<u64>() {
                if q > 0 {
                    db_lock.users[user_idx].quota_bytes = q;
                }
            }
        }

        let updated = &db_lock.users[user_idx];
        let res = format!(
            r#"{{"success":true,"user":{{"id":"{}","name":"{}","email":"{}","role":"{}","status":"{}","tenant_id":"{}","avatar":"{}","color":"{}","storage_quota_bytes":{},"quota_bytes":{}}}}}"#,
            escape_json(&updated.id),
            escape_json(&updated.name),
            escape_json(&updated.email),
            updated.role.as_str(),
            escape_json(&updated.status),
            escape_json(&updated.tenant_id),
            escape_json(&updated.avatar),
            escape_json(&updated.color),
            updated.quota_bytes,
            updated.quota_bytes
        );

        db_lock.save_users();
        (200, res)
    }

    pub fn change_password(user_id: &str, body: &str, ctx: &AuthContext, db: &DbHandle) -> (u16, String) {
        let json = match parse_json(body) {
            Some(j) => j,
            None => return (400, r#"{"error":"JSON tidak valid"}"#.to_string()),
        };

        let old_pass = json.get_str("old_password").unwrap_or("").trim();
        let new_pass = json.get_str("new_password").unwrap_or("").trim();

        if new_pass.len() < 4 {
            return (400, r#"{"error":"Kata sandi baru minimal 4 karakter"}"#.to_string());
        }

        let is_self = ctx.user_id == user_id;
        let is_superadmin = ctx.role == Role::SuperAdmin;

        if !is_self && !is_superadmin {
            return (403, r#"{"error":"Tidak diizinkan mengubah kata sandi pengguna lain"}"#.to_string());
        }

        let mut db_lock = db.write().unwrap();
        let user_idx = match db_lock.users.iter().position(|u| u.id == user_id) {
            Some(idx) => idx,
            None => return (404, r#"{"error":"Pengguna tidak ditemukan"}"#.to_string()),
        };

        if is_self && !is_superadmin {
            let current_pw = &db_lock.users[user_idx].password;
            if !old_pass.is_empty() && current_pw != old_pass {
                return (400, r#"{"error":"Kata sandi saat ini (lama) tidak sesuai"}"#.to_string());
            }
        }

        db_lock.users[user_idx].password = new_pass.to_string();
        db_lock.save_users();

        (200, r#"{"success":true,"message":"Kata sandi berhasil diperbarui"}"#.to_string())
    }

    pub fn delete_user(user_id: &str, ctx: &AuthContext, db: &DbHandle) -> (u16, String) {
        if !ctx.role.can_manage_users() {
            return (403, r#"{"error":"Hanya Admin atau SuperAdmin yang dapat menghapus pengguna"}"#.to_string());
        }

        if ctx.user_id == user_id {
            return (400, r#"{"error":"Anda tidak dapat menghapus akun Anda sendiri yang sedang aktif"}"#.to_string());
        }

        let mut db_lock = db.write().unwrap();
        let pos = match db_lock.users.iter().position(|u| u.id == user_id) {
            Some(p) => p,
            None => return (404, r#"{"error":"Pengguna tidak ditemukan"}"#.to_string()),
        };

        if ctx.role == Role::Admin && db_lock.users[pos].tenant_id != ctx.tenant_id {
            return (403, r#"{"error":"Admin hanya dapat mengelola pengguna di organisasi sendiri"}"#.to_string());
        }

        if db_lock.users[pos].role == Role::SuperAdmin {
            let count_super = db_lock.users.iter().filter(|u| u.role == Role::SuperAdmin).count();
            if count_super <= 1 {
                return (400, r#"{"error":"Tidak dapat menghapus SuperAdmin terakhir dalam sistem"}"#.to_string());
            }
        }

        db_lock.users.remove(pos);
        db_lock.save_users();

        (200, r#"{"success":true,"message":"Pengguna berhasil dihapus"}"#.to_string())
    }
}

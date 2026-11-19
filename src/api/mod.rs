pub mod audit;
pub mod auth;
pub mod files;
pub mod gdrive;
pub mod json_helper;
pub mod meet;
pub mod notes;
pub mod os;
pub mod settings;
pub mod shares;

use crate::auth::AuthContext;
use crate::db::DbHandle;
use meet::MeetSignalingHandle;
use std::path::Path;

pub struct ApiRouter;

impl ApiRouter {
    #[allow(clippy::too_many_arguments)]
    pub fn route(
        method: &str,
        path: &str,
        query: &str,
        body: &str,
        ctx: &AuthContext,
        db: &DbHandle,
        meet_signals: &MeetSignalingHandle,
        db_dir: &Path,
        client_ip: &str,
    ) -> Option<(u16, String, &'static str)> {
        // Strip "/api" prefix
        if !path.starts_with("/api") {
            return None;
        }

        let sub_path = &path[4..];
        let result = Self::route_inner(method, sub_path, query, body, ctx, db, meet_signals, db_dir, client_ip);
        if let Some((code, _, _)) = &result {
            audit::AuditApi::auto_record_request(method, sub_path, body, *code, ctx, db, db_dir, client_ip);
        }
        result
    }

    #[allow(clippy::too_many_arguments)]
    fn route_inner(
        method: &str,
        sub_path: &str,
        query: &str,
        body: &str,
        ctx: &AuthContext,
        db: &DbHandle,
        meet_signals: &MeetSignalingHandle,
        db_dir: &Path,
        client_ip: &str,
    ) -> Option<(u16, String, &'static str)> {

        // 1. Auth routes
        if method == "POST" && sub_path == "/auth/login" {
            let (code, res) = auth::AuthApi::login(body, db);
            return Some((code, res, "application/json"));
        }
        if method == "GET" && sub_path == "/auth/me" {
            let (code, res) = auth::AuthApi::me(ctx, db);
            return Some((code, res, "application/json"));
        }
        if method == "GET" && sub_path == "/tenants" {
            let (code, res) = auth::AuthApi::list_tenants(ctx, db);
            return Some((code, res, "application/json"));
        }
        if method == "POST" && sub_path == "/tenants" {
            let (code, res) = auth::AuthApi::create_tenant(body, ctx, db);
            return Some((code, res, "application/json"));
        }
        if sub_path.starts_with("/tenants/") {
            let rest = &sub_path[9..];
            if method == "PUT" || method == "PATCH" {
                let (code, res) = auth::AuthApi::update_tenant(rest, body, ctx, db);
                return Some((code, res, "application/json"));
            }
            if method == "DELETE" {
                let (code, res) = auth::AuthApi::delete_tenant(rest, ctx, db);
                return Some((code, res, "application/json"));
            }
        }
        if method == "GET" && sub_path == "/users" {
            let (code, res) = auth::AuthApi::list_users(ctx, db);
            return Some((code, res, "application/json"));
        }
        if method == "POST" && sub_path == "/users" {
            let (code, res) = auth::AuthApi::invite_user(body, ctx, db);
            return Some((code, res, "application/json"));
        }
        if sub_path.starts_with("/users/") {
            let rest = &sub_path[7..];
            if let Some(user_id) = rest.strip_suffix("/password") {
                if method == "PUT" || method == "POST" {
                    let (code, res) = auth::AuthApi::change_password(user_id, body, ctx, db);
                    return Some((code, res, "application/json"));
                }
            } else {
                if method == "PUT" || method == "PATCH" {
                    let (code, res) = auth::AuthApi::update_user(rest, body, ctx, db);
                    return Some((code, res, "application/json"));
                }
                if method == "DELETE" {
                    let (code, res) = auth::AuthApi::delete_user(rest, ctx, db);
                    return Some((code, res, "application/json"));
                }
            }
        }

        // 2. Files routes
        if method == "GET" && sub_path == "/storage/quota" {
            let (code, res) = files::FilesApi::get_storage_quota(query, ctx, db, db_dir);
            return Some((code, res, "application/json"));
        }
        if method == "GET" && sub_path == "/files" {
            let (code, res) = files::FilesApi::list(query, ctx, db, db_dir);
            return Some((code, res, "application/json"));
        }
        if method == "POST" && (sub_path == "/files" || sub_path == "/files/upload") {
            let (code, res) = files::FilesApi::create_or_save(body, ctx, db, db_dir);
            return Some((code, res, "application/json"));
        }
        if sub_path.starts_with("/files/") {
            let rest = &sub_path[7..];
            if let Some(id) = rest.strip_suffix("/content") {
                if method == "GET" {
                    let (code, res) = files::FilesApi::get_content(id, ctx, db, db_dir);
                    return Some((code, res, "application/json"));
                } else if method == "PUT" || method == "POST" {
                    let (code, res) = files::FilesApi::update_content(id, body, ctx, db, db_dir);
                    return Some((code, res, "application/json"));
                }
            } else {
                if method == "GET" {
                    let (code, res) = files::FilesApi::get_metadata(rest, ctx, db);
                    return Some((code, res, "application/json"));
                } else if method == "PUT" || method == "PATCH" || (method == "POST" && !rest.is_empty()) {
                    // Rename / update file attributes
                    let mut req_body = body.to_string();
                    if !req_body.contains(r#""id""#) {
                        let id_field = format!(r#""id":"{}","#, rest);
                        if req_body.trim().starts_with('{') {
                            req_body = req_body.replacen('{', &format!("{{{}", id_field), 1);
                        }
                    }
                    let (code, res) = files::FilesApi::create_or_save(&req_body, ctx, db, db_dir);
                    return Some((code, res, "application/json"));
                } else if method == "DELETE" {
                    let (code, res) = files::FilesApi::delete(rest, ctx, db, db_dir);
                    return Some((code, res, "application/json"));
                }
            }
        }

        // 3. Shares routes
        if method == "GET" && sub_path == "/shares" {
            let (code, res) = shares::SharesApi::list(ctx, db);
            return Some((code, res, "application/json"));
        }
        if method == "POST" && sub_path == "/shares" {
            let (code, res) = shares::SharesApi::create(body, ctx, db);
            return Some((code, res, "application/json"));
        }
        if sub_path.starts_with("/shares/public/") {
            let token = &sub_path[15..];
            let password_opt = query.split('&').find_map(|p| {
                p.split_once('=').and_then(|(k, v)| if k == "password" { Some(v) } else { None })
            });
            let (code, res) = shares::SharesApi::get_public(token, password_opt, db);
            return Some((code, res, "application/json"));
        }
        if sub_path.starts_with("/shares/") {
            let rest = &sub_path[8..];
            if let Some(share_id) = rest.strip_suffix("/approve") {
                if method == "POST" || method == "PUT" {
                    let (code, res) = shares::SharesApi::review_approval(share_id, true, body, ctx, db);
                    return Some((code, res, "application/json"));
                }
            } else if let Some(share_id) = rest.strip_suffix("/reject") {
                if method == "POST" || method == "PUT" {
                    let (code, res) = shares::SharesApi::review_approval(share_id, false, body, ctx, db);
                    return Some((code, res, "application/json"));
                }
            } else if let Some(share_id) = rest.strip_suffix("/comments") {
                if method == "GET" {
                    let (code, res) = shares::SharesApi::list_comments(share_id, db_dir);
                    return Some((code, res, "application/json"));
                } else if method == "POST" {
                    let (code, res) = shares::SharesApi::add_comment(share_id, body, ctx, db, db_dir);
                    return Some((code, res, "application/json"));
                }
            } else if method == "DELETE" {
                let (code, res) = shares::SharesApi::delete(rest, ctx, db);
                return Some((code, res, "application/json"));
            }
        }

        // 4. Notes routes
        if method == "GET" && sub_path == "/notes" {
            let (code, res) = notes::NotesApi::list(ctx, db);
            return Some((code, res, "application/json"));
        }
        if method == "POST" && sub_path == "/notes" {
            let (code, res) = notes::NotesApi::save(body, ctx, db);
            return Some((code, res, "application/json"));
        }
        if method == "DELETE" && sub_path.starts_with("/notes/") {
            let note_id = &sub_path[7..];
            let (code, res) = notes::NotesApi::delete(note_id, ctx, db);
            return Some((code, res, "application/json"));
        }

        // 5. Meet WebRTC routes
        if method == "GET" && sub_path == "/meet/rooms" {
            let (code, res) = meet::MeetApi::list_rooms(ctx, db, meet_signals);
            return Some((code, res, "application/json"));
        }
        if method == "POST" && sub_path == "/meet/rooms" {
            let (code, res) = meet::MeetApi::create_room(body, ctx, db);
            return Some((code, res, "application/json"));
        }
        if method == "POST" && sub_path == "/meet/signal" {
            let (code, res) = meet::MeetApi::handle_signal(body, meet_signals);
            return Some((code, res, "application/json"));
        }
        if method == "GET" && sub_path.starts_with("/meet/signals/") {
            let room_code = &sub_path[14..];
            let mut peer_id = "";
            let mut since_ts = 0;
            for p in query.split('&') {
                if let Some((k, v)) = p.split_once('=') {
                    if k == "peer_id" {
                        peer_id = v;
                    } else if k == "since" {
                        since_ts = v.parse().unwrap_or(0);
                    }
                }
            }
            let (code, res) = meet::MeetApi::poll_signals(room_code, peer_id, since_ts, meet_signals);
            return Some((code, res, "application/json"));
        }

        // 6. GDrive routes
        if method == "GET" && sub_path == "/gdrive/status" {
            let (code, res) = gdrive::GDriveApi::status(ctx, db_dir);
            return Some((code, res, "application/json"));
        }
        if method == "POST" && sub_path == "/gdrive/connect" {
            let (code, res) = gdrive::GDriveApi::connect(body, ctx, db_dir);
            return Some((code, res, "application/json"));
        }
        if method == "POST" && sub_path == "/gdrive/disconnect" {
            let (code, res) = gdrive::GDriveApi::disconnect(ctx, db_dir);
            return Some((code, res, "application/json"));
        }
        if method == "GET" && sub_path == "/gdrive/files" {
            let (code, res) = gdrive::GDriveApi::list_remote_files(ctx, db_dir);
            return Some((code, res, "application/json"));
        }
        if method == "POST" && sub_path == "/gdrive/import" {
            let (code, res) = gdrive::GDriveApi::import_file(body, ctx, db, db_dir);
            return Some((code, res, "application/json"));
        }
        if method == "POST" && sub_path == "/gdrive/export" {
            let (code, res) = gdrive::GDriveApi::export_file(body, ctx, db, db_dir);
            return Some((code, res, "application/json"));
        }
        if method == "POST" && sub_path == "/gdrive/sync" {
            let (code, res) = gdrive::GDriveApi::sync(ctx, db_dir);
            return Some((code, res, "application/json"));
        }

        // 7. Settings & Integrations
        if method == "GET" && sub_path == "/settings/api-keys" {
            let (code, res) = settings::SettingsApi::list_api_keys(ctx, db);
            return Some((code, res, "application/json"));
        }
        if method == "POST" && sub_path == "/settings/api-keys" {
            let (code, res) = settings::SettingsApi::create_api_key(body, ctx, db);
            return Some((code, res, "application/json"));
        }
        if method == "GET" && sub_path == "/settings/toon-inspect" {
            let (code, res) = settings::SettingsApi::inspect_toon_db(db_dir);
            return Some((code, res, "application/json"));
        }
        if method == "GET" && sub_path == "/system/metrics" {
            let (code, res) = settings::SettingsApi::get_system_metrics(db);
            return Some((code, res, "application/json"));
        }
        if method == "GET" && sub_path == "/docs/openapi.json" {
            let (code, res) = settings::SettingsApi::openapi_spec();
            return Some((code, res, "application/json"));
        }

        // 8. Security Audit Trail routes
        if method == "GET" && sub_path == "/audit/logs" {
            let (code, res) = audit::AuditApi::list_logs(query, ctx, db_dir);
            return Some((code, res, "application/json"));
        }
        if method == "POST" && sub_path == "/audit/log" {
            let (code, res) = audit::AuditApi::log_client_event(body, ctx, db, db_dir, client_ip);
            return Some((code, res, "application/json"));
        }
        if method == "GET" && sub_path == "/audit/export" {
            let (code, res) = audit::AuditApi::export_csv(ctx, db_dir);
            return Some((code, res, "text/csv; charset=utf-8"));
        }
        if method == "DELETE" && sub_path == "/audit/logs" {
            let (code, res) = audit::AuditApi::clear_logs(ctx, db, db_dir, client_ip);
            return Some((code, res, "application/json"));
        }

        // 9. Linux Desktop Environment (OS Integration) routes
        if method == "GET" && sub_path == "/os/status" {
            let (code, res) = os::OsApi::status();
            return Some((code, res, "application/json"));
        }
        if method == "GET" && sub_path == "/os/apps" {
            let (code, res) = os::OsApi::list_apps();
            return Some((code, res, "application/json"));
        }
        if method == "POST" && sub_path == "/os/launch" {
            let (code, res) = os::OsApi::launch_app(body);
            return Some((code, res, "application/json"));
        }
        if method == "POST" && sub_path == "/os/power" {
            let (code, res) = os::OsApi::power_action(body);
            return Some((code, res, "application/json"));
        }

        Some((404, r#"{"error":"Endpoint API tidak ditemukan"}"#.to_string(), "application/json"))
    }
}

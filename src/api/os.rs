use crate::api::json_helper::{escape_json, parse_json};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};

pub struct OsApi;

#[derive(Clone)]
struct DesktopApp {
    id: String,
    name: String,
    exec: String,
    comment: String,
    category: String,
    icon_emoji: String,
}

impl OsApi {
    /// GET /api/os/status — Linux DE telemetry (hostname, distro, battery, wifi, memory, de_mode)
    pub fn status() -> (u16, String) {
        let hostname = fs::read_to_string("/etc/hostname")
            .unwrap_or_else(|_| "BijiOS".to_string())
            .trim()
            .to_string();

        let mut distro = "Linux".to_string();
        if let Ok(os_rel) = fs::read_to_string("/etc/os-release") {
            for line in os_rel.lines() {
                if let Some(val) = line.strip_prefix("PRETTY_NAME=") {
                    distro = val.trim_matches('"').to_string();
                    break;
                }
            }
        }

        // Battery info (if laptop)
        let mut battery_pct: i32 = 100;
        let mut battery_status = "AC Power".to_string();
        let mut has_battery = false;
        if let Ok(entries) = fs::read_dir("/sys/class/power_supply") {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with("BAT") {
                    let p = entry.path();
                    if let Ok(cap) = fs::read_to_string(p.join("capacity")) {
                        if let Ok(v) = cap.trim().parse::<i32>() {
                            battery_pct = v;
                            has_battery = true;
                        }
                    }
                    if let Ok(st) = fs::read_to_string(p.join("status")) {
                        battery_status = st.trim().to_string();
                    }
                    break;
                }
            }
        }

        // Active Wi-Fi SSID (via nmcli if installed)
        let mut wifi_ssid = String::new();
        if let Ok(out) = Command::new("nmcli")
            .args(["-t", "-f", "ACTIVE,SSID", "dev", "wifi"])
            .output()
        {
            if out.status.success() {
                let text = String::from_utf8_lossy(&out.stdout);
                for line in text.lines() {
                    if let Some(ssid) = line.strip_prefix("yes:") {
                        wifi_ssid = ssid.trim().to_string();
                        break;
                    }
                }
            }
        }

        let de_mode = std::env::var("BIJIOS_DE_MODE").unwrap_or_default() == "1";

        let json = format!(
            r#"{{"success":true,"hostname":"{}","distro":"{}","has_battery":{},"battery_percent":{},"battery_status":"{}","wifi_ssid":"{}","de_mode":{}}}"#,
            escape_json(&hostname),
            escape_json(&distro),
            has_battery,
            battery_pct,
            escape_json(&battery_status),
            escape_json(&wifi_ssid),
            de_mode
        );
        (200, json)
    }

    fn scan_desktop_apps() -> Vec<DesktopApp> {
        let mut dirs: Vec<PathBuf> = vec![
            PathBuf::from("/usr/share/applications"),
            PathBuf::from("/usr/local/share/applications"),
        ];
        if let Ok(home) = std::env::var("HOME") {
            dirs.push(PathBuf::from(home).join(".local/share/applications"));
        }

        let mut map: HashMap<String, DesktopApp> = HashMap::new();

        for dir in dirs {
            let Ok(entries) = fs::read_dir(&dir) else { continue };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("desktop") {
                    continue;
                }
                let Ok(content) = fs::read_to_string(&path) else { continue };

                let mut in_main = false;
                let mut app_type = String::new();
                let mut name = String::new();
                let mut exec = String::new();
                let mut comment = String::new();
                let mut categories = String::new();
                let mut no_display = false;
                let mut hidden = false;

                for raw_line in content.lines() {
                    let line = raw_line.trim();
                    if line.starts_with('[') && line.ends_with(']') {
                        in_main = line == "[Desktop Entry]";
                        continue;
                    }
                    if !in_main || line.is_empty() || line.starts_with('#') {
                        continue;
                    }
                    if let Some((k, v)) = line.split_once('=') {
                        match k.trim() {
                            "Type" => app_type = v.trim().to_string(),
                            "Name" if name.is_empty() => name = v.trim().to_string(),
                            "Exec" => exec = v.trim().to_string(),
                            "Comment" if comment.is_empty() => comment = v.trim().to_string(),
                            "Categories" => categories = v.trim().to_string(),
                            "NoDisplay" => no_display = v.trim().eq_ignore_ascii_case("true"),
                            "Hidden" => hidden = v.trim().eq_ignore_ascii_case("true"),
                            _ => {}
                        }
                    }
                }

                if app_type != "Application" || no_display || hidden || name.is_empty() || exec.is_empty() {
                    continue;
                }
                let id = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_string();
                if id.is_empty() || id.contains("bijios") {
                    continue;
                }

                let cat_lower = categories.to_lowercase();
                let name_lower = name.to_lowercase();
                let (category, icon_emoji) = if cat_lower.contains("network") || cat_lower.contains("webbrowser") || name_lower.contains("firefox") || name_lower.contains("chrom") {
                    ("Internet", "🌐")
                } else if cat_lower.contains("development") || name_lower.contains("code") || name_lower.contains("terminal") {
                    ("Development", "💻")
                } else if cat_lower.contains("office") {
                    ("Office", "📄")
                } else if cat_lower.contains("graphics") {
                    ("Graphics", "🎨")
                } else if cat_lower.contains("audiovideo") || cat_lower.contains("audio") || cat_lower.contains("video") {
                    ("Media", "🎵")
                } else if cat_lower.contains("system") || cat_lower.contains("settings") {
                    ("System", "🛠️")
                } else if cat_lower.contains("game") {
                    ("Games", "🎮")
                } else {
                    ("Utilities", "📦")
                };

                map.insert(
                    id.clone(),
                    DesktopApp {
                        id,
                        name,
                        exec,
                        comment,
                        category: category.to_string(),
                        icon_emoji: icon_emoji.to_string(),
                    },
                );
            }
        }

        let mut list: Vec<DesktopApp> = map.into_values().collect();
        list.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        list.truncate(80);
        list
    }

    /// GET /api/os/apps — List installed native Linux (.deb) applications
    pub fn list_apps() -> (u16, String) {
        let apps = Self::scan_desktop_apps();
        let items: Vec<String> = apps
            .iter()
            .map(|a| {
                format!(
                    r#"{{"id":"{}","name":"{}","comment":"{}","category":"{}","icon":"{}"}}"#,
                    escape_json(&a.id),
                    escape_json(&a.name),
                    escape_json(&a.comment),
                    escape_json(&a.category),
                    escape_json(&a.icon_emoji)
                )
            })
            .collect();
        (200, format!(r#"{{"success":true,"apps":[{}]}}"#, items.join(",")))
    }

    /// POST /api/os/launch — Launch a verified installed .desktop application
    pub fn launch_app(body: &str) -> (u16, String) {
        let Some(json) = parse_json(body) else {
            return (400, r#"{"error":"Invalid JSON"}"#.to_string());
        };
        let app_id = json.get_str("id").unwrap_or("").trim();
        if app_id.is_empty() {
            return (400, r#"{"error":"App id is required"}"#.to_string());
        }

        let apps = Self::scan_desktop_apps();
        let Some(app) = apps.into_iter().find(|a| a.id == app_id) else {
            return (404, r#"{"error":"Application not found"}"#.to_string());
        };

        // Strip Freedesktop field codes (%u, %U, %f, %F, %i, %c, %k)
        let tokens: Vec<String> = app
            .exec
            .split_whitespace()
            .filter(|t| !t.starts_with('%'))
            .map(|t| t.trim_matches('"').to_string())
            .collect();

        if tokens.is_empty() {
            return (400, r#"{"error":"Invalid Exec command"}"#.to_string());
        }

        let program = &tokens[0];
        let args = &tokens[1..];

        let mut cmd = if std::path::Path::new("/usr/bin/bijios-app-launch").exists() {
            let mut c = Command::new("/usr/bin/bijios-app-launch");
            c.arg(program).args(args);
            c
        } else {
            let mut c = Command::new(program);
            c.args(args);
            c
        };

        match cmd
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(_) => (
                200,
                format!(
                    r#"{{"success":true,"launched":"{}"}}"#,
                    escape_json(&app.name)
                ),
            ),
            Err(e) => (
                500,
                format!(
                    r#"{{"error":"Gagal menjalankan {}: {}"}}"#,
                    escape_json(&app.name),
                    escape_json(&e.to_string())
                ),
            ),
        }
    }

    /// POST /api/os/power — Power management (suspend, reboot, poweroff, logout)
    pub fn power_action(body: &str) -> (u16, String) {
        let Some(json) = parse_json(body) else {
            return (400, r#"{"error":"Invalid JSON"}"#.to_string());
        };
        let action = json.get_str("action").unwrap_or("").trim().to_lowercase();

        // Logout is handled in-place by BijiOS Desktop Tahoe Login screen (avoid exiting X11 to LightDM)
        if action == "logout" {
            return (200, r#"{"success":true,"action":"logout","handled_by":"bijios-desktop"}"#.to_string());
        }

        let (cmd, args): (&str, Vec<&str>) = match action.as_str() {
            "suspend" | "sleep" => ("systemctl", vec!["suspend"]),
            "reboot" | "restart" => ("systemctl", vec!["reboot"]),
            "poweroff" | "shutdown" => ("systemctl", vec!["poweroff"]),
            "logout" => ("openbox", vec!["--exit"]),
            _ => return (400, r#"{"error":"Unknown power action"}"#.to_string()),
        };

        match Command::new(cmd)
            .args(&args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(_) => (200, format!(r#"{{"success":true,"action":"{}"}}"#, escape_json(&action))),
            Err(e) => (
                500,
                format!(r#"{{"error":"Power action failed: {}"}}"#, escape_json(&e.to_string())),
            ),
        }
    }
}

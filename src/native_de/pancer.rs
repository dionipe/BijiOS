//! Pancer — BijiOS Native Interactive Multi-Tab & Split-Pane PTY/TTY Terminal
//! Powered by Pure Rust + GTK4 + VTE4 (`libvte-2.91-gtk4`)
//! Features:
//! - Menu Bar: File (New Window, New Tab), Edit (Copy, Paste, Clear),
//!   Shell (Duplicate Window, Duplicate Tab, Split Vertical, Split Horizontal, Close Tab)
//! - Dynamic Host Tab Bar: | host 1 | host 2 | + | (auto-detects local host vs active SSH host)

use crate::native_de::apps::{create_app_window, home_dir, open_finder_at};
use crate::native_de::gtk4_ffi::*;
use std::cell::{Cell, RefCell};
use std::ffi::{c_int, CStr};
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::rc::Rc;

// ============================================================================
// Dynamic Host & CWD Detection via /proc + VTE OSC Title
// ============================================================================

fn local_hostname() -> String {
    fs::read_to_string("/etc/hostname")
        .unwrap_or_else(|_| "BijiOS".into())
        .trim()
        .to_string()
}

fn read_pid_cwd(pid: i32) -> PathBuf {
    if pid > 0 {
        if let Ok(p) = fs::read_link(format!("/proc/{}/cwd", pid)) {
            if p.is_dir() {
                return p;
            }
        }
    }
    home_dir()
}

/// Find an active `ssh` child process under `shell_pid` and parse its target host
fn find_ssh_host_for_shell(shell_pid: i32) -> Option<String> {
    if shell_pid <= 0 {
        return None;
    }
    let Ok(rd) = fs::read_dir("/proc") else {
        return None;
    };
    let mut children_of: Vec<(i32, i32, String)> = Vec::new();
    for ent in rd.flatten() {
        let Ok(pid) = ent.file_name().to_string_lossy().parse::<i32>() else {
            continue;
        };
        let Ok(stat) = fs::read_to_string(ent.path().join("stat")) else {
            continue;
        };
        // Format: pid (comm) state ppid ...
        let Some(rparen) = stat.rfind(')') else {
            continue;
        };
        let lparen = stat.find('(').unwrap_or(0);
        let comm = stat[lparen + 1..rparen].to_string();
        let after = stat[rparen + 1..].trim();
        let ppid = after
            .split_whitespace()
            .nth(1)
            .and_then(|s| s.parse::<i32>().ok())
            .unwrap_or(0);
        children_of.push((pid, ppid, comm));
    }

    // BFS from shell_pid to find any descendant `ssh` process
    let mut queue = vec![shell_pid];
    let mut visited = vec![shell_pid];
    while let Some(curr) = queue.pop() {
        for (pid, ppid, comm) in &children_of {
            if *ppid == curr && !visited.contains(pid) {
                visited.push(*pid);
                if comm == "ssh" {
                    if let Ok(raw) = fs::read(format!("/proc/{}/cmdline", pid)) {
                        let args: Vec<String> = raw
                            .split(|&b| b == 0)
                            .filter(|s| !s.is_empty())
                            .map(|s| String::from_utf8_lossy(s).into_owned())
                            .collect();
                        if let Some(host) = parse_ssh_destination(&args) {
                            return Some(host);
                        }
                    }
                    return Some("ssh".to_string());
                }
                queue.push(*pid);
            }
        }
    }
    None
}

fn parse_ssh_destination(args: &[String]) -> Option<String> {
    // Options that take an argument in OpenSSH
    const WITH_ARG: &[&str] = &[
        "-b", "-c", "-D", "-E", "-e", "-F", "-I", "-i", "-J", "-L", "-l", "-m", "-O", "-o",
        "-p", "-Q", "-R", "-S", "-W", "-w",
    ];
    let mut login_user: Option<String> = None;
    let mut i = 1; // skip "ssh"
    while i < args.len() {
        let a = &args[i];
        if a == "--" {
            if i + 1 < args.len() {
                let dest = &args[i + 1];
                return Some(match login_user {
                    Some(u) if !dest.contains('@') => format!("{}@{}", u, dest),
                    _ => dest.clone(),
                });
            }
            break;
        }
        if a.starts_with('-') {
            if a == "-l" && i + 1 < args.len() {
                login_user = Some(args[i + 1].clone());
                i += 2;
                continue;
            }
            if WITH_ARG.contains(&a.as_str()) {
                i += 2;
                continue;
            }
            i += 1;
            continue;
        }
        return Some(match login_user {
            Some(u) if !a.contains('@') => format!("{}@{}", u, a),
            _ => a.clone(),
        });
    }
    None
}

unsafe fn detect_pane_host(pid: i32, vte: *mut GtkWidget, local_host: &str) -> String {
    if let Some(ssh_host) = find_ssh_host_for_shell(pid) {
        return format!("🔐 {}", ssh_host);
    }
    // Check OSC window title set by remote shell (e.g. "user@remotehost: ~")
    let title_ptr = vte_terminal_get_window_title(vte);
    if !title_ptr.is_null() {
        let t = CStr::from_ptr(title_ptr).to_string_lossy();
        if let Some(at_pos) = t.find('@') {
            let after_at = &t[at_pos + 1..];
            let host_part = after_at
                .split([':', ' ', '/'])
                .next()
                .unwrap_or("")
                .trim();
            if !host_part.is_empty() && !host_part.eq_ignore_ascii_case(local_host) {
                return format!("🔐 {}", host_part);
            }
        }
    }
    format!("💻 {}", local_host)
}

// ============================================================================
// VTE Pane Creation & Spawn Callback
// ============================================================================

#[derive(Clone)]
struct VtePane {
    vte: *mut GtkWidget,
    pid: Rc<Cell<i32>>,
}

extern "C" fn on_vte_spawned(
    _vte: GPointer,
    pid: c_int,
    _err: GPointer,
    user_data: GPointer,
) {
    if !user_data.is_null() {
        unsafe {
            let cell = Rc::from_raw(user_data as *const Cell<i32>);
            if pid > 0 {
                cell.set(pid);
            }
        }
    }
}

unsafe fn create_vte_pane(work_dir: &PathBuf) -> VtePane {
    let vte = vte_terminal_new();
    gtk_widget_set_hexpand(vte, 1);
    gtk_widget_set_vexpand(vte, 1);
    vte_terminal_set_scrollback_lines(vte, 10000);

    let font_str = cstr("DejaVu Sans Mono 11");
    let font_desc = pango_font_description_from_string(font_str.as_ptr());
    if !font_desc.is_null() {
        vte_terminal_set_font(vte, font_desc);
        pango_font_description_free(font_desc);
    }

    // BijiOS Official Brand Color Palette (Hutan Malam bg, Krem Tanah fg, Emas Benih cursor)
    let fg = GdkRGBA::hex(0xF4, 0xEB, 0xD9);
    let bg = GdkRGBA::hex(0x09, 0x18, 0x14);
    let cursor = GdkRGBA::hex(0xE8, 0xA3, 0x3D);
    let palette: [GdkRGBA; 16] = [
        GdkRGBA::hex(0x12, 0x33, 0x2A),
        GdkRGBA::hex(0xE0, 0x5A, 0x4F),
        GdkRGBA::hex(0x2F, 0x7D, 0x4F),
        GdkRGBA::hex(0xE8, 0xA3, 0x3D),
        GdkRGBA::hex(0x4E, 0x9A, 0xF1),
        GdkRGBA::hex(0xB4, 0x8E, 0xAD),
        GdkRGBA::hex(0x4F, 0xB8, 0xA6),
        GdkRGBA::hex(0xE5, 0xDC, 0xC8),
        GdkRGBA::hex(0x3B, 0x5C, 0x52),
        GdkRGBA::hex(0xF8, 0x71, 0x71),
        GdkRGBA::hex(0x7F, 0xBF, 0x6A),
        GdkRGBA::hex(0xF6, 0xC4, 0x69),
        GdkRGBA::hex(0x7D, 0xD3, 0xFC),
        GdkRGBA::hex(0xD8, 0xB4, 0xFE),
        GdkRGBA::hex(0x5E, 0xEA, 0xD4),
        GdkRGBA::hex(0xFF, 0xFD, 0xF7),
    ];
    vte_terminal_set_colors(vte, &fg, &bg, palette.as_ptr(), palette.len());
    vte_terminal_set_color_cursor(vte, &cursor);

    let pid_cell = Rc::new(Cell::new(0i32));
    let pid_raw = Rc::into_raw(Rc::clone(&pid_cell)) as GPointer;

    let wd = work_dir.to_string_lossy().to_string();
    let c_wd = cstr(&wd);
    let c_shell = cstr("/bin/bash");
    let rc_candidates = [
        "/usr/share/bijios-desktop/pancer-bashrc",
        "/home/dionipe/apps/bijios-desktop/packaging/deb/usr/share/bijios-desktop/pancer-bashrc",
    ];
    let rc_path = rc_candidates
        .iter()
        .find(|p| std::path::Path::new(p).is_file())
        .copied();
    let c_rcflag = cstr("--rcfile");
    let c_rcpath = cstr(rc_path.unwrap_or(""));
    let argv_rc = [c_shell.as_ptr(), c_rcflag.as_ptr(), c_rcpath.as_ptr(), std::ptr::null()];
    let argv_plain = [c_shell.as_ptr(), std::ptr::null()];
    let argv_ptr = if rc_path.is_some() {
        argv_rc.as_ptr()
    } else {
        argv_plain.as_ptr()
    };
    vte_terminal_spawn_async(
        vte,
        0,
        c_wd.as_ptr(),
        argv_ptr,
        std::ptr::null(),
        0,
        std::ptr::null_mut(),
        std::ptr::null_mut(),
        std::ptr::null_mut(),
        -1,
        std::ptr::null_mut(),
        on_vte_spawned as GPointer,
        pid_raw,
    );

    VtePane { vte, pid: pid_cell }
}

// ============================================================================
// Pancer Window State (Multi-Tab + Split Panes + Dynamic Host Bar)
// ============================================================================

struct PancerTab {
    id: u32,
    stack_name: String,
    container: *mut GtkWidget,
    panes: Vec<VtePane>,
    tab_pill: *mut GtkWidget,
    tab_btn: *mut GtkWidget,
}

struct PancerState {
    next_id: u32,
    active_tab_id: u32,
    tabs: Vec<PancerTab>,
}

impl PancerState {
    fn active_tab_mut(&mut self) -> Option<&mut PancerTab> {
        let aid = self.active_tab_id;
        self.tabs.iter_mut().find(|t| t.id == aid)
    }

    fn active_tab(&self) -> Option<&PancerTab> {
        let aid = self.active_tab_id;
        self.tabs.iter().find(|t| t.id == aid)
    }

    unsafe fn active_pane(&self) -> Option<VtePane> {
        let tab = self.active_tab()?;
        for p in &tab.panes {
            if gtk_widget_has_focus(p.vte) != 0 {
                return Some(p.clone());
            }
        }
        tab.panes.last().cloned()
    }
}

pub fn open_pancer(app: *mut GtkApplication) {
    open_pancer_with_dir(app, None);
}

// ============================================================================
// ~/.ssh/config Host Manager (Read, Add, Edit, Delete) + Sidebar Persistence
// ============================================================================
#[derive(Clone, Default)]
struct SshHost {
    alias: String,
    hostname: String,
    user: String,
    port: String,
    identity_file: String,
}

fn ssh_config_path() -> PathBuf {
    home_dir().join(".ssh").join("config")
}

fn pancer_conf_path() -> PathBuf {
    home_dir()
        .join(".config")
        .join("bijios-desktop")
        .join("pancer.conf")
}

fn load_sidebar_visible() -> bool {
    if let Ok(txt) = fs::read_to_string(pancer_conf_path()) {
        for line in txt.lines() {
            if let Some((k, v)) = line.split_once('=') {
                if k.trim() == "sidebar_visible" {
                    return v.trim() != "false";
                }
            }
        }
    }
    true
}

fn save_sidebar_visible(visible: bool) {
    let p = pancer_conf_path();
    if let Some(parent) = p.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(p, format!("sidebar_visible={}\n", visible));
}

fn read_ssh_hosts() -> Vec<SshHost> {
    let mut hosts = Vec::new();
    let Ok(txt) = fs::read_to_string(ssh_config_path()) else {
        return hosts;
    };
    let mut cur: Option<SshHost> = None;
    for raw in txt.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.split_whitespace();
        let Some(key) = parts.next() else { continue };
        let val = parts.collect::<Vec<_>>().join(" ");
        if key.eq_ignore_ascii_case("Host") {
            if let Some(h) = cur.take() {
                hosts.push(h);
            }
            if !val.is_empty() && !val.contains('*') && !val.contains('?') {
                cur = Some(SshHost {
                    alias: val,
                    port: "22".to_string(),
                    ..Default::default()
                });
            }
        } else if let Some(ref mut h) = cur {
            if key.eq_ignore_ascii_case("HostName") {
                h.hostname = val;
            } else if key.eq_ignore_ascii_case("User") {
                h.user = val;
            } else if key.eq_ignore_ascii_case("Port") {
                h.port = val;
            } else if key.eq_ignore_ascii_case("IdentityFile") {
                h.identity_file = val;
            }
        }
    }
    if let Some(h) = cur {
        hosts.push(h);
    }
    hosts
}

fn format_ssh_host_block(h: &SshHost) -> String {
    let mut out = format!("Host {}\n", h.alias.trim());
    if !h.hostname.trim().is_empty() {
        out.push_str(&format!("    HostName {}\n", h.hostname.trim()));
    }
    if !h.user.trim().is_empty() {
        out.push_str(&format!("    User {}\n", h.user.trim()));
    }
    let port = h.port.trim();
    if !port.is_empty() && port != "22" {
        out.push_str(&format!("    Port {}\n", port));
    }
    if !h.identity_file.trim().is_empty() {
        out.push_str(&format!("    IdentityFile {}\n", h.identity_file.trim()));
    }
    out
}

/// Rewrite ~/.ssh/config replacing or deleting `target_alias` while preserving all other blocks/comments
fn update_ssh_config(target_alias: Option<&str>, replacement: Option<&SshHost>) -> bool {
    let path = ssh_config_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(parent, fs::Permissions::from_mode(0o700));
        }
    }
    let existing = fs::read_to_string(&path).unwrap_or_default();
    let mut out = String::new();
    let mut skipping = false;
    let mut replaced = false;

    for raw in existing.lines() {
        let trimmed = raw.trim();
        let mut parts = trimmed.split_whitespace();
        if let Some(k) = parts.next() {
            if k.eq_ignore_ascii_case("Host") {
                let alias_val = parts.collect::<Vec<_>>().join(" ");
                if target_alias.map(|t| t == alias_val).unwrap_or(false) {
                    skipping = true;
                    if let Some(new_h) = replacement {
                        out.push_str(&format_ssh_host_block(new_h));
                        replaced = true;
                    }
                    continue;
                } else {
                    skipping = false;
                }
            }
        }
        if !skipping {
            out.push_str(raw);
            out.push('\n');
        }
    }

    if let Some(new_h) = replacement {
        if !replaced {
            if !out.is_empty() && !out.ends_with("\n\n") {
                out.push('\n');
            }
            out.push_str(&format_ssh_host_block(new_h));
        }
    }

    if fs::write(&path, out).is_ok() {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o600));
        }
        true
    } else {
        false
    }
}

unsafe fn open_ssh_host_dialog(
    app: *mut GtkApplication,
    parent_win: *mut GtkWidget,
    editing: Option<SshHost>,
    on_saved: Rc<RefCell<Box<dyn Fn()>>>,
) {
    let is_edit = editing.is_some();
    let title = if is_edit {
        "✏️ Edit SSH Host (~/.ssh/config)"
    } else {
        "➕ Add SSH Host (~/.ssh/config)"
    };
    let dlg = create_app_window(app, title, 450, 360);
    gtk_window_set_resizable(dlg, 0);
    gtk_window_set_modal(dlg, 1);
    gtk_window_set_transient_for(dlg, parent_win);

    let root = new_box(GTK_ORIENTATION_VERTICAL, 10, &[]);
    set_margin_all(root, 18);

    let hdr = new_label(title, &["app-heading"]);
    gtk_label_set_xalign(hdr, 0.0);
    gtk_box_append(root, hdr);

    let init = editing.clone().unwrap_or(SshHost {
        port: "22".to_string(),
        ..Default::default()
    });

    let mk = |lbl: &str, val: &str, ph_txt: &str| -> (*mut GtkWidget, *mut GtkWidget) {
        let r = new_box(GTK_ORIENTATION_HORIZONTAL, 8, &[]);
        let l = new_label(lbl, &["kv-key"]);
        gtk_label_set_xalign(l, 0.0);
        gtk_widget_set_size_request(l, 118, -1);
        let e = gtk_entry_new();
        gtk_widget_set_hexpand(e, 1);
        let ph = cstr(ph_txt);
        gtk_entry_set_placeholder_text(e, ph.as_ptr());
        if !val.is_empty() {
            set_entry_text(e, val);
        }
        gtk_box_append(r, l);
        gtk_box_append(r, e);
        (r, e)
    };

    let (r_alias, e_alias) = mk("Host Alias", &init.alias, "e.g. prod-web");
    let (r_host, e_host) = mk("HostName / IP", &init.hostname, "e.g. 192.168.1.50 or host.id");
    let (r_user, e_user) = mk("Username", &init.user, "e.g. root or ubuntu");
    let (r_port, e_port) = mk("Port", &init.port, "22");
    let (r_key, e_key) = mk("IdentityFile", &init.identity_file, "optional: ~/.ssh/id_ed25519");

    let card = new_box(GTK_ORIENTATION_VERTICAL, 8, &["app-card"]);
    gtk_box_append(card, r_alias);
    gtk_box_append(card, r_host);
    gtk_box_append(card, r_user);
    gtk_box_append(card, r_port);
    gtk_box_append(card, r_key);
    gtk_box_append(root, card);

    let err_lbl = new_label("", &["finder-meta-lbl"]);
    gtk_label_set_xalign(err_lbl, 0.0);
    gtk_widget_set_visible(err_lbl, 0);
    gtk_box_append(root, err_lbl);

    let btns = new_box(GTK_ORIENTATION_HORIZONTAL, 8, &[]);
    gtk_widget_set_halign(btns, GTK_ALIGN_END);
    let btn_cancel = new_button("Cancel", &["toolbar-btn"]);
    let btn_save = new_button(if is_edit { "💾 Save Host" } else { "➕ Add Host" }, &["primary-btn"]);
    gtk_box_append(btns, btn_cancel);
    gtk_box_append(btns, btn_save);
    gtk_box_append(root, btns);

    connect_signal(btn_cancel, "clicked", move || gtk_window_close(dlg));

    let old_alias = editing.map(|h| h.alias);
    connect_signal(btn_save, "clicked", move || {
        let alias = get_entry_text(e_alias).trim().to_string();
        let hostname = get_entry_text(e_host).trim().to_string();
        if alias.is_empty() || alias.contains(char::is_whitespace) {
            set_label_text(err_lbl, "❌ Host Alias is required (no spaces).");
            gtk_widget_set_visible(err_lbl, 1);
            return;
        }
        if hostname.is_empty() {
            set_label_text(err_lbl, "❌ HostName / IP is required.");
            gtk_widget_set_visible(err_lbl, 1);
            return;
        }
        let h = SshHost {
            alias: alias.clone(),
            hostname,
            user: get_entry_text(e_user).trim().to_string(),
            port: {
                let p = get_entry_text(e_port).trim().to_string();
                if p.is_empty() { "22".to_string() } else { p }
            },
            identity_file: get_entry_text(e_key).trim().to_string(),
        };
        let target = old_alias.as_deref().or(Some(alias.as_str()));
        if update_ssh_config(target, Some(&h)) {
            (on_saved.borrow())();
            gtk_window_close(dlg);
        } else {
            set_label_text(err_lbl, "❌ Failed to write ~/.ssh/config.");
            gtk_widget_set_visible(err_lbl, 1);
        }
    });

    gtk_window_set_child(dlg, root);
    gtk_window_present(dlg);
    gtk_widget_grab_focus(e_alias);
}

// ============================================================================
// About Pancer Dialog (Structured Key-Value + Hotkeys + Open-Source Credits)
// ============================================================================
unsafe fn open_about_pancer(app: *mut GtkApplication, parent_win: *mut GtkWidget) {
    let dlg = create_app_window(app, "💻 About Pancer", 580, 560);
    gtk_window_set_resizable(dlg, 0);
    gtk_window_set_modal(dlg, 1);
    gtk_window_set_transient_for(dlg, parent_win);

    let scroll = gtk_scrolled_window_new();
    gtk_scrolled_window_set_policy(scroll, GTK_POLICY_NEVER, GTK_POLICY_AUTOMATIC);
    let root = new_box(GTK_ORIENTATION_VERTICAL, 12, &[]);
    set_margin_all(root, 20);

    // Header Card
    let hdr_card = new_box(GTK_ORIENTATION_HORIZONTAL, 16, &["app-card"]);
    let logo = new_logo_image(58, true);
    gtk_widget_set_valign(logo, GTK_ALIGN_CENTER);
    let tcol = new_box(GTK_ORIENTATION_VERTICAL, 3, &[]);
    gtk_widget_set_hexpand(tcol, 1);
    gtk_widget_set_valign(tcol, GTK_ALIGN_CENTER);
    let title = new_label("Pancer — BijiOS Native Terminal", &["app-heading"]);
    gtk_label_set_xalign(title, 0.0);
    let sub = new_label(
        "Version 1.0.0 (BijiOS \"Padi\")  •  Pure Rust + GTK4 + VTE4",
        &["app-muted"],
    );
    gtk_label_set_xalign(sub, 0.0);
    gtk_box_append(tcol, title);
    gtk_box_append(tcol, sub);
    gtk_box_append(hdr_card, logo);
    gtk_box_append(hdr_card, tcol);
    gtk_box_append(root, hdr_card);

    // Application Details Card (2-column Key / Value)
    let det_card = new_box(GTK_ORIENTATION_VERTICAL, 6, &["app-card"]);
    let d_hdr = new_label("💻 APPLICATION DETAILS", &["app-section-title"]);
    gtk_label_set_xalign(d_hdr, 0.0);
    gtk_box_append(det_card, d_hdr);

    let kv: [(&str, &str); 5] = [
        ("Application", "Pancer — Multi-Tab & Split-Pane SSH Terminal"),
        ("Version", "1.0.0 (Codename: Padi)"),
        ("Language", "100% Pure Rust (Zero External Crates)"),
        ("Terminal Engine", "Native GTK4 + libvte-2.91-gtk4 (PTY/TTY)"),
        ("Shell Features", "Dynamic Host Tab Bar, ~/.ssh/config Manager, ble.sh Auto-Suggestions"),
    ];
    for (k, v) in kv {
        let row = new_box(GTK_ORIENTATION_HORIZONTAL, 10, &[]);
        let kl = new_label(k, &["kv-key"]);
        gtk_label_set_xalign(kl, 0.0);
        gtk_widget_set_size_request(kl, 130, -1);
        let vl = new_label(v, &["kv-val"]);
        gtk_label_set_xalign(vl, 0.0);
        gtk_label_set_wrap(vl, 1);
        gtk_widget_set_hexpand(vl, 1);
        gtk_box_append(row, kl);
        gtk_box_append(row, vl);
        gtk_box_append(det_card, row);
    }

    let repo_row = new_box(GTK_ORIENTATION_HORIZONTAL, 6, &[]);
    let rk = new_label("Project Links", &["kv-key"]);
    gtk_label_set_xalign(rk, 0.0);
    gtk_widget_set_size_request(rk, 130, -1);
    gtk_box_append(repo_row, rk);
    gtk_box_append(
        repo_row,
        new_link_button("🐙 github.com/dionipe/BijiOS ↗", "https://github.com/dionipe/BijiOS", &["link-pill"]),
    );
    gtk_box_append(
        repo_row,
        new_link_button("👤 @dionipe ↗", "https://github.com/dionipe", &["link-pill"]),
    );
    gtk_box_append(det_card, repo_row);
    gtk_box_append(root, det_card);

    // Keyboard Shortcuts Card
    let hk_card = new_box(GTK_ORIENTATION_VERTICAL, 6, &["app-card"]);
    let hk_hdr = new_label("⌨️ KEYBOARD SHORTCUTS", &["app-section-title"]);
    gtk_label_set_xalign(hk_hdr, 0.0);
    gtk_box_append(hk_card, hk_hdr);

    let hk_grid = gtk_grid_new();
    gtk_grid_set_row_spacing(hk_grid, 4);
    gtk_grid_set_column_spacing(hk_grid, 16);
    gtk_grid_set_column_homogeneous(hk_grid, 1);

    let shortcuts: [(&str, &str); 12] = [
        ("Ctrl+Shift+N", "New Window"),
        ("Ctrl+Shift+T", "New Tab"),
        ("Ctrl+Shift+C / V", "Copy / Paste"),
        ("Ctrl+Shift+K", "Clear Terminal"),
        ("Ctrl+Shift+U / B", "Duplicate Win / Tab"),
        ("Ctrl+Shift+D / E", "Split Vert / Horiz"),
        ("Ctrl+Shift+S", "Hide / Unhide SSH Sidebar"),
        ("Ctrl+Shift+M", "Broadcast Command Bar"),
        ("Ctrl+Shift+W", "Close Active Tab"),
        ("Esc", "Close Menu / Broadcast"),
        ("→  or  End", "Accept Auto-Suggestion"),
        ("Tab / Shift+Tab", "Auto-Completion Menu"),
    ];
    for (i, (key, desc)) in shortcuts.iter().enumerate() {
        let r = new_box(GTK_ORIENTATION_HORIZONTAL, 6, &[]);
        let kl = new_label(key, &["act-proc-mono"]);
        gtk_label_set_xalign(kl, 0.0);
        gtk_widget_set_size_request(kl, 122, -1);
        let dl = new_label(desc, &["kv-val"]);
        gtk_label_set_xalign(dl, 0.0);
        gtk_box_append(r, kl);
        gtk_box_append(r, dl);
        gtk_grid_attach(hk_grid, r, (i % 2) as i32, (i / 2) as i32, 1, 1);
    }
    gtk_box_append(hk_card, hk_grid);
    gtk_box_append(root, hk_card);

    // Open-Source Credits Card
    let cred_card = new_box(GTK_ORIENTATION_VERTICAL, 6, &["app-card"]);
    let c_hdr = new_label("📜 OPEN-SOURCE COMPONENTS", &["app-section-title"]);
    gtk_label_set_xalign(c_hdr, 0.0);
    gtk_box_append(cred_card, c_hdr);

    let c_pills = new_box(GTK_ORIENTATION_HORIZONTAL, 6, &[]);
    gtk_box_append(
        c_pills,
        new_link_button("GNOME VTE4 (LGPL-3.0) ↗", "https://gitlab.gnome.org/GNOME/vte", &["link-pill"]),
    );
    gtk_box_append(
        c_pills,
        new_link_button("ble.sh (BSD-3-Clause) ↗", "https://github.com/akinomyoga/ble.sh", &["link-pill"]),
    );
    gtk_box_append(
        c_pills,
        new_link_button("bash-completion ↗", "https://github.com/scop/bash-completion", &["link-pill"]),
    );
    gtk_box_append(cred_card, c_pills);
    gtk_box_append(root, cred_card);

    let close_row = new_box(GTK_ORIENTATION_HORIZONTAL, 0, &[]);
    gtk_widget_set_halign(close_row, GTK_ALIGN_END);
    let btn_close = new_button("Close", &["primary-btn"]);
    connect_signal(btn_close, "clicked", move || gtk_window_close(dlg));
    gtk_box_append(close_row, btn_close);
    gtk_box_append(root, close_row);

    gtk_scrolled_window_set_child(scroll, root);
    gtk_window_set_child(dlg, scroll);
    gtk_window_present(dlg);
}

// ============================================================================
// Remote SSH File Explorer (Browse, Download, Upload) & SSHFS Mount in Akar
// ============================================================================
fn sh_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

fn is_mountpoint_active(dir: &PathBuf) -> bool {
    let ds = dir.to_string_lossy().to_string();
    fs::read_to_string("/proc/mounts")
        .map(|txt| txt.lines().any(|l| l.split_whitespace().nth(1) == Some(ds.as_str())))
        .unwrap_or(false)
}

fn mount_ssh_host(alias: &str, remote_dir: &str) -> Result<PathBuf, String> {
    let safe = alias.replace(|c: char| !c.is_alphanumeric() && c != '-' && c != '_' && c != '.', "_");
    let mnt = home_dir().join("Mounts").join(&safe);
    let _ = fs::create_dir_all(&mnt);
    if is_mountpoint_active(&mnt) {
        return Ok(mnt);
    }
    let remote_spec = if remote_dir.is_empty() || remote_dir == "." || remote_dir == "~" {
        format!("{}:", alias)
    } else {
        format!("{}:{}", alias, remote_dir)
    };
    let out = Command::new("sshfs")
        .args([
            "-o",
            "reconnect,ServerAliveInterval=15,BatchMode=yes,ConnectTimeout=8",
            &remote_spec,
            &mnt.to_string_lossy(),
        ])
        .output();
    match out {
        Ok(o) if o.status.success() => Ok(mnt),
        Ok(o) => {
            let err = String::from_utf8_lossy(&o.stderr).trim().to_string();
            Err(if err.is_empty() {
                "SSHFS mount failed (check SSH key / BatchMode access)".to_string()
            } else {
                err
            })
        }
        Err(e) => Err(format!("sshfs error: {}", e)),
    }
}

#[derive(Clone)]
struct RemoteEntry {
    name: String,
    is_dir: bool,
    size: String,
}

fn list_remote_ssh_dir(alias: &str, remote_dir: &str) -> Result<(String, Vec<RemoteEntry>), String> {
    let dir_arg = if remote_dir.trim().is_empty() || remote_dir.trim() == "~" {
        "$HOME".to_string()
    } else {
        sh_quote(remote_dir.trim())
    };
    let remote_cmd = format!(
        "cd {d} 2>/dev/null && pwd && ls -la --group-directories-first 2>/dev/null || (cd {d} && pwd && ls -la)",
        d = dir_arg
    );
    let out = Command::new("ssh")
        .args(["-o", "BatchMode=yes", "-o", "ConnectTimeout=6", alias, &remote_cmd])
        .output()
        .map_err(|e| format!("Failed to run ssh: {}", e))?;

    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(if err.is_empty() {
            "SSH connection failed (configure SSH key for passwordless browsing)".to_string()
        } else {
            err
        });
    }

    let txt = String::from_utf8_lossy(&out.stdout);
    let mut lines = txt.lines();
    let resolved_pwd = lines.next().unwrap_or(remote_dir).trim().to_string();
    let mut entries = Vec::new();
    for line in lines {
        let t = line.trim();
        if t.is_empty() || t.starts_with("total ") {
            continue;
        }
        let parts: Vec<&str> = t.split_whitespace().collect();
        if parts.len() < 9 {
            continue;
        }
        let perms = parts[0];
        let size = parts[4].to_string();
        let name = parts[8..].join(" ");
        let clean_name = name.split(" -> ").next().unwrap_or(&name).to_string();
        if clean_name == "." || clean_name == ".." {
            continue;
        }
        let is_dir = perms.starts_with('d');
        entries.push(RemoteEntry {
            name: clean_name,
            is_dir,
            size,
        });
    }
    entries.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())));
    Ok((resolved_pwd, entries))
}

unsafe fn open_ssh_file_browser(
    app: *mut GtkApplication,
    parent_win: *mut GtkWidget,
    alias: String,
) {
    let win = create_app_window(app, &format!("📂 Remote Files — {}", alias), 780, 560);
    gtk_window_set_transient_for(win, parent_win);

    let root = new_box(GTK_ORIENTATION_VERTICAL, 8, &[]);
    set_margin_all(root, 14);

    // Top Navigation + Mount in Akar Toolbar
    let nav = new_box(GTK_ORIENTATION_HORIZONTAL, 6, &[]);
    let btn_up = new_button("⬆", &["toolbar-btn"]);
    let path_entry = gtk_entry_new();
    gtk_widget_set_hexpand(path_entry, 1);
    set_entry_text(path_entry, "~");
    let btn_go = new_button("🔄 Load", &["toolbar-btn"]);
    let btn_mount = new_button("🔗 Mount in Akar", &["primary-btn"]);
    gtk_box_append(nav, btn_up);
    gtk_box_append(nav, path_entry);
    gtk_box_append(nav, btn_go);
    gtk_box_append(nav, btn_mount);
    gtk_box_append(root, nav);

    // Upload Bar
    let up_row = new_box(GTK_ORIENTATION_HORIZONTAL, 6, &["app-card"]);
    let up_lbl = new_label("⬆ Upload Local File:", &["kv-key"]);
    let up_entry = gtk_entry_new();
    gtk_widget_set_hexpand(up_entry, 1);
    let ph_up = cstr("/home/dionipe/Documents/file.txt");
    gtk_entry_set_placeholder_text(up_entry, ph_up.as_ptr());
    let btn_upload = new_button("⬆ Upload", &["primary-btn"]);
    gtk_box_append(up_row, up_lbl);
    gtk_box_append(up_row, up_entry);
    gtk_box_append(up_row, btn_upload);
    gtk_box_append(root, up_row);

    let status_lbl = new_label(&format!("Connecting to '{}'...", alias), &["app-muted"]);
    gtk_label_set_xalign(status_lbl, 0.0);
    gtk_label_set_wrap(status_lbl, 1);
    gtk_box_append(root, status_lbl);

    // Remote Files List
    let scroller = gtk_scrolled_window_new();
    gtk_scrolled_window_set_policy(scroller, GTK_POLICY_NEVER, GTK_POLICY_AUTOMATIC);
    gtk_widget_set_hexpand(scroller, 1);
    gtk_widget_set_vexpand(scroller, 1);
    let list_box = new_box(GTK_ORIENTATION_VERTICAL, 3, &[]);
    gtk_scrolled_window_set_child(scroller, list_box);
    gtk_box_append(root, scroller);

    let cur_remote_dir = Rc::new(RefCell::new("~".to_string()));
    let populate: Rc<RefCell<Box<dyn Fn()>>> = Rc::new(RefCell::new(Box::new(|| {})));
    let pop_c = Rc::clone(&populate);

    {
        let al = alias.clone();
        let crd = Rc::clone(&cur_remote_dir);
        *populate.borrow_mut() = Box::new(move || {
            clear_box_children(list_box);
            let target = crd.borrow().clone();
            match list_remote_ssh_dir(&al, &target) {
                Ok((pwd, entries)) => {
                    *crd.borrow_mut() = pwd.clone();
                    set_entry_text(path_entry, &pwd);
                    set_label_text(
                        status_lbl,
                        &format!("✅ Connected to {} — {} item(s) in {}", al, entries.len(), pwd),
                    );
                    if entries.is_empty() {
                        let em = new_label("Empty remote directory.", &["finder-meta-lbl"]);
                        set_margin_all(em, 16);
                        gtk_box_append(list_box, em);
                        return;
                    }
                    for (idx, ent) in entries.into_iter().enumerate() {
                        let cls = if idx % 2 == 0 { "act-proc-row" } else { "act-proc-row-alt" };
                        let row = new_box(GTK_ORIENTATION_HORIZONTAL, 8, &[cls]);
                        let ic = if ent.is_dir { "📁" } else { "📄" };
                        let name_btn = new_left_button(ic, &ent.name, &["sidebar-btn"]);
                        gtk_widget_set_hexpand(name_btn, 1);
                        let sz_lbl = new_label(if ent.is_dir { "Folder" } else { &ent.size }, &["act-proc-mono"]);
                        gtk_widget_set_size_request(sz_lbl, 80, -1);

                        gtk_box_append(row, name_btn);
                        gtk_box_append(row, sz_lbl);

                        let full_remote = if pwd == "/" {
                            format!("/{}", ent.name)
                        } else {
                            format!("{}/{}", pwd.trim_end_matches('/'), ent.name)
                        };

                        if ent.is_dir {
                            let fr = full_remote.clone();
                            let crd2 = Rc::clone(&crd);
                            let pc2 = Rc::clone(&pop_c);
                            connect_signal(name_btn, "clicked", move || {
                                *crd2.borrow_mut() = fr.clone();
                                (pc2.borrow())();
                            });
                        } else {
                            let btn_dl = new_button("⬇ Download", &["toolbar-btn"]);
                            let al_dl = al.clone();
                            let fr_dl = full_remote.clone();
                            let fn_dl = ent.name.clone();
                            connect_signal(btn_dl, "clicked", move || {
                                let dl_dir = home_dir().join("Downloads");
                                let _ = fs::create_dir_all(&dl_dir);
                                let dest = dl_dir.join(&fn_dl);
                                let src = format!("{}:{}", al_dl, fr_dl);
                                let ok = Command::new("scp")
                                    .args(["-o", "BatchMode=yes", "-o", "ConnectTimeout=8", &src, &dest.to_string_lossy()])
                                    .status()
                                    .map(|s| s.success())
                                    .unwrap_or(false);
                                set_label_text(
                                    status_lbl,
                                    &if ok {
                                        format!("✅ Downloaded '{}' to ~/Downloads/{}", fn_dl, fn_dl)
                                    } else {
                                        format!("❌ Failed to download '{}' via scp.", fn_dl)
                                    },
                                );
                            });
                            gtk_box_append(row, btn_dl);
                        }
                        gtk_box_append(list_box, row);
                    }
                }
                Err(e) => {
                    set_label_text(status_lbl, &format!("❌ {}", e));
                }
            }
        });
    }

    // Wire Navigation, Upload & Mount in Akar
    {
        let crd = Rc::clone(&cur_remote_dir);
        let pc = Rc::clone(&populate);
        connect_signal(btn_up, "clicked", move || {
            let cur = crd.borrow().clone();
            let parent = std::path::Path::new(&cur)
                .parent()
                .map(|p| p.to_string_lossy().to_string())
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "/".to_string());
            *crd.borrow_mut() = parent;
            (pc.borrow())();
        });
    }
    {
        let crd = Rc::clone(&cur_remote_dir);
        let pc = Rc::clone(&populate);
        connect_signal(btn_go, "clicked", move || {
            *crd.borrow_mut() = get_entry_text(path_entry).trim().to_string();
            (pc.borrow())();
        });
    }
    {
        let crd = Rc::clone(&cur_remote_dir);
        let pc = Rc::clone(&populate);
        connect_signal(path_entry, "activate", move || {
            *crd.borrow_mut() = get_entry_text(path_entry).trim().to_string();
            (pc.borrow())();
        });
    }
    {
        let al = alias.clone();
        let crd = Rc::clone(&cur_remote_dir);
        let pc = Rc::clone(&populate);
        connect_signal(btn_upload, "clicked", move || {
            let local = get_entry_text(up_entry).trim().to_string();
            if local.is_empty() || !std::path::Path::new(&local).is_file() {
                set_label_text(status_lbl, "❌ Local file not found. Enter a valid local file path.");
                return;
            }
            let rdir = crd.borrow().clone();
            let dest = format!("{}:{}/", al, rdir.trim_end_matches('/'));
            let ok = Command::new("scp")
                .args(["-o", "BatchMode=yes", "-o", "ConnectTimeout=8", &local, &dest])
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
            if ok {
                set_entry_text(up_entry, "");
                set_label_text(status_lbl, &format!("✅ Uploaded '{}' to {}", local, dest));
                (pc.borrow())();
            } else {
                set_label_text(status_lbl, "❌ Failed to upload file via scp.");
            }
        });
    }
    {
        let al = alias.clone();
        let crd = Rc::clone(&cur_remote_dir);
        connect_signal(btn_mount, "clicked", move || {
            let rdir = crd.borrow().clone();
            match mount_ssh_host(&al, &rdir) {
                Ok(mnt) => {
                    set_label_text(status_lbl, &format!("✅ Mounted at {} — opening in Akar...", mnt.display()));
                    open_finder_at(app, mnt);
                }
                Err(e) => {
                    set_label_text(status_lbl, &format!("❌ {}", e));
                }
            }
        });
    }

    (populate.borrow())();
    gtk_window_set_child(win, root);
    gtk_window_present(win);
}

pub fn open_pancer_with_dir(app: *mut GtkApplication, initial_dir: Option<PathBuf>) {
    unsafe {
        let win = create_app_window(app, "💻  Pancer", 1040, 580);

        let overlay = gtk_overlay_new();
        let root = new_box(GTK_ORIENTATION_VERTICAL, 0, &[]);
        gtk_widget_set_hexpand(root, 1);
        gtk_widget_set_vexpand(root, 1);
        gtk_overlay_set_child(overlay, root);

        // ---------------------------------------------------------------------
        // 1. Pancer Menu Bar (File | Edit | Shell            [◧ SSH Sidebar])
        // ---------------------------------------------------------------------
        let menubar = new_box(GTK_ORIENTATION_HORIZONTAL, 4, &["pancer-menubar"]);
        let btn_file = new_button("File", &["pancer-menu-btn"]);
        let btn_edit = new_button("Edit", &["pancer-menu-btn"]);
        let btn_shell = new_button("Shell", &["pancer-menu-btn"]);
        let btn_help = new_button("Help", &["pancer-menu-btn"]);
        let mb_spacer = new_box(GTK_ORIENTATION_HORIZONTAL, 0, &[]);
        gtk_widget_set_hexpand(mb_spacer, 1);
        let btn_broadcast = new_button("📡 Broadcast", &["pancer-sidebar-toggle"]);
        let tip_bc = cstr("Toggle Broadcast Command Bar (Ctrl+Shift+M)");
        gtk_widget_set_tooltip_text(btn_broadcast, tip_bc.as_ptr());
        let init_sb_vis = load_sidebar_visible();
        let btn_toggle_sb = new_button(
            if init_sb_vis { "◧ Hide Sidebar" } else { "◧ SSH Hosts" },
            &["pancer-sidebar-toggle"],
        );
        let tip_sb = cstr("Toggle SSH Hosts Sidebar (Ctrl+Shift+S)");
        gtk_widget_set_tooltip_text(btn_toggle_sb, tip_sb.as_ptr());

        gtk_box_append(menubar, btn_file);
        gtk_box_append(menubar, btn_edit);
        gtk_box_append(menubar, btn_shell);
        gtk_box_append(menubar, btn_help);
        gtk_box_append(menubar, mb_spacer);
        gtk_box_append(menubar, btn_broadcast);
        gtk_box_append(menubar, btn_toggle_sb);
        gtk_box_append(root, menubar);

        // Transparent click-outside backdrop placed UNDER the 3 dropdown menus
        let menu_backdrop = gtk_button_new();
        add_css(menu_backdrop, "pancer-menu-backdrop");
        gtk_widget_set_hexpand(menu_backdrop, 1);
        gtk_widget_set_vexpand(menu_backdrop, 1);
        gtk_widget_set_halign(menu_backdrop, GTK_ALIGN_FILL);
        gtk_widget_set_valign(menu_backdrop, GTK_ALIGN_FILL);
        set_margins(menu_backdrop, 26, 0, 0, 0);
        gtk_widget_set_visible(menu_backdrop, 0);
        gtk_overlay_add_overlay(overlay, menu_backdrop);

        // In-Window Dropdown Menus (Zero Black X11 Popups via GtkOverlay)
        let make_dropdown = |margin_start: i32| -> *mut GtkWidget {
            let panel = new_box(GTK_ORIENTATION_VERTICAL, 2, &["pancer-dropdown"]);
            gtk_widget_set_halign(panel, GTK_ALIGN_START);
            gtk_widget_set_valign(panel, GTK_ALIGN_START);
            set_margins(panel, 28, 0, margin_start, 0);
            gtk_widget_set_size_request(panel, 248, -1);
            gtk_widget_set_visible(panel, 0);
            gtk_overlay_add_overlay(overlay, panel);
            panel
        };

        let menu_file = make_dropdown(8);
        let menu_edit = make_dropdown(54);
        let menu_shell = make_dropdown(100);
        let menu_help = make_dropdown(152);

        let any_menu_open = move || -> bool {
            gtk_widget_get_visible(menu_file) != 0
                || gtk_widget_get_visible(menu_edit) != 0
                || gtk_widget_get_visible(menu_shell) != 0
                || gtk_widget_get_visible(menu_help) != 0
        };

        let hide_all_menus = move || {
            gtk_widget_set_visible(menu_backdrop, 0);
            gtk_widget_set_visible(menu_file, 0);
            gtk_widget_set_visible(menu_edit, 0);
            gtk_widget_set_visible(menu_shell, 0);
            gtk_widget_set_visible(menu_help, 0);
        };

        connect_signal(btn_file, "clicked", move || {
            let vis = gtk_widget_get_visible(menu_file) != 0;
            hide_all_menus();
            if !vis {
                gtk_widget_set_visible(menu_backdrop, 1);
                gtk_widget_set_visible(menu_file, 1);
            }
        });
        connect_signal(btn_edit, "clicked", move || {
            let vis = gtk_widget_get_visible(menu_edit) != 0;
            hide_all_menus();
            if !vis {
                gtk_widget_set_visible(menu_backdrop, 1);
                gtk_widget_set_visible(menu_edit, 1);
            }
        });
        connect_signal(btn_shell, "clicked", move || {
            let vis = gtk_widget_get_visible(menu_shell) != 0;
            hide_all_menus();
            if !vis {
                gtk_widget_set_visible(menu_backdrop, 1);
                gtk_widget_set_visible(menu_shell, 1);
            }
        });
        connect_signal(btn_help, "clicked", move || {
            let vis = gtk_widget_get_visible(menu_help) != 0;
            hide_all_menus();
            if !vis {
                gtk_widget_set_visible(menu_backdrop, 1);
                gtk_widget_set_visible(menu_help, 1);
            }
        });

        // ---------------------------------------------------------------------
        // 2. Main Split Body: [ SSH Hosts Sidebar | Terminal Tabs + Stack ]
        // ---------------------------------------------------------------------
        let body_hbox = new_box(GTK_ORIENTATION_HORIZONTAL, 0, &[]);
        gtk_widget_set_hexpand(body_hbox, 1);
        gtk_widget_set_vexpand(body_hbox, 1);
        gtk_box_append(root, body_hbox);

        // Left SSH Hosts Sidebar (~/.ssh/config)
        let ssh_sidebar = new_box(GTK_ORIENTATION_VERTICAL, 8, &["pancer-sidebar"]);
        gtk_widget_set_size_request(ssh_sidebar, 228, -1);
        gtk_widget_set_hexpand(ssh_sidebar, 0);
        gtk_widget_set_vexpand(ssh_sidebar, 1);
        gtk_widget_set_visible(ssh_sidebar, if init_sb_vis { 1 } else { 0 });

        let sb_top = new_box(GTK_ORIENTATION_HORIZONTAL, 6, &[]);
        let sb_hdr = new_label("🔐 SSH HOSTS", &["pancer-sidebar-hdr"]);
        gtk_label_set_xalign(sb_hdr, 0.0);
        gtk_widget_set_hexpand(sb_hdr, 1);
        let btn_ssh_add = new_button("➕ Add", &["pancer-sidebar-toggle"]);
        gtk_box_append(sb_top, sb_hdr);
        gtk_box_append(sb_top, btn_ssh_add);
        gtk_box_append(ssh_sidebar, sb_top);

        let ssh_filter = gtk_entry_new();
        add_css(ssh_filter, "pancer-entry");
        let ph_sf = cstr("🔍 Filter ~/.ssh/config...");
        gtk_entry_set_placeholder_text(ssh_filter, ph_sf.as_ptr());
        gtk_box_append(ssh_sidebar, ssh_filter);

        let ssh_scroll = gtk_scrolled_window_new();
        gtk_scrolled_window_set_policy(ssh_scroll, GTK_POLICY_NEVER, GTK_POLICY_AUTOMATIC);
        gtk_widget_set_vexpand(ssh_scroll, 1);
        let ssh_list_box = new_box(GTK_ORIENTATION_VERTICAL, 6, &[]);
        gtk_scrolled_window_set_child(ssh_scroll, ssh_list_box);
        gtk_box_append(ssh_sidebar, ssh_scroll);

        let ssh_status_lbl = new_label("", &["pancer-ssh-meta"]);
        gtk_label_set_xalign(ssh_status_lbl, 0.0);
        gtk_label_set_wrap(ssh_status_lbl, 1);
        gtk_widget_set_visible(ssh_status_lbl, 0);
        gtk_box_append(ssh_sidebar, ssh_status_lbl);

        gtk_box_append(body_hbox, ssh_sidebar);

        // Right Terminal Area (Broadcast Bar + Dynamic Host Tab Bar + Stack)
        let term_col = new_box(GTK_ORIENTATION_VERTICAL, 0, &[]);
        gtk_widget_set_hexpand(term_col, 1);
        gtk_widget_set_vexpand(term_col, 1);
        gtk_box_append(body_hbox, term_col);

        // Toggleable Broadcast Command Bar (Ctrl+Shift+M)
        let broadcast_bar = new_box(GTK_ORIENTATION_HORIZONTAL, 6, &["pancer-broadcast-bar"]);
        gtk_widget_set_visible(broadcast_bar, 0);
        let bc_badge = new_label("📡 BROADCAST", &["pancer-broadcast-badge"]);
        let bc_all_tabs = Rc::new(Cell::new(true));
        let btn_bc_scope = new_button("All Tabs", &["pancer-sidebar-toggle"]);
        let tip_scope = cstr("Switch between All Tabs & Panes or Current Tab only");
        gtk_widget_set_tooltip_text(btn_bc_scope, tip_scope.as_ptr());
        let bc_entry = gtk_entry_new();
        add_css(bc_entry, "pancer-entry");
        gtk_widget_set_hexpand(bc_entry, 1);
        let ph_bc = cstr("Type command to broadcast to terminals (press Enter)...");
        gtk_entry_set_placeholder_text(bc_entry, ph_bc.as_ptr());
        let btn_bc_send = new_button("▶ Send", &["pancer-sidebar-toggle"]);
        let bc_status = new_label("", &["pancer-ssh-meta"]);
        let btn_bc_close = new_button("✕", &["pancer-ssh-act"]);
        gtk_box_append(broadcast_bar, bc_badge);
        gtk_box_append(broadcast_bar, btn_bc_scope);
        gtk_box_append(broadcast_bar, bc_entry);
        gtk_box_append(broadcast_bar, btn_bc_send);
        gtk_box_append(broadcast_bar, bc_status);
        gtk_box_append(broadcast_bar, btn_bc_close);
        gtk_box_append(term_col, broadcast_bar);

        // Dynamic Host Tab Bar: | 💻 BijiOS | 🔐 host2 | + |
        let tabbar = new_box(GTK_ORIENTATION_HORIZONTAL, 5, &["pancer-tabbar"]);
        let tabs_box = new_box(GTK_ORIENTATION_HORIZONTAL, 4, &[]);
        let btn_add_tab = new_button("+", &["pancer-add-tab"]);
        let tip_add = cstr("New Tab (Ctrl+Shift+T)");
        gtk_widget_set_tooltip_text(btn_add_tab, tip_add.as_ptr());

        gtk_box_append(tabbar, tabs_box);
        gtk_box_append(tabbar, btn_add_tab);
        gtk_box_append(term_col, tabbar);

        // ---------------------------------------------------------------------
        // 3. Terminal Tabs Stack & Lifecycle Guard
        // ---------------------------------------------------------------------
        let stack = gtk_stack_new();
        gtk_widget_set_hexpand(stack, 1);
        gtk_widget_set_vexpand(stack, 1);
        gtk_box_append(term_col, stack);

        let alive = Rc::new(Cell::new(true));
        let state = Rc::new(RefCell::new(PancerState {
            next_id: 1,
            active_tab_id: 0,
            tabs: Vec::new(),
        }));
        let local_host = Rc::new(local_hostname());

        {
            let a = Rc::clone(&alive);
            let st_d = Rc::clone(&state);
            connect_signal(win, "destroy", move || {
                a.set(false);
                if let Ok(mut s) = st_d.try_borrow_mut() {
                    s.tabs.clear();
                }
            });
        }

        // Clicking anywhere outside an open dropdown dismisses it and restores terminal focus
        {
            let st_bg = Rc::clone(&state);
            let a_bg = Rc::clone(&alive);
            connect_signal(menu_backdrop, "clicked", move || {
                hide_all_menus();
                if a_bg.get() {
                    let vte_opt = st_bg
                        .try_borrow()
                        .ok()
                        .and_then(|s| s.active_pane())
                        .map(|p| p.vte);
                    if let Some(vte) = vte_opt {
                        gtk_widget_grab_focus(vte);
                    }
                }
            });
        }

        // Helper: switch active tab
        let select_tab: Rc<dyn Fn(u32)> = {
            let st = Rc::clone(&state);
            let a = Rc::clone(&alive);
            Rc::new(move |tab_id: u32| {
                if !a.get() {
                    return;
                }
                let (target_stack, focus_vte) = {
                    let Ok(mut s) = st.try_borrow_mut() else { return; };
                    s.active_tab_id = tab_id;
                    let mut target = String::new();
                    let mut fv: *mut GtkWidget = std::ptr::null_mut();
                    for t in &s.tabs {
                        if t.id == tab_id {
                            remove_css(t.tab_pill, "pancer-tab");
                            add_css(t.tab_pill, "pancer-tab-active");
                            target = t.stack_name.clone();
                            if let Some(p) = t.panes.first() {
                                fv = p.vte;
                            }
                        } else {
                            remove_css(t.tab_pill, "pancer-tab-active");
                            add_css(t.tab_pill, "pancer-tab");
                        }
                    }
                    (target, fv)
                };
                if !target_stack.is_empty() {
                    let c = cstr(&target_stack);
                    gtk_stack_set_visible_child_name(stack, c.as_ptr());
                }
                if !focus_vte.is_null() {
                    gtk_widget_grab_focus(focus_vte);
                }
            })
        };

        // Helper: close a tab by id safely (drops RefCell borrow BEFORE gtk_stack_remove so
        // synchronous VTE child-exited signals can NEVER cause a re-entrant RefCell panic!)
        let close_tab: Rc<dyn Fn(u32)> = {
            let st = Rc::clone(&state);
            let sel = Rc::clone(&select_tab);
            let a = Rc::clone(&alive);
            Rc::new(move |tab_id: u32| {
                if !a.get() {
                    return;
                }
                let (removed_opt, next_id_opt) = {
                    let Ok(mut s) = st.try_borrow_mut() else { return; };
                    let removed = s
                        .tabs
                        .iter()
                        .position(|t| t.id == tab_id)
                        .map(|idx| s.tabs.remove(idx));
                    let next_id = s.tabs.first().map(|t| t.id);
                    (removed, next_id)
                };
                if let Some(removed) = removed_opt {
                    if gtk_widget_get_parent(removed.tab_pill) == tabs_box {
                        gtk_box_remove(tabs_box, removed.tab_pill);
                    }
                    if gtk_widget_get_parent(removed.container) == stack {
                        gtk_stack_remove(stack, removed.container);
                    }
                }
                if let Some(nid) = next_id_opt {
                    sel(nid);
                } else {
                    a.set(false);
                    gtk_window_close(win);
                }
            })
        };

        // Helper: add a new tab in `dir`
        let add_tab: Rc<dyn Fn(PathBuf)> = {
            let st = Rc::clone(&state);
            let sel = Rc::clone(&select_tab);
            let cls = Rc::clone(&close_tab);
            let lh = Rc::clone(&local_host);
            let a = Rc::clone(&alive);
            Rc::new(move |dir: PathBuf| {
                if !a.get() {
                    return;
                }
                let tab_id = {
                    let Ok(mut s) = st.try_borrow_mut() else { return; };
                    let id = s.next_id;
                    s.next_id += 1;
                    id
                };
                let stack_name = format!("tab-{}", tab_id);

                let container = new_box(GTK_ORIENTATION_VERTICAL, 0, &[]);
                gtk_widget_set_hexpand(container, 1);
                gtk_widget_set_vexpand(container, 1);

                let pane = create_vte_pane(&dir);
                gtk_box_append(container, pane.vte);

                let c_name = cstr(&stack_name);
                gtk_stack_add_named(stack, container, c_name.as_ptr());

                // Tab pill in dynamic host bar: [ 💻 BijiOS  × ]
                let tab_pill = new_box(GTK_ORIENTATION_HORIZONTAL, 4, &["pancer-tab-active"]);
                let tab_btn = new_button(&format!("💻 {}", lh), &["pancer-tab-btn"]);
                let tab_close = new_button("×", &["pancer-tab-close"]);
                gtk_box_append(tab_pill, tab_btn);
                gtk_box_append(tab_pill, tab_close);
                gtk_box_append(tabs_box, tab_pill);

                {
                    let sel_c = Rc::clone(&sel);
                    connect_signal(tab_btn, "clicked", move || {
                        hide_all_menus();
                        sel_c(tab_id);
                    });
                }
                {
                    let cls_c = Rc::clone(&cls);
                    connect_signal(tab_close, "clicked", move || {
                        hide_all_menus();
                        cls_c(tab_id);
                    });
                }
                {
                    let cls_e = Rc::clone(&cls);
                    let st_e = Rc::clone(&st);
                    let a_e = Rc::clone(&a);
                    connect_notify(pane.vte, "child-exited", move || {
                        if !a_e.get() {
                            return;
                        }
                        // Use try_borrow() so re-entrant teardown never panics across FFI!
                        let single = st_e
                            .try_borrow()
                            .ok()
                            .and_then(|s| {
                                s.tabs
                                    .iter()
                                    .find(|t| t.id == tab_id)
                                    .map(|t| t.panes.len() <= 1)
                            })
                            .unwrap_or(false);
                        if single {
                            cls_e(tab_id);
                        }
                    });
                }

                if let Ok(mut s) = st.try_borrow_mut() {
                    s.tabs.push(PancerTab {
                        id: tab_id,
                        stack_name,
                        container,
                        panes: vec![pane],
                        tab_pill,
                        tab_btn,
                    });
                }

                sel(tab_id);
            })
        };

        // Helper: split active tab vertically (side-by-side) or horizontally (top-bottom)
        let split_active: Rc<dyn Fn(c_int)> = {
            let st = Rc::clone(&state);
            let a = Rc::clone(&alive);
            Rc::new(move |orientation: c_int| {
                if !a.get() {
                    return;
                }
                let (dir, container, tab_id) = {
                    let Ok(s) = st.try_borrow() else { return; };
                    let d = s
                        .active_pane()
                        .map(|p| read_pid_cwd(p.pid.get()))
                        .unwrap_or_else(home_dir);
                    let Some(tab) = s.active_tab() else { return; };
                    (d, tab.container, tab.id)
                };
                let existing = gtk_widget_get_first_child(container);
                if existing.is_null() {
                    return;
                }

                // Ref existing child BEFORE removing from container so GTK4 never frees it!
                g_object_ref(existing as GPointer);
                gtk_box_remove(container, existing);

                let paned = gtk_paned_new(orientation);
                gtk_widget_set_hexpand(paned, 1);
                gtk_widget_set_vexpand(paned, 1);

                let new_pane = create_vte_pane(&dir);
                let new_vte = new_pane.vte;

                gtk_paned_set_start_child(paned, existing);
                gtk_paned_set_end_child(paned, new_vte);
                gtk_paned_set_resize_start_child(paned, 1);
                gtk_paned_set_resize_end_child(paned, 1);
                gtk_paned_set_shrink_start_child(paned, 0);
                gtk_paned_set_shrink_end_child(paned, 0);

                g_object_unref(existing as GPointer);
                gtk_box_append(container, paned);
                if let Ok(mut s) = st.try_borrow_mut() {
                    if let Some(tab) = s.tabs.iter_mut().find(|t| t.id == tab_id) {
                        tab.panes.push(new_pane);
                    }
                }
                gtk_widget_grab_focus(new_vte);
            })
        };

        // Helper: menu item with icon, label, and right-aligned hotkey hint
        let new_menu_item = |icon: &str, text: &str, accel: &str| -> *mut GtkWidget {
            let btn = gtk_button_new();
            add_css(btn, "pancer-menu-item");
            let row = new_box(GTK_ORIENTATION_HORIZONTAL, 8, &[]);
            gtk_widget_set_hexpand(row, 1);
            let ic = new_label(icon, &[]);
            let lbl = new_label(text, &[]);
            gtk_label_set_xalign(lbl, 0.0);
            gtk_widget_set_hexpand(lbl, 1);
            let acc = new_label(accel, &["pancer-accel"]);
            gtk_label_set_xalign(acc, 1.0);
            gtk_box_append(row, ic);
            gtk_box_append(row, lbl);
            gtk_box_append(row, acc);
            gtk_button_set_child(btn, row);
            btn
        };

        // Shared action closures (used by both menu buttons AND keyboard hotkeys!)
        let act_new_win: Rc<dyn Fn()> = Rc::new(move || {
            hide_all_menus();
            open_pancer_with_dir(app, None);
        });
        let act_new_tab: Rc<dyn Fn()> = {
            let at = Rc::clone(&add_tab);
            Rc::new(move || {
                hide_all_menus();
                at(home_dir());
            })
        };
        let act_copy: Rc<dyn Fn()> = {
            let st = Rc::clone(&state);
            Rc::new(move || {
                hide_all_menus();
                let pane = st.try_borrow().ok().and_then(|s| s.active_pane());
                if let Some(p) = pane {
                    vte_terminal_copy_clipboard_format(p.vte, 1);
                    gtk_widget_grab_focus(p.vte);
                }
            })
        };
        let act_paste: Rc<dyn Fn()> = {
            let st = Rc::clone(&state);
            Rc::new(move || {
                hide_all_menus();
                let pane = st.try_borrow().ok().and_then(|s| s.active_pane());
                if let Some(p) = pane {
                    vte_terminal_paste_clipboard(p.vte);
                    gtk_widget_grab_focus(p.vte);
                }
            })
        };
        let act_clear: Rc<dyn Fn()> = {
            let st = Rc::clone(&state);
            Rc::new(move || {
                hide_all_menus();
                let pane = st.try_borrow().ok().and_then(|s| s.active_pane());
                if let Some(p) = pane {
                    vte_terminal_reset(p.vte, 1, 1);
                    gtk_widget_grab_focus(p.vte);
                }
            })
        };
        let act_dup_win: Rc<dyn Fn()> = {
            let st = Rc::clone(&state);
            Rc::new(move || {
                hide_all_menus();
                let dir = st
                    .try_borrow()
                    .ok()
                    .and_then(|s| s.active_pane())
                    .map(|p| read_pid_cwd(p.pid.get()))
                    .unwrap_or_else(home_dir);
                open_pancer_with_dir(app, Some(dir));
            })
        };
        let act_dup_tab: Rc<dyn Fn()> = {
            let st = Rc::clone(&state);
            let at = Rc::clone(&add_tab);
            Rc::new(move || {
                hide_all_menus();
                let dir = st
                    .try_borrow()
                    .ok()
                    .and_then(|s| s.active_pane())
                    .map(|p| read_pid_cwd(p.pid.get()))
                    .unwrap_or_else(home_dir);
                at(dir);
            })
        };
        let act_split_v: Rc<dyn Fn()> = {
            let sp = Rc::clone(&split_active);
            Rc::new(move || {
                hide_all_menus();
                sp(GTK_ORIENTATION_HORIZONTAL);
            })
        };
        let act_split_h: Rc<dyn Fn()> = {
            let sp = Rc::clone(&split_active);
            Rc::new(move || {
                hide_all_menus();
                sp(GTK_ORIENTATION_VERTICAL);
            })
        };
        let act_close_tab: Rc<dyn Fn()> = {
            let st = Rc::clone(&state);
            let cls = Rc::clone(&close_tab);
            Rc::new(move || {
                hide_all_menus();
                if let Ok(s) = st.try_borrow() {
                    let aid = s.active_tab_id;
                    drop(s);
                    cls(aid);
                }
            })
        };

        let act_toggle_sidebar: Rc<dyn Fn()> = Rc::new(move || {
            hide_all_menus();
            let now_vis = gtk_widget_get_visible(ssh_sidebar) != 0;
            let next_vis = !now_vis;
            gtk_widget_set_visible(ssh_sidebar, if next_vis { 1 } else { 0 });
            let lbl = cstr(if next_vis { "◧ Hide Sidebar" } else { "◧ SSH Hosts" });
            gtk_button_set_label(btn_toggle_sb, lbl.as_ptr());
            save_sidebar_visible(next_vis);
        });

        let act_toggle_broadcast: Rc<dyn Fn()> = {
            let st_bc = Rc::clone(&state);
            Rc::new(move || {
                hide_all_menus();
                let now_vis = gtk_widget_get_visible(broadcast_bar) != 0;
                let next_vis = !now_vis;
                gtk_widget_set_visible(broadcast_bar, if next_vis { 1 } else { 0 });
                let lbl = cstr(if next_vis { "📡 Broadcast ON" } else { "📡 Broadcast" });
                gtk_button_set_label(btn_broadcast, lbl.as_ptr());
                if next_vis {
                    gtk_widget_grab_focus(bc_entry);
                } else if let Some(p) = st_bc.try_borrow().ok().and_then(|s| s.active_pane()) {
                    gtk_widget_grab_focus(p.vte);
                }
            })
        };

        // Scope switch (All Tabs & Panes vs Current Tab)
        {
            let sc = Rc::clone(&bc_all_tabs);
            connect_signal(btn_bc_scope, "clicked", move || {
                let next = !sc.get();
                sc.set(next);
                let lbl = cstr(if next { "All Tabs" } else { "Current Tab" });
                gtk_button_set_label(btn_bc_scope, lbl.as_ptr());
                gtk_widget_grab_focus(bc_entry);
            });
        }
        {
            let tb = Rc::clone(&act_toggle_broadcast);
            connect_signal(btn_bc_close, "clicked", move || tb());
        }

        // Send broadcast command to target VTE panes simultaneously
        let send_broadcast: Rc<dyn Fn()> = {
            let st_bc = Rc::clone(&state);
            let sc = Rc::clone(&bc_all_tabs);
            Rc::new(move || {
                let cmd = get_entry_text(bc_entry);
                if cmd.is_empty() {
                    return;
                }
                let targets: Vec<*mut GtkWidget> = {
                    let Ok(s) = st_bc.try_borrow() else { return; };
                    if sc.get() {
                        s.tabs
                            .iter()
                            .flat_map(|t| t.panes.iter().map(|p| p.vte))
                            .collect()
                    } else {
                        s.active_tab()
                            .map(|t| t.panes.iter().map(|p| p.vte).collect())
                            .unwrap_or_default()
                    }
                };
                let cmd_nl = format!("{}\n", cmd);
                let c = cstr(&cmd_nl);
                for vte in &targets {
                    vte_terminal_feed_child(*vte, c.as_ptr(), cmd_nl.len() as isize);
                }
                set_entry_text(bc_entry, "");
                set_label_text(bc_status, &format!("✓ Sent to {} pane(s)", targets.len()));
                gtk_widget_grab_focus(bc_entry);
            })
        };
        {
            let sb1 = Rc::clone(&send_broadcast);
            connect_signal(btn_bc_send, "clicked", move || sb1());
            let sb2 = Rc::clone(&send_broadcast);
            connect_signal(bc_entry, "activate", move || sb2());
        }

        // Populate ~/.ssh/config Host Sidebar (Connect, Edit, Delete)
        let populate_ssh_hosts: Rc<RefCell<Box<dyn Fn()>>> = Rc::new(RefCell::new(Box::new(|| {})));
        let pop_ssh_c = Rc::clone(&populate_ssh_hosts);
        {
            let at_ssh = Rc::clone(&add_tab);
            let st_ssh = Rc::clone(&state);
            let a_ssh = Rc::clone(&alive);
            *populate_ssh_hosts.borrow_mut() = Box::new(move || {
                clear_box_children(ssh_list_box);
                let q = get_entry_text(ssh_filter).trim().to_lowercase();
                let hosts: Vec<SshHost> = read_ssh_hosts()
                    .into_iter()
                    .filter(|h| {
                        q.is_empty()
                            || h.alias.to_lowercase().contains(&q)
                            || h.hostname.to_lowercase().contains(&q)
                            || h.user.to_lowercase().contains(&q)
                    })
                    .collect();

                if hosts.is_empty() {
                    let empty = new_label(
                        if q.is_empty() {
                            "No SSH hosts in ~/.ssh/config.\nClick '➕ Add' to add one."
                        } else {
                            "No matching SSH hosts."
                        },
                        &["pancer-ssh-meta"],
                    );
                    gtk_label_set_xalign(empty, 0.0);
                    gtk_label_set_wrap(empty, 1);
                    set_margin_all(empty, 6);
                    gtk_box_append(ssh_list_box, empty);
                    return;
                }

                for h in hosts {
                    let card = new_box(GTK_ORIENTATION_VERTICAL, 4, &["pancer-ssh-card"]);
                    let conn_btn = gtk_button_new();
                    add_css(conn_btn, "pancer-ssh-connect");
                    gtk_widget_set_hexpand(conn_btn, 1);

                    let info = new_box(GTK_ORIENTATION_VERTICAL, 1, &[]);
                    let al_lbl = new_label(&format!("🔐 {}", h.alias), &["pancer-ssh-alias"]);
                    gtk_label_set_xalign(al_lbl, 0.0);
                    gtk_label_set_ellipsize(al_lbl, 3);
                    let meta_str = if h.user.is_empty() {
                        format!("{}:{}", h.hostname, h.port)
                    } else {
                        format!("{}@{}:{}", h.user, h.hostname, h.port)
                    };
                    let m_lbl = new_label(&meta_str, &["pancer-ssh-meta"]);
                    gtk_label_set_xalign(m_lbl, 0.0);
                    gtk_label_set_ellipsize(m_lbl, 3);
                    gtk_box_append(info, al_lbl);
                    gtk_box_append(info, m_lbl);
                    gtk_button_set_child(conn_btn, info);
                    let tip = cstr(&format!("Click to connect: ssh {}", h.alias));
                    gtk_widget_set_tooltip_text(conn_btn, tip.as_ptr());

                    // Compact action row: Files (Download/Upload) | Mount in Akar | Edit | Delete
                    let act_row = new_box(GTK_ORIENTATION_HORIZONTAL, 4, &[]);
                    let btn_files = new_button("📂 Files", &["pancer-ssh-act"]);
                    gtk_widget_set_hexpand(btn_files, 1);
                    let tip_f = cstr("Browse, Download & Upload Remote Files");
                    gtk_widget_set_tooltip_text(btn_files, tip_f.as_ptr());

                    let btn_mount = new_button("🔗 Mount", &["pancer-ssh-act"]);
                    gtk_widget_set_hexpand(btn_mount, 1);
                    let tip_m = cstr("Mount Host via SSHFS & Open in Akar File Manager");
                    gtk_widget_set_tooltip_text(btn_mount, tip_m.as_ptr());

                    let btn_edit = new_button("✏️", &["pancer-ssh-act"]);
                    let tip_e = cstr("Edit SSH Host");
                    gtk_widget_set_tooltip_text(btn_edit, tip_e.as_ptr());
                    let btn_del = new_button("🗑️", &["pancer-ssh-act", "pancer-ssh-del"]);
                    let tip_d = cstr("Delete SSH Host");
                    gtk_widget_set_tooltip_text(btn_del, tip_d.as_ptr());

                    gtk_box_append(act_row, btn_files);
                    gtk_box_append(act_row, btn_mount);
                    gtk_box_append(act_row, btn_edit);
                    gtk_box_append(act_row, btn_del);

                    gtk_box_append(card, conn_btn);
                    gtk_box_append(card, act_row);
                    gtk_box_append(ssh_list_box, card);

                    // Browse remote folder (Download / Upload)
                    {
                        let alias_f = h.alias.clone();
                        connect_signal(btn_files, "clicked", move || {
                            hide_all_menus();
                            open_ssh_file_browser(app, win, alias_f.clone());
                        });
                    }
                    // Mount remote folder via SSHFS & open in Akar File Manager
                    {
                        let alias_m = h.alias.clone();
                        connect_signal(btn_mount, "clicked", move || {
                            hide_all_menus();
                            set_label_text(ssh_status_lbl, &format!("⏳ Mounting {}...", alias_m));
                            gtk_widget_set_visible(ssh_status_lbl, 1);
                            match mount_ssh_host(&alias_m, "~") {
                                Ok(mnt) => {
                                    set_label_text(
                                        ssh_status_lbl,
                                        &format!("✅ Mounted {} → Akar", alias_m),
                                    );
                                    open_finder_at(app, mnt);
                                }
                                Err(e) => {
                                    set_label_text(ssh_status_lbl, &format!("❌ {}", e));
                                }
                            }
                        });
                    }

                    // Click host -> open new tab & auto-run `ssh <alias>`
                    {
                        let alias_c = h.alias.clone();
                        let at = Rc::clone(&at_ssh);
                        let st = Rc::clone(&st_ssh);
                        let alv = Rc::clone(&a_ssh);
                        connect_signal(conn_btn, "clicked", move || {
                            hide_all_menus();
                            at(home_dir());
                            let st2 = Rc::clone(&st);
                            let alv2 = Rc::clone(&alv);
                            let cmd_str = format!("ssh {}\n", alias_c);
                            timeout_add_ms(220, move || {
                                if alv2.get() {
                                    let vte_opt = st2
                                        .try_borrow()
                                        .ok()
                                        .and_then(|s| s.active_pane())
                                        .map(|p| p.vte);
                                    if let Some(vte) = vte_opt {
                                        let c = cstr(&cmd_str);
                                        vte_terminal_feed_child(vte, c.as_ptr(), cmd_str.len() as isize);
                                        gtk_widget_grab_focus(vte);
                                    }
                                }
                                false
                            });
                        });
                    }
                    // Edit host
                    {
                        let h_edit = h.clone();
                        let pc = Rc::clone(&pop_ssh_c);
                        connect_signal(btn_edit, "clicked", move || {
                            hide_all_menus();
                            open_ssh_host_dialog(app, win, Some(h_edit.clone()), Rc::clone(&pc));
                        });
                    }
                    // Delete host
                    {
                        let alias_del = h.alias.clone();
                        let pc = Rc::clone(&pop_ssh_c);
                        connect_signal(btn_del, "clicked", move || {
                            hide_all_menus();
                            if update_ssh_config(Some(&alias_del), None) {
                                (pc.borrow())();
                            }
                        });
                    }
                }
            });
        }

        {
            let pc = Rc::clone(&populate_ssh_hosts);
            connect_signal(btn_ssh_add, "clicked", move || {
                hide_all_menus();
                open_ssh_host_dialog(app, win, None, Rc::clone(&pc));
            });
        }
        {
            let pc = Rc::clone(&populate_ssh_hosts);
            connect_signal(ssh_filter, "changed", move || {
                (pc.borrow())();
            });
        }

        // ---------------------------------------------------------------------
        // Populate File / Edit / Shell Dropdown Menu Items (with Hotkeys!)
        // ---------------------------------------------------------------------
        let mi_new_win = new_menu_item("🪟", "New Window", "Ctrl+Shift+N");
        let mi_new_tab = new_menu_item("➕", "New Tab", "Ctrl+Shift+T");
        gtk_box_append(menu_file, mi_new_win);
        gtk_box_append(menu_file, mi_new_tab);

        let mi_copy = new_menu_item("📋", "Copy", "Ctrl+Shift+C");
        let mi_paste = new_menu_item("📥", "Paste", "Ctrl+Shift+V");
        let mi_clear = new_menu_item("🧹", "Clear", "Ctrl+Shift+K");
        gtk_box_append(menu_edit, mi_copy);
        gtk_box_append(menu_edit, mi_paste);
        gtk_box_append(menu_edit, mi_clear);

        let mi_dup_win = new_menu_item("🪟", "Duplicate Window", "Ctrl+Shift+U");
        let mi_dup_tab = new_menu_item("📑", "Duplicate Tab", "Ctrl+Shift+B");
        let mi_split_v = new_menu_item("◫", "Split Vertical", "Ctrl+Shift+D");
        let mi_split_h = new_menu_item("⬓", "Split Horizontal", "Ctrl+Shift+E");
        let mi_toggle_sb = new_menu_item("◧", "Toggle SSH Sidebar", "Ctrl+Shift+S");
        let mi_broadcast = new_menu_item("📡", "Broadcast Command", "Ctrl+Shift+M");
        let mi_close_tab = new_menu_item("✖", "Close Tab", "Ctrl+Shift+W");

        gtk_box_append(menu_shell, mi_dup_win);
        gtk_box_append(menu_shell, mi_dup_tab);
        gtk_box_append(menu_shell, gtk_separator_new(GTK_ORIENTATION_HORIZONTAL));
        gtk_box_append(menu_shell, mi_split_v);
        gtk_box_append(menu_shell, mi_split_h);
        gtk_box_append(menu_shell, gtk_separator_new(GTK_ORIENTATION_HORIZONTAL));
        gtk_box_append(menu_shell, mi_toggle_sb);
        gtk_box_append(menu_shell, mi_broadcast);
        gtk_box_append(menu_shell, mi_close_tab);

        let mi_about_pancer = new_menu_item("🌱", "About Pancer", "");
        gtk_box_append(menu_help, mi_about_pancer);
        connect_signal(mi_about_pancer, "clicked", move || {
            hide_all_menus();
            open_about_pancer(app, win);
        });

        let wire = |btn: *mut GtkWidget, f: &Rc<dyn Fn()>| {
            let fc = Rc::clone(f);
            connect_signal(btn, "clicked", move || fc());
        };
        wire(mi_new_win, &act_new_win);
        wire(mi_new_tab, &act_new_tab);
        wire(mi_copy, &act_copy);
        wire(mi_paste, &act_paste);
        wire(mi_clear, &act_clear);
        wire(mi_dup_win, &act_dup_win);
        wire(mi_dup_tab, &act_dup_tab);
        wire(mi_split_v, &act_split_v);
        wire(mi_split_h, &act_split_h);
        wire(mi_toggle_sb, &act_toggle_sidebar);
        wire(btn_toggle_sb, &act_toggle_sidebar);
        wire(mi_broadcast, &act_toggle_broadcast);
        wire(btn_broadcast, &act_toggle_broadcast);
        wire(mi_close_tab, &act_close_tab);
        wire(btn_add_tab, &act_new_tab);

        // ---------------------------------------------------------------------
        // Global Capture-Phase Hotkeys (Ctrl+Shift+<Key> + Escape)
        // ---------------------------------------------------------------------
        {
            let f_nw = Rc::clone(&act_new_win);
            let f_nt = Rc::clone(&act_new_tab);
            let f_cp = Rc::clone(&act_copy);
            let f_ps = Rc::clone(&act_paste);
            let f_cl = Rc::clone(&act_clear);
            let f_dw = Rc::clone(&act_dup_win);
            let f_dt = Rc::clone(&act_dup_tab);
            let f_sv = Rc::clone(&act_split_v);
            let f_sh = Rc::clone(&act_split_h);
            let f_sb = Rc::clone(&act_toggle_sidebar);
            let f_bc = Rc::clone(&act_toggle_broadcast);
            let f_ct = Rc::clone(&act_close_tab);
            let st_k = Rc::clone(&state);
            connect_key_pressed(win, move |keyval, mods| {
                // Escape closes any open dropdown menu or broadcast bar cleanly
                if keyval == 0xff1b {
                    if any_menu_open() {
                        hide_all_menus();
                        let pane = st_k.try_borrow().ok().and_then(|s| s.active_pane());
                        if let Some(p) = pane {
                            gtk_widget_grab_focus(p.vte);
                        }
                        return true;
                    }
                    if gtk_widget_get_visible(broadcast_bar) != 0 {
                        f_bc();
                        return true;
                    }
                }
                let ctrl = (mods & 4) != 0;  // GDK_CONTROL_MASK
                let shift = (mods & 1) != 0; // GDK_SHIFT_MASK
                if !(ctrl && shift) {
                    return false;
                }
                let ch = char::from_u32(keyval).map(|c| c.to_ascii_uppercase());
                match ch {
                    Some('N') => { f_nw(); true }
                    Some('T') => { f_nt(); true }
                    Some('C') => { f_cp(); true }
                    Some('V') => { f_ps(); true }
                    Some('K') => { f_cl(); true }
                    Some('U') => { f_dw(); true }
                    Some('B') => { f_dt(); true }
                    Some('D') => { f_sv(); true }
                    Some('E') => { f_sh(); true }
                    Some('S') => { f_sb(); true }
                    Some('M') => { f_bc(); true }
                    Some('W') => { f_ct(); true }
                    _ => false,
                }
            });
        }

        (populate_ssh_hosts.borrow())();

        // Dynamic Host Detection Timer (safe against window destroy — never touches freed win!)
        {
            let st = Rc::clone(&state);
            let lh = Rc::clone(&local_host);
            let a = Rc::clone(&alive);
            timeout_add_sec(1, move || {
                if !a.get() {
                    return false;
                }
                let Ok(s) = st.try_borrow() else { return true; };
                for tab in &s.tabs {
                    if let Some(p) = tab.panes.first() {
                        let label = detect_pane_host(p.pid.get(), p.vte, &lh);
                        let c = cstr(&label);
                        gtk_button_set_label(tab.tab_btn, c.as_ptr());
                    }
                }
                true
            });
        }

        // Attach widget tree & show window BEFORE opening initial tab so focus works cleanly
        gtk_window_set_child(win, overlay);
        gtk_window_present(win);

        // Open initial tab
        add_tab(initial_dir.unwrap_or_else(home_dir));
    }
}

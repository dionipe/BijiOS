//! BijiOS 1.0.0 "Padi" — Native System Settings (macOS Sidebar Layout)
//! Sections: Desktop & Dock, Wallpaper (Nature + Dynamic Previews),
//! General (About, Software Update, Storage, System), Users & Groups.

use crate::native_de::apps::{create_app_window, open_finder, read_mem_usage};
use crate::native_de::gtk4_ffi::*;
use crate::native_de::{apply_desktop_settings, resolve_effective_wallpaper_file, DesktopHandles};
use std::cell::RefCell;
use std::fs;
use std::process::{Command, Stdio};
use std::rc::Rc;

// ============================================================================
// System & Linux User/Group Helpers
// ============================================================================

#[derive(Clone)]
struct LinuxUser {
    username: String,
    uid: u32,
    full_name: String,
    home: String,
    shell: String,
    groups: String,
}

fn read_linux_users() -> Vec<LinuxUser> {
    let mut users = Vec::new();
    let Ok(txt) = fs::read_to_string("/etc/passwd") else {
        return users;
    };
    for line in txt.lines() {
        let parts: Vec<&str> = line.split(':').collect();
        if parts.len() < 7 {
            continue;
        }
        let Ok(uid) = parts[2].parse::<u32>() else {
            continue;
        };
        if !(1000..60000).contains(&uid) {
            continue;
        }
        let username = parts[0].to_string();
        let full_name = parts[4].split(',').next().unwrap_or("").trim().to_string();
        let display_name = if full_name.is_empty() {
            username.clone()
        } else {
            full_name
        };
        let groups = Command::new("id")
            .args(["-Gn", &username])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.split_whitespace().collect::<Vec<_>>().join(", "))
            .unwrap_or_default();

        users.push(LinuxUser {
            username,
            uid,
            full_name: display_name,
            home: parts[5].to_string(),
            shell: parts[6].to_string(),
            groups,
        });
    }
    users
}

fn read_linux_groups() -> Vec<(String, u32, String)> {
    let mut groups = Vec::new();
    let Ok(txt) = fs::read_to_string("/etc/group") else {
        return groups;
    };
    let interesting = ["sudo", "adm", "users", "autologin", "docker", "plugdev"];
    for line in txt.lines() {
        let parts: Vec<&str> = line.split(':').collect();
        if parts.len() < 4 {
            continue;
        }
        let name = parts[0].to_string();
        let Ok(gid) = parts[2].parse::<u32>() else {
            continue;
        };
        if interesting.contains(&name.as_str()) || (1000..60000).contains(&gid) {
            let members = if parts[3].trim().is_empty() {
                "—".to_string()
            } else {
                parts[3].replace(',', ", ")
            };
            groups.push((name, gid, members));
        }
    }
    groups
}

fn read_storage_info() -> (String, f64) {
    if let Ok(out) = Command::new("df").args(["-h", "/"]).output() {
        let s = String::from_utf8_lossy(&out.stdout);
        if let Some(line) = s.lines().nth(1) {
            let cols: Vec<&str> = line.split_whitespace().collect();
            if cols.len() >= 5 {
                let pct = cols[4]
                    .trim_end_matches('%')
                    .parse::<f64>()
                    .unwrap_or(0.0)
                    / 100.0;
                return (
                    format!(
                        "Used: {}  •  Available: {}  •  Total: {} ({})",
                        cols[2], cols[3], cols[1], cols[4]
                    ),
                    pct.clamp(0.0, 1.0),
                );
            }
        }
    }
    ("Storage information unavailable".to_string(), 0.0)
}

fn read_kernel_version() -> String {
    Command::new("uname")
        .arg("-sr")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "Linux".to_string())
}

fn read_cpu_model() -> String {
    if let Ok(txt) = fs::read_to_string("/proc/cpuinfo") {
        for line in txt.lines() {
            if line.starts_with("model name") {
                if let Some((_, val)) = line.split_once(':') {
                    return val.trim().to_string();
                }
            }
        }
    }
    "x86_64 Processor".to_string()
}

fn read_uptime_str() -> String {
    if let Ok(txt) = fs::read_to_string("/proc/uptime") {
        if let Some(secs_s) = txt.split_whitespace().next() {
            if let Ok(secs) = secs_s.parse::<f64>() {
                let total = secs as u64;
                let h = total / 3600;
                let m = (total % 3600) / 60;
                return format!("{}h {}m", h, m);
            }
        }
    }
    "Active".to_string()
}

pub(crate) fn run_pkexec_sh(script: &str) -> bool {
    // Try non-interactive sudo first (works seamlessly on Openbox without a Polkit GUI agent)
    if let Ok(st) = Command::new("sudo")
        .args(["-n", "/bin/bash", "-c", script])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
    {
        if st.success() {
            return true;
        }
    }
    Command::new("pkexec")
        .args(["/bin/bash", "-c", script])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub(crate) fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

// ============================================================================
// Modal Dialogs: Edit User & Change Password
// ============================================================================

unsafe fn open_change_password_dialog(
    app: *mut GtkApplication,
    username: String,
    status_lbl: *mut GtkWidget,
) {
    let win = create_app_window(app, &format!("🔑  Change Password — {}", username), 420, 260);
    gtk_window_set_resizable(win, 0);
    let root = new_box(GTK_ORIENTATION_VERTICAL, 12, &[]);
    set_margin_all(root, 20);

    let hdr = new_label(&format!("Change Password for {}", username), &["app-heading"]);
    gtk_label_set_xalign(hdr, 0.0);
    gtk_box_append(root, hdr);

    let pass1 = gtk_entry_new();
    let ph1 = cstr("New password");
    gtk_entry_set_placeholder_text(pass1, ph1.as_ptr());
    gtk_entry_set_visibility(pass1, 0);

    let pass2 = gtk_entry_new();
    let ph2 = cstr("Confirm new password");
    gtk_entry_set_placeholder_text(pass2, ph2.as_ptr());
    gtk_entry_set_visibility(pass2, 0);

    let msg = new_label("", &["app-muted"]);
    gtk_label_set_xalign(msg, 0.0);

    let btn_row = new_box(GTK_ORIENTATION_HORIZONTAL, 8, &[]);
    gtk_widget_set_halign(btn_row, GTK_ALIGN_END);
    let btn_cancel = new_button("Cancel", &["toolbar-btn"]);
    let btn_save = new_button("Update Password", &["primary-btn"]);
    gtk_box_append(btn_row, btn_cancel);
    gtk_box_append(btn_row, btn_save);

    gtk_box_append(root, pass1);
    gtk_box_append(root, pass2);
    gtk_box_append(root, msg);
    gtk_box_append(root, btn_row);

    connect_signal(btn_cancel, "clicked", move || gtk_window_close(win));
    let u_c = username.clone();
    connect_signal(btn_save, "clicked", move || {
        let p1 = get_entry_text(pass1);
        let p2 = get_entry_text(pass2);
        if p1.is_empty() {
            set_label_text(msg, "Password cannot be empty.");
            return;
        }
        if p1 != p2 {
            set_label_text(msg, "Passwords do not match.");
            return;
        }
        let script = format!(
            "printf '%s:%s\\n' {} {} | chpasswd",
            shell_quote(&u_c),
            shell_quote(&p1)
        );
        if run_pkexec_sh(&script) {
            set_label_text(status_lbl, &format!("✅ Password updated for '{}'.", u_c));
            gtk_window_close(win);
        } else {
            set_label_text(msg, "❌ Authorization cancelled or failed.");
        }
    });

    gtk_window_set_child(win, root);
    gtk_window_present(win);
    gtk_widget_grab_focus(pass1);
}

unsafe fn open_edit_user_dialog(
    app: *mut GtkApplication,
    user: LinuxUser,
    status_lbl: *mut GtkWidget,
    on_refresh: Rc<RefCell<Box<dyn Fn()>>>,
) {
    let win = create_app_window(app, &format!("✏️  Edit User — {}", user.username), 440, 290);
    gtk_window_set_resizable(win, 0);
    let root = new_box(GTK_ORIENTATION_VERTICAL, 12, &[]);
    set_margin_all(root, 20);

    let hdr = new_label(&format!("Edit Account: {}", user.username), &["app-heading"]);
    gtk_label_set_xalign(hdr, 0.0);
    gtk_box_append(root, hdr);

    let fn_lbl = new_label("Full Name", &["app-muted"]);
    gtk_label_set_xalign(fn_lbl, 0.0);
    let fn_entry = gtk_entry_new();
    set_entry_text(fn_entry, &user.full_name);

    let grp_lbl = new_label("Add to Group (optional, e.g. sudo, docker, users)", &["app-muted"]);
    gtk_label_set_xalign(grp_lbl, 0.0);
    let grp_entry = gtk_entry_new();
    let gph = cstr("Leave empty to keep current groups");
    gtk_entry_set_placeholder_text(grp_entry, gph.as_ptr());

    let msg = new_label("", &["app-muted"]);
    gtk_label_set_xalign(msg, 0.0);

    let btn_row = new_box(GTK_ORIENTATION_HORIZONTAL, 8, &[]);
    gtk_widget_set_halign(btn_row, GTK_ALIGN_END);
    let btn_cancel = new_button("Cancel", &["toolbar-btn"]);
    let btn_save = new_button("Save Changes", &["primary-btn"]);
    gtk_box_append(btn_row, btn_cancel);
    gtk_box_append(btn_row, btn_save);

    gtk_box_append(root, fn_lbl);
    gtk_box_append(root, fn_entry);
    gtk_box_append(root, grp_lbl);
    gtk_box_append(root, grp_entry);
    gtk_box_append(root, msg);
    gtk_box_append(root, btn_row);

    connect_signal(btn_cancel, "clicked", move || gtk_window_close(win));
    let u_c = user.username.clone();
    connect_signal(btn_save, "clicked", move || {
        let new_fn = get_entry_text(fn_entry).trim().to_string();
        let add_grp = get_entry_text(grp_entry).trim().to_string();
        let mut cmds = Vec::new();
        if !new_fn.is_empty() {
            cmds.push(format!("chfn -f {} {}", shell_quote(&new_fn), shell_quote(&u_c)));
        }
        if !add_grp.is_empty() {
            cmds.push(format!(
                "usermod -aG {} {}",
                shell_quote(&add_grp),
                shell_quote(&u_c)
            ));
        }
        if cmds.is_empty() {
            gtk_window_close(win);
            return;
        }
        let script = cmds.join(" && ");
        if run_pkexec_sh(&script) {
            set_label_text(status_lbl, &format!("✅ Updated account '{}'.", u_c));
            (on_refresh.borrow())();
            gtk_window_close(win);
        } else {
            set_label_text(msg, "❌ Failed to update user (check group name or auth).");
        }
    });

    gtk_window_set_child(win, root);
    gtk_window_present(win);
}

// ============================================================================
// Main System Settings Window (macOS Sidebar + GtkStack)
// ============================================================================

pub fn open_system_settings(
    app: *mut GtkApplication,
    handles: Rc<DesktopHandles>,
    on_lock: Rc<dyn Fn()>,
) {
    unsafe {
        let win = create_app_window(app, "⚙️  System Settings", 960, 640);
        let root = new_box(GTK_ORIENTATION_HORIZONTAL, 0, &[]);

        // ---------------------------------------------------------------------
        // Left Sidebar (Fixed 215px width — hexpand=0 so content gets full space!)
        // ---------------------------------------------------------------------
        let sidebar = new_box(GTK_ORIENTATION_VERTICAL, 4, &["app-sidebar"]);
        gtk_widget_set_size_request(sidebar, 215, -1);
        gtk_widget_set_hexpand(sidebar, 0);
        gtk_widget_set_halign(sidebar, GTK_ALIGN_START);

        let cur_user = std::env::var("USER").unwrap_or_else(|_| "dionipe".to_string());
        let cur_av = handles.settings.borrow().user_avatar.clone();

        // Profile Mini Card in Sidebar (reflects custom user avatar)
        let prof_mini = new_box(GTK_ORIENTATION_HORIZONTAL, 10, &["app-card"]);
        let av = new_user_avatar_widget(&cur_av, &cur_user, 44);
        gtk_widget_set_valign(av, GTK_ALIGN_CENTER);
        let prof_txt = new_box(GTK_ORIENTATION_VERTICAL, 1, &[]);
        let p_name = new_label(&cur_user, &["finder-name-btn"]);
        gtk_label_set_xalign(p_name, 0.0);
        let p_sub = new_label("BijiOS 1.0.0 Padi", &["finder-meta-lbl"]);
        gtk_label_set_xalign(p_sub, 0.0);
        gtk_box_append(prof_txt, p_name);
        gtk_box_append(prof_txt, p_sub);
        gtk_box_append(prof_mini, av);
        gtk_box_append(prof_mini, prof_txt);
        gtk_box_append(sidebar, prof_mini);

        let hdr_app = new_label("APPEARANCE", &["sidebar-section-hdr"]);
        gtk_label_set_xalign(hdr_app, 0.0);
        gtk_box_append(sidebar, hdr_app);

        let stack = gtk_stack_new();
        gtk_widget_set_hexpand(stack, 1);
        gtk_widget_set_vexpand(stack, 1);

        let nav_items = [
            ("dock", "🎛️", "Desktop & Dock", true),
            ("display", "🖥️", "Displays", false),
            ("wallpaper", "🖼️", "Wallpaper", false),
        ];
        let nav_sys = [
            ("sound", "🔊", "Sound"),
            ("network", "🌐", "Network"),
            ("general", "⚙️", "General"),
            ("users", "👥", "Users & Groups"),
        ];

        let all_nav_btns: Rc<RefCell<Vec<*mut GtkWidget>>> = Rc::new(RefCell::new(Vec::new()));

        for (page, icon, label, active) in nav_items {
            let cls = if active { "sidebar-btn-active" } else { "sidebar-btn" };
            let btn = new_left_button(icon, label, &[cls]);
            gtk_box_append(sidebar, btn);
            all_nav_btns.borrow_mut().push(btn);

            let page_s = page.to_string();
            let btns_r = Rc::clone(&all_nav_btns);
            connect_signal(btn, "clicked", move || {
                for &b in btns_r.borrow().iter() {
                    remove_css(b, "sidebar-btn-active");
                    add_css(b, "sidebar-btn");
                }
                remove_css(btn, "sidebar-btn");
                add_css(btn, "sidebar-btn-active");
                let c = cstr(&page_s);
                gtk_stack_set_visible_child_name(stack, c.as_ptr());
            });
        }

        let hdr_sys = new_label("SYSTEM", &["sidebar-section-hdr"]);
        gtk_label_set_xalign(hdr_sys, 0.0);
        gtk_box_append(sidebar, hdr_sys);

        for (page, icon, label) in nav_sys {
            let btn = new_left_button(icon, label, &["sidebar-btn"]);
            gtk_box_append(sidebar, btn);
            all_nav_btns.borrow_mut().push(btn);

            let page_s = page.to_string();
            let btns_r = Rc::clone(&all_nav_btns);
            connect_signal(btn, "clicked", move || {
                for &b in btns_r.borrow().iter() {
                    remove_css(b, "sidebar-btn-active");
                    add_css(b, "sidebar-btn");
                }
                remove_css(btn, "sidebar-btn");
                add_css(btn, "sidebar-btn-active");
                let c = cstr(&page_s);
                gtk_stack_set_visible_child_name(stack, c.as_ptr());
            });
        }

        // ---------------------------------------------------------------------
        // PAGE 1: Desktop & Dock
        // ---------------------------------------------------------------------
        let dock_page = build_dock_page(Rc::clone(&handles));
        let n_dock = cstr("dock");
        gtk_stack_add_named(stack, dock_page, n_dock.as_ptr());

        // ---------------------------------------------------------------------
        // PAGE 2: Displays & Remote Desktop (VNC)
        // ---------------------------------------------------------------------
        let disp_page = build_display_page(Rc::clone(&handles));
        let n_disp = cstr("display");
        gtk_stack_add_named(stack, disp_page, n_disp.as_ptr());

        // ---------------------------------------------------------------------
        // PAGE 2: Wallpaper (Nature Wallpapers + Dynamic Wallpaper + Previews)
        // ---------------------------------------------------------------------
        let wp_page = build_wallpaper_page(Rc::clone(&handles));
        let n_wp = cstr("wallpaper");
        gtk_stack_add_named(stack, wp_page, n_wp.as_ptr());

        // ---------------------------------------------------------------------
        // PAGE 4: Sound (Hardware & Virtual PipeWire/PulseAudio)
        // ---------------------------------------------------------------------
        let snd_page = crate::native_de::sys_hw::build_sound_page();
        let n_snd = cstr("sound");
        gtk_stack_add_named(stack, snd_page, n_snd.as_ptr());

        // ---------------------------------------------------------------------
        // PAGE 5: Network (Ethernet & Wi-Fi via nmcli)
        // ---------------------------------------------------------------------
        let net_page = crate::native_de::sys_hw::build_network_page(app);
        let n_net = cstr("network");
        gtk_stack_add_named(stack, net_page, n_net.as_ptr());

        // ---------------------------------------------------------------------
        // PAGE 6: General (About, Software Update, Storage, System)
        // ---------------------------------------------------------------------
        let gen_page = build_general_page(app, win, Rc::clone(&on_lock));
        let n_gen = cstr("general");
        gtk_stack_add_named(stack, gen_page, n_gen.as_ptr());

        // ---------------------------------------------------------------------
        // PAGE 4: Users & Groups (Profile, Avatar, Add/Edit/Password User, Groups)
        // ---------------------------------------------------------------------
        let usr_page = build_users_page(app, Rc::clone(&handles));
        let n_usr = cstr("users");
        gtk_stack_add_named(stack, usr_page, n_usr.as_ptr());

        gtk_stack_set_visible_child_name(stack, n_dock.as_ptr());

        gtk_box_append(root, sidebar);
        gtk_box_append(root, stack);
        gtk_window_set_child(win, root);
        gtk_window_present(win);
    }
}

// ============================================================================
// Page 1: Desktop & Dock
// ============================================================================
unsafe fn build_dock_page(handles: Rc<DesktopHandles>) -> *mut GtkWidget {
    let scroller = gtk_scrolled_window_new();
    gtk_scrolled_window_set_policy(scroller, GTK_POLICY_NEVER, GTK_POLICY_AUTOMATIC);
    gtk_widget_set_hexpand(scroller, 1);
    let page = new_box(GTK_ORIENTATION_VERTICAL, 14, &[]);
    set_margin_all(page, 22);

    let title = new_label("Desktop & Dock", &["app-heading"]);
    gtk_label_set_xalign(title, 0.0);
    gtk_box_append(page, title);

    let cur = handles.settings.borrow().clone();

    let dock_card = new_box(GTK_ORIENTATION_VERTICAL, 14, &["app-card"]);
    let hdr = new_label("DOCK APPEARANCE & BEHAVIOR", &["app-section-title"]);
    gtk_label_set_xalign(hdr, 0.0);
    gtk_box_append(dock_card, hdr);

    // 0. Hide / Unhide Dock
    let hide_row = new_box(GTK_ORIENTATION_HORIZONTAL, 12, &[]);
    let hide_lbl = new_label("Hide Dock", &["finder-name-btn"]);
    gtk_label_set_xalign(hide_lbl, 0.0);
    gtk_widget_set_hexpand(hide_lbl, 1);
    let hide_state_lbl = new_label(
        if cur.dock_hidden { "Hidden" } else { "Visible" },
        &["app-muted"],
    );
    let hide_switch = gtk_switch_new();
    gtk_widget_set_valign(hide_switch, GTK_ALIGN_CENTER);
    gtk_switch_set_active(hide_switch, if cur.dock_hidden { 1 } else { 0 });
    gtk_box_append(hide_row, hide_lbl);
    gtk_box_append(hide_row, hide_state_lbl);
    gtk_box_append(hide_row, hide_switch);
    gtk_box_append(dock_card, hide_row);

    {
        let h = Rc::clone(&handles);
        connect_notify(hide_switch, "notify::active", move || {
            let hidden = gtk_switch_get_active(hide_switch) != 0;
            set_label_text(hide_state_lbl, if hidden { "Hidden" } else { "Visible" });
            h.settings.borrow_mut().dock_hidden = hidden;
            apply_desktop_settings(&h);
        });
    }

    // 1. Dock Size (15px - 64px)
    let sz_row = new_box(GTK_ORIENTATION_HORIZONTAL, 12, &[]);
    let sz_lbl = new_label("Dock Size", &["finder-name-btn"]);
    gtk_label_set_xalign(sz_lbl, 0.0);
    gtk_widget_set_size_request(sz_lbl, 200, -1);
    let sz_scale = gtk_scale_new_with_range(GTK_ORIENTATION_HORIZONTAL, 15.0, 64.0, 1.0);
    gtk_widget_set_hexpand(sz_scale, 1);
    gtk_range_set_value(sz_scale, cur.dock_size as f64);
    let sz_val = new_label(&format!("{} px", cur.dock_size), &["app-muted"]);
    gtk_widget_set_size_request(sz_val, 52, -1);
    gtk_label_set_xalign(sz_val, 1.0);
    gtk_box_append(sz_row, sz_lbl);
    gtk_box_append(sz_row, sz_scale);
    gtk_box_append(sz_row, sz_val);
    gtk_box_append(dock_card, sz_row);

    {
        let h = Rc::clone(&handles);
        connect_signal(sz_scale, "value-changed", move || {
            let v = gtk_range_get_value(sz_scale).round() as i32;
            set_label_text(sz_val, &format!("{} px", v));
            h.settings.borrow_mut().dock_size = v;
            apply_desktop_settings(&h);
        });
    }

    // 2. Magnification
    let mag_row = new_box(GTK_ORIENTATION_HORIZONTAL, 12, &[]);
    let mag_lbl = new_label("Magnification", &["finder-name-btn"]);
    gtk_label_set_xalign(mag_lbl, 0.0);
    gtk_widget_set_size_request(mag_lbl, 200, -1);
    let mag_switch = gtk_switch_new();
    gtk_widget_set_valign(mag_switch, GTK_ALIGN_CENTER);
    gtk_switch_set_active(mag_switch, if cur.magnification_enabled { 1 } else { 0 });
    let mag_scale = gtk_scale_new_with_range(GTK_ORIENTATION_HORIZONTAL, 1.0, 1.6, 0.05);
    gtk_widget_set_hexpand(mag_scale, 1);
    gtk_range_set_value(mag_scale, cur.magnification);
    gtk_widget_set_sensitive(mag_scale, if cur.magnification_enabled { 1 } else { 0 });
    let mag_val = new_label(&format!("{:.2}x", cur.magnification), &["app-muted"]);
    gtk_widget_set_size_request(mag_val, 52, -1);
    gtk_label_set_xalign(mag_val, 1.0);
    gtk_box_append(mag_row, mag_lbl);
    gtk_box_append(mag_row, mag_switch);
    gtk_box_append(mag_row, mag_scale);
    gtk_box_append(mag_row, mag_val);
    gtk_box_append(dock_card, mag_row);

    {
        let h = Rc::clone(&handles);
        connect_notify(mag_switch, "notify::active", move || {
            let on = gtk_switch_get_active(mag_switch) != 0;
            gtk_widget_set_sensitive(mag_scale, if on { 1 } else { 0 });
            h.settings.borrow_mut().magnification_enabled = on;
            apply_desktop_settings(&h);
        });
    }
    {
        let h = Rc::clone(&handles);
        connect_signal(mag_scale, "value-changed", move || {
            let v = gtk_range_get_value(mag_scale);
            set_label_text(mag_val, &format!("{:.2}x", v));
            h.settings.borrow_mut().magnification = v;
            apply_desktop_settings(&h);
        });
    }

    // 3. Position on Screen
    let pos_row = new_box(GTK_ORIENTATION_HORIZONTAL, 12, &[]);
    let pos_lbl = new_label("Position on Screen", &["finder-name-btn"]);
    gtk_label_set_xalign(pos_lbl, 0.0);
    gtk_widget_set_size_request(pos_lbl, 200, -1);
    let pos_box = new_box(GTK_ORIENTATION_HORIZONTAL, 6, &[]);
    let pos_opts = [("Bottom", "bottom"), ("Left", "left"), ("Right", "right")];
    let pos_btns: Rc<RefCell<Vec<*mut GtkWidget>>> = Rc::new(RefCell::new(Vec::new()));
    for (label, key) in pos_opts {
        let cls = if cur.dock_position == key { "seg-btn-active" } else { "seg-btn" };
        let b = new_button(label, &[cls]);
        gtk_box_append(pos_box, b);
        pos_btns.borrow_mut().push(b);

        let key_s = key.to_string();
        let h = Rc::clone(&handles);
        let all_b = Rc::clone(&pos_btns);
        connect_signal(b, "clicked", move || {
            for &bi in all_b.borrow().iter() {
                remove_css(bi, "seg-btn-active");
                add_css(bi, "seg-btn");
            }
            remove_css(b, "seg-btn");
            add_css(b, "seg-btn-active");
            h.settings.borrow_mut().dock_position = key_s.clone();
            apply_desktop_settings(&h);
        });
    }
    gtk_box_append(pos_row, pos_lbl);
    gtk_box_append(pos_row, pos_box);
    gtk_box_append(dock_card, pos_row);

    // 4. Minimize Windows Using
    let min_row = new_box(GTK_ORIENTATION_HORIZONTAL, 12, &[]);
    let min_lbl = new_label("Minimize Windows Using", &["finder-name-btn"]);
    gtk_label_set_xalign(min_lbl, 0.0);
    gtk_widget_set_size_request(min_lbl, 200, -1);
    let min_box = new_box(GTK_ORIENTATION_HORIZONTAL, 6, &[]);
    let min_opts = [("Genie Effect", "genie"), ("Scale Effect", "scale")];
    let min_btns: Rc<RefCell<Vec<*mut GtkWidget>>> = Rc::new(RefCell::new(Vec::new()));
    for (label, key) in min_opts {
        let cls = if cur.minimize_effect == key { "seg-btn-active" } else { "seg-btn" };
        let b = new_button(label, &[cls]);
        gtk_box_append(min_box, b);
        min_btns.borrow_mut().push(b);

        let key_s = key.to_string();
        let h = Rc::clone(&handles);
        let all_b = Rc::clone(&min_btns);
        connect_signal(b, "clicked", move || {
            for &bi in all_b.borrow().iter() {
                remove_css(bi, "seg-btn-active");
                add_css(bi, "seg-btn");
            }
            remove_css(b, "seg-btn");
            add_css(b, "seg-btn-active");
            h.settings.borrow_mut().minimize_effect = key_s.clone();
            apply_desktop_settings(&h);
        });
    }
    gtk_box_append(min_row, min_lbl);
    gtk_box_append(min_row, min_box);
    gtk_box_append(dock_card, min_row);

    // 5. Animate Opening Applications
    let anim_row = new_box(GTK_ORIENTATION_HORIZONTAL, 12, &[]);
    let anim_lbl = new_label("Animate Opening Applications", &["finder-name-btn"]);
    gtk_label_set_xalign(anim_lbl, 0.0);
    gtk_widget_set_hexpand(anim_lbl, 1);
    let anim_switch = gtk_switch_new();
    gtk_widget_set_valign(anim_switch, GTK_ALIGN_CENTER);
    gtk_switch_set_active(anim_switch, if cur.animate_opening { 1 } else { 0 });
    gtk_box_append(anim_row, anim_lbl);
    gtk_box_append(anim_row, anim_switch);
    gtk_box_append(dock_card, anim_row);

    {
        let h = Rc::clone(&handles);
        connect_notify(anim_switch, "notify::active", move || {
            let on = gtk_switch_get_active(anim_switch) != 0;
            h.settings.borrow_mut().animate_opening = on;
            apply_desktop_settings(&h);
        });
    }

    gtk_box_append(page, dock_card);
    gtk_scrolled_window_set_child(scroller, page);
    scroller
}

// ============================================================================
// Page 2: Displays (Hardware Resolution via xrandr) & Remote Desktop (VNC)
// ============================================================================
fn probe_xrandr_output() -> (String, String, Vec<String>) {
    let mut out_name = String::new();
    let mut cur_mode = String::new();
    let mut modes: Vec<String> = Vec::new();
    if let Ok(out) = Command::new("xrandr").arg("--query").output() {
        let txt = String::from_utf8_lossy(&out.stdout);
        let mut in_connected = false;
        for line in txt.lines() {
            if !line.starts_with(' ') && !line.starts_with('\t') {
                if line.contains(" connected") && out_name.is_empty() {
                    out_name = line.split_whitespace().next().unwrap_or("VNC-0").to_string();
                    in_connected = true;
                } else if in_connected {
                    break;
                }
            } else if in_connected {
                let mut parts = line.split_whitespace();
                if let Some(m) = parts.next() {
                    if m.contains('x') && m.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
                        if line.contains('*') {
                            cur_mode = m.to_string();
                        }
                        if !modes.contains(&m.to_string()) {
                            modes.push(m.to_string());
                        }
                    }
                }
            }
        }
    }
    if out_name.is_empty() {
        out_name = "VNC-0".to_string();
    }
    if modes.is_empty() {
        modes = vec![
            "1920x1080".into(),
            "1680x1050".into(),
            "1600x900".into(),
            "1440x900".into(),
            "1366x768".into(),
            "1280x720".into(),
        ];
    }
    (out_name, cur_mode, modes)
}

fn is_systemd_active(unit: &str) -> bool {
    Command::new("systemctl")
        .args(["is-active", "--quiet", unit])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn read_primary_ipv4() -> String {
    if let Ok(out) = Command::new("ip").args(["-4", "-o", "addr", "show"]).output() {
        let txt = String::from_utf8_lossy(&out.stdout);
        for line in txt.lines() {
            let p: Vec<&str> = line.split_whitespace().collect();
            if p.len() >= 4 && p[1] != "lo" {
                return p[3].split('/').next().unwrap_or("127.0.0.1").to_string();
            }
        }
    }
    "127.0.0.1".to_string()
}

unsafe fn build_display_page(handles: Rc<DesktopHandles>) -> *mut GtkWidget {
    let scroller = gtk_scrolled_window_new();
    gtk_scrolled_window_set_policy(scroller, GTK_POLICY_NEVER, GTK_POLICY_AUTOMATIC);
    gtk_widget_set_hexpand(scroller, 1);
    let page = new_box(GTK_ORIENTATION_VERTICAL, 14, &[]);
    set_margin_all(page, 22);

    let title = new_label("Displays & Remote Desktop", &["app-heading"]);
    gtk_label_set_xalign(title, 0.0);
    gtk_box_append(page, title);

    // =========================================================================
    // Card 1: Hardware Display Output & Resolution (xrandr)
    // =========================================================================
    let disp_card = new_box(GTK_ORIENTATION_VERTICAL, 10, &["app-card"]);
    let hdr_row = new_box(GTK_ORIENTATION_HORIZONTAL, 8, &[]);
    let disp_hdr = new_label("🖥️ HARDWARE DISPLAY & RESOLUTION", &["app-section-title"]);
    gtk_label_set_xalign(disp_hdr, 0.0);
    gtk_widget_set_hexpand(disp_hdr, 1);
    let btn_refresh_disp = new_button("🔄 Refresh Outputs", &["toolbar-btn"]);
    gtk_box_append(hdr_row, disp_hdr);
    gtk_box_append(hdr_row, btn_refresh_disp);
    gtk_box_append(disp_card, hdr_row);

    let disp_info = new_label("", &["app-muted"]);
    gtk_label_set_xalign(disp_info, 0.0);
    gtk_label_set_wrap(disp_info, 1);
    gtk_box_append(disp_card, disp_info);

    // Compact Resolution Dropdown Selector (zero black X11 popup artifacts)
    let dd_row = new_box(GTK_ORIENTATION_HORIZONTAL, 10, &[]);
    let dd_lbl = new_label("Resolution", &["finder-name-btn"]);
    gtk_label_set_xalign(dd_lbl, 0.0);
    gtk_widget_set_size_request(dd_lbl, 110, -1);

    let dd_toggle_btn = new_button("1920 × 1080   ▾", &["toolbar-btn"]);
    gtk_widget_set_hexpand(dd_toggle_btn, 1);
    gtk_box_append(dd_row, dd_lbl);
    gtk_box_append(dd_row, dd_toggle_btn);
    gtk_box_append(disp_card, dd_row);

    let dd_scroll = gtk_scrolled_window_new();
    gtk_scrolled_window_set_policy(dd_scroll, GTK_POLICY_NEVER, GTK_POLICY_AUTOMATIC);
    gtk_widget_set_size_request(dd_scroll, -1, 180);
    add_css(dd_scroll, "bijios-dropdown-menu");
    gtk_widget_set_visible(dd_scroll, 0);

    let dd_list = new_box(GTK_ORIENTATION_VERTICAL, 2, &[]);
    gtk_scrolled_window_set_child(dd_scroll, dd_list);
    gtk_box_append(disp_card, dd_scroll);

    connect_signal(dd_toggle_btn, "clicked", move || {
        let vis = gtk_widget_get_visible(dd_scroll) != 0;
        gtk_widget_set_visible(dd_scroll, if vis { 0 } else { 1 });
    });

    let apply_resolution: Rc<dyn Fn(u32, u32, Rc<dyn Fn()>)> = {
        let h = Rc::clone(&handles);
        Rc::new(move |w: u32, ht: u32, on_done: Rc<dyn Fn()>| {
            let (output, _, _) = probe_xrandr_output();
            let mode_name = format!("{}x{}", w, ht);
            let script = format!(
                "if ! xrandr --output {out} --mode {m} 2>/dev/null; then \
                   LINE=$(cvt {w} {ht} 60 | awk '/Modeline/{{$1=\"\"; sub(/^ /, \"\"); print}}'); \
                   NAME=$(echo \"$LINE\" | awk '{{print $1}}'); \
                   xrandr --newmode $LINE 2>/dev/null || true; \
                   xrandr --addmode {out} $NAME 2>/dev/null || true; \
                   xrandr --output {out} --mode $NAME 2>/dev/null || xrandr --fb {m}; \
                 fi",
                out = output,
                m = mode_name,
                w = w,
                ht = ht
            );
            let _ = Command::new("sh").args(["-c", &script]).status();
            let desk = h.desktop_win;
            timeout_add_ms(140, move || {
                set_x11_desktop_window(desk);
                on_done();
                false
            });
        })
    };

    let populate_modes: Rc<RefCell<Box<dyn Fn()>>> = Rc::new(RefCell::new(Box::new(|| {})));
    let pop_clone = Rc::clone(&populate_modes);
    {
        let ar = Rc::clone(&apply_resolution);
        *populate_modes.borrow_mut() = Box::new(move || {
            let (out_name, cur_mode, modes) = probe_xrandr_output();
            let (cw, ch) = get_x11_screen_size();
            let active_str = if !cur_mode.is_empty() {
                cur_mode.clone()
            } else {
                format!("{}x{}", cw, ch)
            };
            let pretty_active = active_str.replace('x', " × ");
            set_label_text(
                disp_info,
                &format!(
                    "Detected Output: {} (Connected)  •  Active: {}  •  {} mode(s) available",
                    out_name,
                    pretty_active,
                    modes.len()
                ),
            );
            let dd_txt = cstr(&format!("🖥️  {}   ▾", pretty_active));
            gtk_button_set_label(dd_toggle_btn, dd_txt.as_ptr());

            clear_box_children(dd_list);
            for mode in modes {
                let is_cur = mode == active_str;
                let pretty = mode.replace('x', " × ");
                let item_lbl = if is_cur {
                    format!("✓  {} (Active)", pretty)
                } else {
                    format!("    {}", pretty)
                };
                let b = new_left_button(
                    "",
                    &item_lbl,
                    if is_cur { &["sidebar-btn-active"] } else { &["sidebar-btn"] },
                );
                gtk_box_append(dd_list, b);

                if let Some((ws, hs)) = mode.split_once('x') {
                    if let (Ok(w), Ok(ht)) = (ws.parse::<u32>(), hs.parse::<u32>()) {
                        let ar_c = Rc::clone(&ar);
                        let pc = Rc::clone(&pop_clone);
                        connect_signal(b, "clicked", move || {
                            gtk_widget_set_visible(dd_scroll, 0);
                            let pc2 = Rc::clone(&pc);
                            ar_c(w, ht, Rc::new(move || (pc2.borrow())()));
                        });
                    }
                }
            }
        });
    }

    {
        let pc = Rc::clone(&populate_modes);
        connect_signal(btn_refresh_disp, "clicked", move || {
            (pc.borrow())();
        });
    }

    // Custom Hardware Resolution Row
    let custom_hdr = new_label("CUSTOM RESOLUTION", &["kv-key"]);
    gtk_label_set_xalign(custom_hdr, 0.0);
    gtk_box_append(disp_card, custom_hdr);

    let custom_row = new_box(GTK_ORIENTATION_HORIZONTAL, 8, &[]);
    let w_in = gtk_entry_new();
    let ph_w = cstr("Width (e.g. 1920)");
    gtk_entry_set_placeholder_text(w_in, ph_w.as_ptr());
    gtk_widget_set_size_request(w_in, 136, -1);
    let x_lbl = new_label("×", &["kv-key"]);
    let h_in = gtk_entry_new();
    let ph_h = cstr("Height (e.g. 1080)");
    gtk_entry_set_placeholder_text(h_in, ph_h.as_ptr());
    gtk_widget_set_size_request(h_in, 136, -1);
    let btn_custom_res = new_button("Apply Custom", &["primary-btn"]);
    {
        let ar = Rc::clone(&apply_resolution);
        let pc = Rc::clone(&populate_modes);
        connect_signal(btn_custom_res, "clicked", move || {
            let pw = get_entry_text(w_in).trim().parse::<u32>().unwrap_or(0);
            let ph = get_entry_text(h_in).trim().parse::<u32>().unwrap_or(0);
            if (640..=7680).contains(&pw) && (480..=4320).contains(&ph) {
                let pc2 = Rc::clone(&pc);
                ar(pw, ph, Rc::new(move || (pc2.borrow())()));
            } else {
                set_label_text(disp_info, "❌ Enter a valid resolution between 640×480 and 7680×4320.");
            }
        });
    }
    gtk_box_append(custom_row, w_in);
    gtk_box_append(custom_row, x_lbl);
    gtk_box_append(custom_row, h_in);
    gtk_box_append(custom_row, btn_custom_res);
    gtk_box_append(disp_card, custom_row);
    gtk_box_append(page, disp_card);

    (populate_modes.borrow())();

    // =========================================================================
    // Card 2: Remote Desktop (VNC Servers Enable / Disable & Password)
    // =========================================================================
    let vnc_card = new_box(GTK_ORIENTATION_VERTICAL, 12, &["app-card"]);
    let vnc_hdr = new_label("📡 REMOTE DESKTOP (VNC SERVERS)", &["app-section-title"]);
    gtk_label_set_xalign(vnc_hdr, 0.0);
    let host_ip = read_primary_ipv4();
    let vnc_status = new_label(
        &format!("Host IPv4: {}  •  Toggle installed VNC services on or off below.", host_ip),
        &["app-muted"],
    );
    gtk_label_set_xalign(vnc_status, 0.0);
    gtk_label_set_wrap(vnc_status, 1);
    gtk_box_append(vnc_card, vnc_hdr);
    gtk_box_append(vnc_card, vnc_status);

    let add_vnc_service_row = |unit: &'static str, title_txt: &str, desc_txt: &str, port: u16| {
        let row = new_box(GTK_ORIENTATION_HORIZONTAL, 12, &["act-proc-row"]);
        let col = new_box(GTK_ORIENTATION_VERTICAL, 2, &[]);
        gtk_widget_set_hexpand(col, 1);

        let top_r = new_box(GTK_ORIENTATION_HORIZONTAL, 8, &[]);
        let t_lbl = new_label(title_txt, &["act-proc-name"]);
        gtk_label_set_xalign(t_lbl, 0.0);
        let active_now = is_systemd_active(unit);
        let badge = new_label(
            if active_now { "Active" } else { "Stopped" },
            if active_now { &["act-state-run"] } else { &["act-state-sleep"] },
        );
        gtk_box_append(top_r, t_lbl);
        gtk_box_append(top_r, badge);

        let sub = new_label(
            &format!("{}  •  Connect: {}:{}", desc_txt, host_ip, port),
            &["finder-meta-lbl"],
        );
        gtk_label_set_xalign(sub, 0.0);
        gtk_box_append(col, top_r);
        gtk_box_append(col, sub);

        let btn_restart = new_button("🔄 Restart", &["toolbar-btn"]);
        gtk_widget_set_valign(btn_restart, GTK_ALIGN_CENTER);

        let sw = gtk_switch_new();
        gtk_widget_set_valign(sw, GTK_ALIGN_CENTER);
        gtk_switch_set_active(sw, if active_now { 1 } else { 0 });

        let update_badge = move |is_on: bool| {
            remove_css(badge, "act-state-run");
            remove_css(badge, "act-state-sleep");
            if is_on {
                add_css(badge, "act-state-run");
                set_label_text(badge, "Active");
            } else {
                add_css(badge, "act-state-sleep");
                set_label_text(badge, "Stopped");
            }
        };

        let sw_guard = Rc::new(std::cell::Cell::new(false));
        {
            let guard = Rc::clone(&sw_guard);
            connect_notify(sw, "notify::active", move || {
                if guard.get() {
                    return;
                }
                let want_on = gtk_switch_get_active(sw) != 0;
                let cmd = if want_on {
                    format!("systemctl enable --now {}", unit)
                } else {
                    format!("systemctl disable --now {}", unit)
                };
                if run_pkexec_sh(&cmd) {
                    let real_on = is_systemd_active(unit);
                    update_badge(real_on);
                    set_label_text(
                        vnc_status,
                        &format!(
                            "✅ {} is now {}.",
                            unit,
                            if real_on { "ENABLED & RUNNING" } else { "DISABLED & STOPPED" }
                        ),
                    );
                } else {
                    let real_on = is_systemd_active(unit);
                    update_badge(real_on);
                    guard.set(true);
                    gtk_switch_set_active(sw, if real_on { 1 } else { 0 });
                    guard.set(false);
                    set_label_text(
                        vnc_status,
                        &format!("❌ Failed to toggle {} (admin permission required).", unit),
                    );
                }
            });
        }

        {
            let guard = Rc::clone(&sw_guard);
            connect_signal(btn_restart, "clicked", move || {
                let cmd = format!("systemctl restart {}", unit);
                if run_pkexec_sh(&cmd) {
                    let real_on = is_systemd_active(unit);
                    update_badge(real_on);
                    guard.set(true);
                    gtk_switch_set_active(sw, if real_on { 1 } else { 0 });
                    guard.set(false);
                    set_label_text(vnc_status, &format!("✅ Restarted {}.", unit));
                } else {
                    set_label_text(vnc_status, &format!("❌ Failed to restart {}.", unit));
                }
            });
        }

        gtk_box_append(row, col);
        gtk_box_append(row, btn_restart);
        gtk_box_append(row, sw);
        gtk_box_append(vnc_card, row);
    };

    add_vnc_service_row(
        "bijios-tigervnc.service",
        "TigerVNC Server (Dynamic Auto-Resize)",
        "Virtual Desktop :1 (RFB ExtendedDesktopSize)",
        5901,
    );
    add_vnc_service_row(
        "x11vnc.service",
        "X11VNC Server (Live Console Mirror)",
        "Mirrors physical/console Display :0",
        5900,
    );

    // Update VNC Password Row
    let pw_hdr = new_label("VNC ACCESS PASSWORD (MAX 8 CHARS)", &["kv-key"]);
    gtk_label_set_xalign(pw_hdr, 0.0);
    gtk_box_append(vnc_card, pw_hdr);

    let pw_row = new_box(GTK_ORIENTATION_HORIZONTAL, 8, &[]);
    let vnc_pw_in = gtk_entry_new();
    gtk_widget_set_hexpand(vnc_pw_in, 1);
    gtk_entry_set_visibility(vnc_pw_in, 0);
    let ph_vpw = cstr("New VNC password (6–8 characters)");
    gtk_entry_set_placeholder_text(vnc_pw_in, ph_vpw.as_ptr());
    let btn_vnc_pw = new_button("🔑 Update VNC Password", &["primary-btn"]);
    connect_signal(btn_vnc_pw, "clicked", move || {
        let pw = get_entry_text(vnc_pw_in);
        if pw.len() < 4 || pw.len() > 8 {
            set_label_text(vnc_status, "❌ VNC password must be 4 to 8 characters.");
            return;
        }
        let user = std::env::var("USER").unwrap_or_else(|_| "dionipe".to_string());
        let qpw = shell_quote(&pw);
        let qusr = shell_quote(&user);
        let script = format!(
            "mkdir -p /home/{u}/.vnc && printf '%s\\n' {p} | tigervncpasswd -f > /home/{u}/.vnc/passwd && chmod 600 /home/{u}/.vnc/passwd && chown {u}:{u} /home/{u}/.vnc/passwd; \
             x11vnc -storepasswd {p} /etc/x11vnc.passwd && chmod 600 /etc/x11vnc.passwd; \
             systemctl is-active --quiet bijios-tigervnc.service && systemctl restart bijios-tigervnc.service || true; \
             systemctl is-active --quiet x11vnc.service && systemctl restart x11vnc.service || true",
            u = qusr.trim_matches('\''),
            p = qpw
        );
        if run_pkexec_sh(&script) {
            set_entry_text(vnc_pw_in, "");
            set_label_text(vnc_status, "✅ VNC password updated for TigerVNC & X11VNC.");
        } else {
            set_label_text(vnc_status, "❌ Failed to update VNC password.");
        }
    });
    gtk_box_append(pw_row, vnc_pw_in);
    gtk_box_append(pw_row, btn_vnc_pw);
    gtk_box_append(vnc_card, pw_row);

    gtk_box_append(page, vnc_card);
    gtk_scrolled_window_set_child(scroller, page);
    scroller
}

// ============================================================================
// Page 2: Wallpaper (Nature Previews + Dynamic Wallpaper)
// ============================================================================
unsafe fn build_wallpaper_page(handles: Rc<DesktopHandles>) -> *mut GtkWidget {
    let scroller = gtk_scrolled_window_new();
    gtk_scrolled_window_set_policy(scroller, GTK_POLICY_NEVER, GTK_POLICY_AUTOMATIC);
    gtk_widget_set_hexpand(scroller, 1);
    let page = new_box(GTK_ORIENTATION_VERTICAL, 14, &[]);
    set_margin_all(page, 22);

    let title = new_label("Wallpaper", &["app-heading"]);
    gtk_label_set_xalign(title, 0.0);
    gtk_box_append(page, title);

    let cur = handles.settings.borrow().clone();

    // Active Preview Card
    let preview_card = new_box(GTK_ORIENTATION_HORIZONTAL, 16, &["app-card"]);
    let preview_holder = new_box(GTK_ORIENTATION_VERTICAL, 0, &[]);
    let active_file = resolve_effective_wallpaper_file(&cur);
    gtk_box_append(preview_holder, new_wallpaper_preview(active_file, 220, 132));

    let prev_info = new_box(GTK_ORIENTATION_VERTICAL, 8, &[]);
    gtk_widget_set_hexpand(prev_info, 1);
    gtk_widget_set_valign(prev_info, GTK_ALIGN_CENTER);
    let prev_title = new_label(
        if cur.wallpaper == "dynamic-padi" {
            "Dynamic — Padi Horizon"
        } else {
            &cur.wallpaper
        },
        &["app-heading"],
    );
    gtk_label_set_xalign(prev_title, 0.0);
    let prev_sub = new_label(
        "Dynamic Wallpaper shifts automatically between Dawn, Emerald Terraces, Golden Harvest Sunset, and Forest Night.",
        &["app-muted"],
    );
    gtk_label_set_xalign(prev_sub, 0.0);
    gtk_label_set_wrap(prev_sub, 1);

    // Dynamic Phase Selector
    let phase_row = new_box(GTK_ORIENTATION_HORIZONTAL, 6, &[]);
    let phases = [
        ("Auto (Time)", "auto"),
        ("Dawn", "dawn"),
        ("Day", "day"),
        ("Sunset", "sunset"),
        ("Night", "night"),
    ];
    let phase_btns: Rc<RefCell<Vec<*mut GtkWidget>>> = Rc::new(RefCell::new(Vec::new()));

    let update_preview: Rc<dyn Fn()> = {
        let h = Rc::clone(&handles);
        Rc::new(move || {
            let s = h.settings.borrow().clone();
            let f = resolve_effective_wallpaper_file(&s);
            clear_box_children(preview_holder);
            gtk_box_append(preview_holder, new_wallpaper_preview(f, 220, 132));
            let label = match s.wallpaper.as_str() {
                "dynamic-padi" => format!("Dynamic — Padi Horizon ({})", s.dynamic_phase),
                "padi-terraces.svg" => "Padi Terraces (Emerald)".to_string(),
                "golden-harvest.svg" => "Golden Harvest Sunset".to_string(),
                "hutan-malam.svg" => "Hutan Malam (Forest Night)".to_string(),
                "sequoia-mist.svg" => "Sequoia Mist (Alpine)".to_string(),
                "coastal-dawn.svg" => "Coastal Dawn (Archipelago)".to_string(),
                other => other.to_string(),
            };
            set_label_text(prev_title, &label);
        })
    };

    for (lbl, key) in phases {
        let cls = if cur.dynamic_phase == key { "seg-btn-active" } else { "seg-btn" };
        let b = new_button(lbl, &[cls]);
        gtk_box_append(phase_row, b);
        phase_btns.borrow_mut().push(b);

        let key_s = key.to_string();
        let h = Rc::clone(&handles);
        let all_p = Rc::clone(&phase_btns);
        let up = Rc::clone(&update_preview);
        connect_signal(b, "clicked", move || {
            for &bi in all_p.borrow().iter() {
                remove_css(bi, "seg-btn-active");
                add_css(bi, "seg-btn");
            }
            remove_css(b, "seg-btn");
            add_css(b, "seg-btn-active");
            {
                let mut st = h.settings.borrow_mut();
                st.wallpaper = "dynamic-padi".to_string();
                st.dynamic_phase = key_s.clone();
            }
            apply_desktop_settings(&h);
            up();
        });
    }

    gtk_box_append(prev_info, prev_title);
    gtk_box_append(prev_info, prev_sub);
    gtk_box_append(prev_info, phase_row);
    gtk_box_append(preview_card, preview_holder);
    gtk_box_append(preview_card, prev_info);
    gtk_box_append(page, preview_card);

    // Nature & Dynamic Wallpaper Gallery with Visual Image Previews
    let gal_hdr = new_label("NATURE & DYNAMIC WALLPAPERS", &["app-section-title"]);
    gtk_label_set_xalign(gal_hdr, 0.0);
    gtk_box_append(page, gal_hdr);

    let flow = gtk_flow_box_new();
    gtk_flow_box_set_max_children_per_line(flow, 3);
    gtk_flow_box_set_min_children_per_line(flow, 2);
    gtk_flow_box_set_row_spacing(flow, 12);
    gtk_flow_box_set_column_spacing(flow, 12);
    gtk_flow_box_set_selection_mode(flow, 0);

    let items = [
        ("dynamic-padi", "padi-terraces.svg", "Padi Horizon", "Dynamic"),
        ("padi-terraces.svg", "padi-terraces.svg", "Padi Terraces", "Nature"),
        ("golden-harvest.svg", "golden-harvest.svg", "Golden Harvest", "Sunset"),
        ("hutan-malam.svg", "hutan-malam.svg", "Hutan Malam", "Night"),
        ("sequoia-mist.svg", "sequoia-mist.svg", "Sequoia Mist", "Alpine"),
        ("coastal-dawn.svg", "coastal-dawn.svg", "Coastal Dawn", "Ocean"),
    ];

    let wp_tiles: Rc<RefCell<Vec<*mut GtkWidget>>> = Rc::new(RefCell::new(Vec::new()));
    for (id, preview_svg, name, tag) in items {
        let is_active = cur.wallpaper == id;
        let btn = gtk_button_new();
        add_css(btn, if is_active { "wp-tile-active" } else { "wp-tile" });

        let col = new_box(GTK_ORIENTATION_VERTICAL, 6, &[]);
        gtk_box_append(col, new_wallpaper_preview(preview_svg, 195, 112));

        let cap_row = new_box(GTK_ORIENTATION_HORIZONTAL, 6, &[]);
        let n_lbl = new_label(name, &["finder-name-btn"]);
        gtk_label_set_xalign(n_lbl, 0.0);
        gtk_widget_set_hexpand(n_lbl, 1);
        let t_lbl = new_label(tag, &["wp-badge"]);
        gtk_box_append(cap_row, n_lbl);
        gtk_box_append(cap_row, t_lbl);
        gtk_box_append(col, cap_row);

        gtk_button_set_child(btn, col);
        gtk_flow_box_append(flow, btn);
        wp_tiles.borrow_mut().push(btn);

        let id_s = id.to_string();
        let h = Rc::clone(&handles);
        let all_t = Rc::clone(&wp_tiles);
        let up = Rc::clone(&update_preview);
        connect_signal(btn, "clicked", move || {
            for &ti in all_t.borrow().iter() {
                remove_css(ti, "wp-tile-active");
                add_css(ti, "wp-tile");
            }
            remove_css(btn, "wp-tile");
            add_css(btn, "wp-tile-active");
            h.settings.borrow_mut().wallpaper = id_s.clone();
            apply_desktop_settings(&h);
            up();
        });
    }

    gtk_box_append(page, flow);
    gtk_scrolled_window_set_child(scroller, page);
    scroller
}

// ============================================================================
// Page 3: General (About, Software Update, Storage, System)
// ============================================================================
unsafe fn build_general_page(
    app: *mut GtkApplication,
    settings_win: *mut GtkWidget,
    on_lock: Rc<dyn Fn()>,
) -> *mut GtkWidget {
    let scroller = gtk_scrolled_window_new();
    gtk_scrolled_window_set_policy(scroller, GTK_POLICY_NEVER, GTK_POLICY_AUTOMATIC);
    gtk_widget_set_hexpand(scroller, 1);
    let page = new_box(GTK_ORIENTATION_VERTICAL, 14, &[]);
    set_margin_all(page, 22);

    let title = new_label("General", &["app-heading"]);
    gtk_label_set_xalign(title, 0.0);
    gtk_box_append(page, title);

    // 1. About Card (BijiOS OS Logo + Detailed OS Info + Host IDVE / KVM Logo)
    let about_card = new_box(GTK_ORIENTATION_VERTICAL, 10, &["app-card"]);

    let about_top = new_box(GTK_ORIENTATION_HORIZONTAL, 14, &[]);
    let logo = new_logo_image(56, true);
    gtk_widget_set_valign(logo, GTK_ALIGN_CENTER);

    let about_col = new_box(GTK_ORIENTATION_VERTICAL, 3, &[]);
    gtk_widget_set_hexpand(about_col, 1);
    let a_hdr = new_label("ABOUT BIJIOS & OPERATING SYSTEM", &["app-section-title"]);
    gtk_label_set_xalign(a_hdr, 0.0);
    let a_name = new_label("BijiOS Desktop — Version 1.0.0 (Codename: Padi)", &["finder-name-btn"]);
    gtk_label_set_xalign(a_name, 0.0);
    gtk_box_append(about_col, a_hdr);
    gtk_box_append(about_col, a_name);

    let idve_box = new_box(GTK_ORIENTATION_VERTICAL, 2, &[]);
    gtk_widget_set_valign(idve_box, GTK_ALIGN_CENTER);
    gtk_widget_set_halign(idve_box, GTK_ALIGN_END);
    gtk_box_append(idve_box, new_idve_logo(112, 34));
    let idve_lbl = new_label("IDVE / KVM Host", &["finder-meta-lbl"]);
    gtk_label_set_xalign(idve_lbl, 1.0);
    gtk_box_append(idve_box, idve_lbl);

    gtk_box_append(about_top, logo);
    gtk_box_append(about_top, about_col);
    gtk_box_append(about_top, idve_box);
    gtk_box_append(about_card, about_top);

    let hostname = fs::read_to_string("/etc/hostname").unwrap_or_else(|_| "BijiOS".into());
    let (used_mb, total_mb, _) = read_mem_usage();
    let (arch, kernel) = crate::native_de::apps::read_arch_and_kernel();

    let kv_rows: [(&str, String); 4] = [
        ("Base OS", format!("{} ({})", crate::native_de::apps::read_os_pretty_name(), arch)),
        ("Kernel & Host", format!("{}  •  {}", kernel, hostname.trim())),
        ("CPU & Memory", format!("{}  •  {} / {} MB RAM", crate::native_de::apps::read_cpu_summary(), used_mb, total_mb)),
        ("Hypervisor", crate::native_de::apps::read_virt_host_info()),
    ];
    for (k, v) in kv_rows {
        let row = new_box(GTK_ORIENTATION_HORIZONTAL, 10, &[]);
        let kl = new_label(k, &["kv-key"]);
        gtk_label_set_xalign(kl, 0.0);
        gtk_widget_set_size_request(kl, 120, -1);
        let vl = new_label(&v, &["kv-val"]);
        gtk_label_set_xalign(vl, 0.0);
        gtk_label_set_wrap(vl, 1);
        gtk_widget_set_hexpand(vl, 1);
        gtk_box_append(row, kl);
        gtk_box_append(row, vl);
        gtk_box_append(about_card, row);
    }

    let links_row = new_box(GTK_ORIENTATION_HORIZONTAL, 8, &[]);
    gtk_box_append(
        links_row,
        new_link_button("🐙 github.com/dionipe/BijiOS ↗", "https://github.com/dionipe/BijiOS", &["link-pill"]),
    );
    gtk_box_append(
        links_row,
        new_link_button("👤 @dionipe ↗", "https://github.com/dionipe", &["link-pill"]),
    );
    gtk_box_append(
        links_row,
        new_link_button("☁️ idve.cloud ↗", "https://idve.cloud/", &["link-pill"]),
    );
    gtk_box_append(about_card, links_row);
    gtk_box_append(page, about_card);

    // 2. Software Update Card
    let upd_card = new_box(GTK_ORIENTATION_VERTICAL, 8, &["app-card"]);
    let u_hdr = new_label("SOFTWARE UPDATE", &["app-section-title"]);
    gtk_label_set_xalign(u_hdr, 0.0);
    let u_status = new_label("BijiOS 1.0.0 (Padi) — Ready to check Debian APT updates.", &["app-muted"]);
    gtk_label_set_xalign(u_status, 0.0);
    let u_row = new_box(GTK_ORIENTATION_HORIZONTAL, 8, &[]);
    let btn_check = new_button("🔄 Check for Updates", &["toolbar-btn"]);
    let btn_upgrade = new_button("⬆ Install Updates", &["primary-btn"]);
    gtk_box_append(u_row, btn_check);
    gtk_box_append(u_row, btn_upgrade);
    gtk_box_append(upd_card, u_hdr);
    gtk_box_append(upd_card, u_status);
    gtk_box_append(upd_card, u_row);
    gtk_box_append(page, upd_card);

    connect_signal(btn_check, "clicked", move || {
        set_label_text(u_status, "Checking APT package repository...");
        let count = Command::new("apt")
            .args(["list", "--upgradable"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.lines().filter(|l| l.contains('/')).count())
            .unwrap_or(0);
        if count == 0 {
            set_label_text(u_status, "✅ BijiOS 1.0.0 (Padi) and all packages are up to date.");
        } else {
            set_label_text(
                u_status,
                &format!("📦 {} Debian package update(s) available.", count),
            );
        }
    });
    connect_signal(btn_upgrade, "clicked", move || {
        set_label_text(u_status, "Running system upgrade via APT...");
        std::thread::spawn(|| {
            let _ = run_pkexec_sh("apt-get update && apt-get upgrade -y");
        });
    });

    // 3. Storage Card
    let stor_card = new_box(GTK_ORIENTATION_VERTICAL, 8, &["app-card"]);
    let s_hdr = new_label("STORAGE", &["app-section-title"]);
    gtk_label_set_xalign(s_hdr, 0.0);
    let (disk_str, disk_pct) = read_storage_info();
    let s_lbl = new_label(&disk_str, &["app-muted"]);
    gtk_label_set_xalign(s_lbl, 0.0);
    let s_bar = gtk_progress_bar_new();
    gtk_progress_bar_set_fraction(s_bar, disk_pct);
    let btn_akar = new_button("📁 Open Akar - File Manager", &["toolbar-btn"]);
    gtk_widget_set_halign(btn_akar, GTK_ALIGN_START);
    connect_signal(btn_akar, "clicked", move || open_finder(app));

    gtk_box_append(stor_card, s_hdr);
    gtk_box_append(stor_card, s_lbl);
    gtk_box_append(stor_card, s_bar);
    gtk_box_append(stor_card, btn_akar);
    gtk_box_append(page, stor_card);

    // 4. System & Power Card
    let sys_card = new_box(GTK_ORIENTATION_VERTICAL, 10, &["app-card"]);
    let sy_hdr = new_label("SYSTEM & POWER", &["app-section-title"]);
    gtk_label_set_xalign(sy_hdr, 0.0);
    let sy_meta = new_label(
        &format!(
            "Uptime: {}  •  Display: X11 + Openbox + xcompmgr  •  Runtime: Pure Rust + GTK4",
            read_uptime_str()
        ),
        &["app-muted"],
    );
    gtk_label_set_xalign(sy_meta, 0.0);

    let pwr_row = new_box(GTK_ORIENTATION_HORIZONTAL, 8, &[]);
    let btn_lock = new_button("🔒 Lock Screen", &["toolbar-btn"]);
    let btn_logout = new_button("🚪 Log Out", &["toolbar-btn"]);
    let btn_sleep = new_button("💤 Sleep", &["toolbar-btn"]);
    let btn_reboot = new_button("🔄 Restart", &["accent-gold-btn"]);
    let btn_off = new_button("⏻ Shut Down", &["danger-btn"]);
    gtk_box_append(pwr_row, btn_lock);
    gtk_box_append(pwr_row, btn_logout);
    gtk_box_append(pwr_row, btn_sleep);
    gtk_box_append(pwr_row, btn_reboot);
    gtk_box_append(pwr_row, btn_off);

    {
        let ol = Rc::clone(&on_lock);
        connect_signal(btn_lock, "clicked", move || {
            gtk_window_close(settings_win);
            ol();
        });
    }
    connect_signal(btn_logout, "clicked", || {
        crate::native_de::trigger_power("logout");
    });
    connect_signal(btn_sleep, "clicked", || {
        let _ = Command::new("systemctl").arg("suspend").spawn();
    });
    connect_signal(btn_reboot, "clicked", || {
        let _ = Command::new("systemctl").arg("reboot").spawn();
    });
    connect_signal(btn_off, "clicked", || {
        let _ = Command::new("systemctl").arg("poweroff").spawn();
    });

    gtk_box_append(sys_card, sy_hdr);
    gtk_box_append(sys_card, sy_meta);
    gtk_box_append(sys_card, pwr_row);
    gtk_box_append(page, sys_card);

    gtk_scrolled_window_set_child(scroller, page);
    scroller
}

// ============================================================================
// Page 4: Users & Groups (Profile, Custom Avatar, Add/Edit/Password, Groups)
// ============================================================================
fn collect_local_images(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    let Ok(rd) = fs::read_dir(dir) else { return out };
    for ent in rd.flatten() {
        let p = ent.path();
        if p.is_file() {
            if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
                let e = ext.to_lowercase();
                if matches!(e.as_str(), "png" | "jpg" | "jpeg" | "webp" | "svg") {
                    out.push(p);
                }
            }
        }
    }
    out.sort();
    out
}

fn persist_custom_avatar_image(src: &std::path::Path) -> Option<String> {
    let ext = src
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_lowercase())
        .filter(|e| matches!(e.as_str(), "png" | "jpg" | "jpeg" | "webp" | "svg"))
        .unwrap_or_else(|| "png".to_string());
    let dest_dir = crate::native_de::apps::home_dir()
        .join(".config")
        .join("bijios-desktop");
    let _ = fs::create_dir_all(&dest_dir);
    let dest = dest_dir.join(format!("avatar.{}", ext));
    if src != dest {
        fs::copy(src, &dest).ok()?;
    }
    Some(format!("file:{}", dest.to_string_lossy()))
}

unsafe fn open_avatar_picker_dialog(
    app: *mut GtkApplication,
    handles: Rc<DesktopHandles>,
    username: String,
    profile_avatar_holder: *mut GtkWidget,
    status_lbl: *mut GtkWidget,
) {
    let dlg = create_app_window(app, "🖼️ Choose Profile Avatar", 520, 560);
    gtk_window_set_modal(dlg, 1);

    let scroll = gtk_scrolled_window_new();
    gtk_scrolled_window_set_policy(scroll, GTK_POLICY_NEVER, GTK_POLICY_AUTOMATIC);
    let root = new_box(GTK_ORIENTATION_VERTICAL, 14, &[]);
    set_margin_all(root, 20);

    // Live Preview Header
    let prev_card = new_box(GTK_ORIENTATION_HORIZONTAL, 16, &["app-card"]);
    let preview_slot = new_box(GTK_ORIENTATION_HORIZONTAL, 0, &[]);
    let cur_av = handles.settings.borrow().user_avatar.clone();
    gtk_box_append(preview_slot, new_user_avatar_widget(&cur_av, &username, 76));

    let prev_col = new_box(GTK_ORIENTATION_VERTICAL, 4, &[]);
    gtk_widget_set_valign(prev_col, GTK_ALIGN_CENTER);
    gtk_widget_set_hexpand(prev_col, 1);
    let p_title = new_label(&format!("Profile Avatar — @{}", username), &["app-heading"]);
    gtk_label_set_xalign(p_title, 0.0);
    let p_sub = new_label(
        "Choose a built-in BijiOS avatar or select your own photo (PNG, JPG, WebP, SVG).",
        &["app-muted"],
    );
    gtk_label_set_xalign(p_sub, 0.0);
    gtk_label_set_wrap(p_sub, 1);
    gtk_box_append(prev_col, p_title);
    gtk_box_append(prev_col, p_sub);

    gtk_box_append(prev_card, preview_slot);
    gtk_box_append(prev_card, prev_col);
    gtk_box_append(root, prev_card);

    let dlg_status = new_label("", &["app-muted"]);
    gtk_label_set_xalign(dlg_status, 0.0);

    let apply_avatar: Rc<dyn Fn(String, &str)> = {
        let h = Rc::clone(&handles);
        let u = username.clone();
        Rc::new(move |new_av: String, msg: &str| {
            h.settings.borrow_mut().user_avatar = new_av.clone();
            apply_desktop_settings(&h);

            clear_box_children(preview_slot);
            gtk_box_append(preview_slot, new_user_avatar_widget(&new_av, &u, 76));

            clear_box_children(profile_avatar_holder);
            gtk_box_append(profile_avatar_holder, new_user_avatar_widget(&new_av, &u, 60));

            set_label_text(dlg_status, msg);
            set_label_text(status_lbl, msg);
        })
    };

    // Section 1: Built-in BijiOS Presets
    let pre_card = new_box(GTK_ORIENTATION_VERTICAL, 8, &["app-card"]);
    let pre_hdr = new_label("✨ BUILT-IN BIJIOS AVATARS", &["app-section-title"]);
    gtk_label_set_xalign(pre_hdr, 0.0);
    gtk_box_append(pre_card, pre_hdr);

    let pre_flow = gtk_flow_box_new();
    gtk_flow_box_set_max_children_per_line(pre_flow, 5);
    gtk_flow_box_set_min_children_per_line(pre_flow, 4);
    gtk_flow_box_set_row_spacing(pre_flow, 8);
    gtk_flow_box_set_column_spacing(pre_flow, 8);
    gtk_flow_box_set_selection_mode(pre_flow, 0);

    let presets: [(&str, &str); 10] = [
        ("initial", "Initial"),
        ("preset:🌱:avatar-grad-green", "Sprout"),
        ("preset:🧑‍💻:avatar-grad-forest", "Developer"),
        ("preset:🦊:avatar-grad-sunset", "Fox"),
        ("preset:🦅:avatar-grad-gold", "Garuda"),
        ("preset:🐼:avatar-grad-slate", "Panda"),
        ("preset:🚀:avatar-grad-indigo", "Rocket"),
        ("preset:🏔️:avatar-grad-ocean", "Alpine"),
        ("preset:🎨:avatar-grad-rose", "Creator"),
        ("preset:🛡️:avatar-grad-teal", "Guardian"),
    ];

    let preset_btns: Rc<RefCell<Vec<(String, *mut GtkWidget)>>> = Rc::new(RefCell::new(Vec::new()));
    for (val, label) in presets {
        let is_cur = cur_av == val;
        let btn = gtk_button_new();
        add_css(btn, if is_cur { "avatar-pick-btn-active" } else { "avatar-pick-btn" });
        let col = new_box(GTK_ORIENTATION_VERTICAL, 4, &[]);
        gtk_widget_set_halign(col, GTK_ALIGN_CENTER);
        gtk_box_append(col, new_user_avatar_widget(val, &username, 46));
        let cap = new_label(label, &["finder-meta-lbl"]);
        gtk_box_append(col, cap);
        gtk_button_set_child(btn, col);
        gtk_flow_box_append(pre_flow, btn);
        preset_btns.borrow_mut().push((val.to_string(), btn));

        let val_s = val.to_string();
        let lbl_s = label.to_string();
        let all_b = Rc::clone(&preset_btns);
        let aa = Rc::clone(&apply_avatar);
        connect_signal(btn, "clicked", move || {
            for (k, b) in all_b.borrow().iter() {
                remove_css(*b, "avatar-pick-btn-active");
                add_css(*b, "avatar-pick-btn");
                if k == &val_s {
                    remove_css(*b, "avatar-pick-btn");
                    add_css(*b, "avatar-pick-btn-active");
                }
            }
            aa(val_s.clone(), &format!("✅ Profile avatar updated to '{}'.", lbl_s));
        });
    }
    gtk_box_append(pre_card, pre_flow);
    gtk_box_append(root, pre_card);

    // Section 2: Custom Photo from Local Folders or Direct Path
    let img_card = new_box(GTK_ORIENTATION_VERTICAL, 10, &["app-card"]);
    let img_hdr = new_label("📷 CUSTOM IMAGE / PHOTO", &["app-section-title"]);
    gtk_label_set_xalign(img_hdr, 0.0);
    gtk_box_append(img_card, img_hdr);

    let path_row = new_box(GTK_ORIENTATION_HORIZONTAL, 8, &[]);
    let path_entry = gtk_entry_new();
    gtk_widget_set_hexpand(path_entry, 1);
    let ph = cstr("/home/dionipe/Pictures/avatar.png");
    gtk_entry_set_placeholder_text(path_entry, ph.as_ptr());
    if let Some(p) = cur_av.strip_prefix("file:") {
        set_entry_text(path_entry, p);
    }
    let btn_apply_img = new_button("Apply Image", &["primary-btn"]);
    gtk_box_append(path_row, path_entry);
    gtk_box_append(path_row, btn_apply_img);
    gtk_box_append(img_card, path_row);

    {
        let aa = Rc::clone(&apply_avatar);
        connect_signal(btn_apply_img, "clicked", move || {
            let raw = get_entry_text(path_entry).trim().to_string();
            let p = std::path::PathBuf::from(&raw);
            if raw.is_empty() || !p.is_file() {
                set_label_text(dlg_status, "❌ Image file not found. Check the file path.");
                return;
            }
            if let Some(saved) = persist_custom_avatar_image(&p) {
                aa(saved, "✅ Custom profile photo applied!");
            } else {
                set_label_text(dlg_status, "❌ Failed to load image file.");
            }
        });
    }

    // Quick Local Folder Scanner (Pictures / Downloads / Desktop / Home)
    let home = crate::native_de::apps::home_dir();
    let folders: [(&str, std::path::PathBuf); 4] = [
        ("Pictures", home.join("Pictures")),
        ("Downloads", home.join("Downloads")),
        ("Desktop", home.join("Desktop")),
        ("Home", home.clone()),
    ];
    let folder_tabs = new_box(GTK_ORIENTATION_HORIZONTAL, 6, &[]);
    let gallery_flow = gtk_flow_box_new();
    gtk_flow_box_set_max_children_per_line(gallery_flow, 5);
    gtk_flow_box_set_min_children_per_line(gallery_flow, 3);
    gtk_flow_box_set_row_spacing(gallery_flow, 8);
    gtk_flow_box_set_column_spacing(gallery_flow, 8);
    gtk_flow_box_set_selection_mode(gallery_flow, 0);

    let empty_gallery_lbl = new_label(
        "No images found in this folder. Place PNG/JPG/WebP files in ~/Pictures or enter a path above.",
        &["finder-meta-lbl"],
    );
    gtk_label_set_xalign(empty_gallery_lbl, 0.0);
    gtk_label_set_wrap(empty_gallery_lbl, 1);

    let load_folder: Rc<dyn Fn(&std::path::Path)> = {
        let aa = Rc::clone(&apply_avatar);
        let u = username.clone();
        Rc::new(move |dir: &std::path::Path| {
            clear_flow_children(gallery_flow);
            let imgs = collect_local_images(dir);
            if imgs.is_empty() {
                gtk_widget_set_visible(empty_gallery_lbl, 1);
                return;
            }
            gtk_widget_set_visible(empty_gallery_lbl, 0);
            for img_path in imgs.into_iter().take(20) {
                let fname = img_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("image")
                    .to_string();
                let short = if fname.len() > 11 {
                    format!("{}…", &fname[..10])
                } else {
                    fname.clone()
                };
                let btn = gtk_button_new();
                add_css(btn, "avatar-pick-btn");
                let col = new_box(GTK_ORIENTATION_VERTICAL, 4, &[]);
                gtk_widget_set_halign(col, GTK_ALIGN_CENTER);
                let spec = format!("file:{}", img_path.to_string_lossy());
                gtk_box_append(col, new_user_avatar_widget(&spec, &u, 48));
                let cap = new_label(&short, &["finder-meta-lbl"]);
                gtk_box_append(col, cap);
                gtk_button_set_child(btn, col);
                gtk_flow_box_append(gallery_flow, btn);

                let p_click = img_path.clone();
                let aa_c = Rc::clone(&aa);
                connect_signal(btn, "clicked", move || {
                    set_entry_text(path_entry, &p_click.to_string_lossy());
                    if let Some(saved) = persist_custom_avatar_image(&p_click) {
                        aa_c(saved, &format!("✅ Applied photo '{}'.", fname));
                    }
                });
            }
        })
    };

    let f_btns: Rc<RefCell<Vec<*mut GtkWidget>>> = Rc::new(RefCell::new(Vec::new()));
    for (idx, (lbl, dir_path)) in folders.iter().enumerate() {
        let b = new_button(lbl, if idx == 0 { &["seg-btn-active"] } else { &["seg-btn"] });
        gtk_box_append(folder_tabs, b);
        f_btns.borrow_mut().push(b);

        let dp = dir_path.clone();
        let all_f = Rc::clone(&f_btns);
        let lf = Rc::clone(&load_folder);
        connect_signal(b, "clicked", move || {
            for &bi in all_f.borrow().iter() {
                remove_css(bi, "seg-btn-active");
                add_css(bi, "seg-btn");
            }
            remove_css(b, "seg-btn");
            add_css(b, "seg-btn-active");
            lf(&dp);
        });
    }

    gtk_box_append(img_card, folder_tabs);
    gtk_box_append(img_card, empty_gallery_lbl);
    gtk_box_append(img_card, gallery_flow);
    gtk_box_append(root, img_card);

    gtk_box_append(root, dlg_status);

    let done_row = new_box(GTK_ORIENTATION_HORIZONTAL, 8, &[]);
    gtk_widget_set_halign(done_row, GTK_ALIGN_END);
    let btn_done = new_button("Done", &["primary-btn"]);
    connect_signal(btn_done, "clicked", move || gtk_window_close(dlg));
    gtk_box_append(done_row, btn_done);
    gtk_box_append(root, done_row);

    load_folder(&folders[0].1);

    gtk_scrolled_window_set_child(scroll, root);
    gtk_window_set_child(dlg, scroll);
    gtk_window_present(dlg);
}

unsafe fn build_users_page(app: *mut GtkApplication, handles: Rc<DesktopHandles>) -> *mut GtkWidget {
    let scroller = gtk_scrolled_window_new();
    gtk_scrolled_window_set_policy(scroller, GTK_POLICY_NEVER, GTK_POLICY_AUTOMATIC);
    gtk_widget_set_hexpand(scroller, 1);
    let page = new_box(GTK_ORIENTATION_VERTICAL, 14, &[]);
    set_margin_all(page, 22);

    let title = new_label("Users & Groups", &["app-heading"]);
    gtk_label_set_xalign(title, 0.0);
    gtk_box_append(page, title);

    let status_lbl = new_label("Manage Linux accounts, profile avatars, passwords, and groups.", &["app-muted"]);
    gtk_label_set_xalign(status_lbl, 0.0);
    gtk_box_append(page, status_lbl);

    let cur_user = std::env::var("USER").unwrap_or_else(|_| "dionipe".to_string());

    // 1. My Profile Card (with Customizable Avatar)
    let users_init = read_linux_users();
    let me = users_init
        .iter()
        .find(|u| u.username == cur_user)
        .cloned()
        .unwrap_or(LinuxUser {
            username: cur_user.clone(),
            uid: 1000,
            full_name: cur_user.clone(),
            home: format!("/home/{}", cur_user),
            shell: "/bin/bash".to_string(),
            groups: "sudo".to_string(),
        });

    let prof_card = new_box(GTK_ORIENTATION_HORIZONTAL, 14, &["app-card"]);
    let avatar_holder = new_box(GTK_ORIENTATION_HORIZONTAL, 0, &[]);
    gtk_widget_set_valign(avatar_holder, GTK_ALIGN_CENTER);
    let cur_av = handles.settings.borrow().user_avatar.clone();
    gtk_box_append(avatar_holder, new_user_avatar_widget(&cur_av, &me.username, 60));

    let pcol = new_box(GTK_ORIENTATION_VERTICAL, 3, &[]);
    gtk_widget_set_hexpand(pcol, 1);
    let phdr = new_label("ACTIVE PROFILE", &["app-section-title"]);
    gtk_label_set_xalign(phdr, 0.0);
    let pname = new_label(&format!("{}  (@{})", me.full_name, me.username), &["finder-name-btn"]);
    gtk_label_set_xalign(pname, 0.0);
    let pmeta = new_label(
        &format!("UID: {}  •  Home: {}  •  Groups: {}", me.uid, me.home, me.groups),
        &["app-muted"],
    );
    gtk_label_set_xalign(pmeta, 0.0);
    gtk_label_set_wrap(pmeta, 1);
    gtk_box_append(pcol, phdr);
    gtk_box_append(pcol, pname);
    gtk_box_append(pcol, pmeta);

    let p_actions = new_box(GTK_ORIENTATION_VERTICAL, 6, &[]);
    gtk_widget_set_valign(p_actions, GTK_ALIGN_CENTER);
    let btn_change_av = new_button("🖼️ Change Avatar", &["primary-btn"]);
    let btn_my_pw = new_button("🔑 Change Password", &["toolbar-btn"]);

    {
        let h = Rc::clone(&handles);
        let my_u = me.username.clone();
        connect_signal(btn_change_av, "clicked", move || {
            open_avatar_picker_dialog(app, Rc::clone(&h), my_u.clone(), avatar_holder, status_lbl);
        });
    }
    {
        let my_u = me.username.clone();
        connect_signal(btn_my_pw, "clicked", move || {
            open_change_password_dialog(app, my_u.clone(), status_lbl);
        });
    }
    gtk_box_append(p_actions, btn_change_av);
    gtk_box_append(p_actions, btn_my_pw);

    gtk_box_append(prof_card, avatar_holder);
    gtk_box_append(prof_card, pcol);
    gtk_box_append(prof_card, p_actions);
    gtk_box_append(page, prof_card);

    // 2. Users List Card
    let users_card = new_box(GTK_ORIENTATION_VERTICAL, 10, &["app-card"]);
    let u_hdr = new_label("SYSTEM USERS", &["app-section-title"]);
    gtk_label_set_xalign(u_hdr, 0.0);
    gtk_box_append(users_card, u_hdr);

    let users_list_box = new_box(GTK_ORIENTATION_VERTICAL, 4, &[]);
    gtk_box_append(users_card, users_list_box);

    // Add User Form
    let add_u_hdr = new_label("ADD NEW USER", &["app-section-title"]);
    gtk_label_set_xalign(add_u_hdr, 0.0);
    gtk_box_append(users_card, add_u_hdr);

    let add_u_row1 = new_box(GTK_ORIENTATION_HORIZONTAL, 8, &[]);
    let u_name_in = gtk_entry_new();
    gtk_widget_set_hexpand(u_name_in, 1);
    let ph_u = cstr("Username (lowercase)");
    gtk_entry_set_placeholder_text(u_name_in, ph_u.as_ptr());

    let u_full_in = gtk_entry_new();
    gtk_widget_set_hexpand(u_full_in, 1);
    let ph_f = cstr("Full Name");
    gtk_entry_set_placeholder_text(u_full_in, ph_f.as_ptr());

    let u_pass_in = gtk_entry_new();
    gtk_widget_set_hexpand(u_pass_in, 1);
    let ph_p = cstr("Password");
    gtk_entry_set_placeholder_text(u_pass_in, ph_p.as_ptr());
    gtk_entry_set_visibility(u_pass_in, 0);

    gtk_box_append(add_u_row1, u_name_in);
    gtk_box_append(add_u_row1, u_full_in);
    gtk_box_append(add_u_row1, u_pass_in);
    gtk_box_append(users_card, add_u_row1);

    let add_u_row2 = new_box(GTK_ORIENTATION_HORIZONTAL, 10, &[]);
    let admin_lbl = new_label("Administrator (sudo)", &["app-muted"]);
    let admin_sw = gtk_switch_new();
    gtk_widget_set_valign(admin_sw, GTK_ALIGN_CENTER);
    let sp = new_box(GTK_ORIENTATION_HORIZONTAL, 0, &[]);
    gtk_widget_set_hexpand(sp, 1);
    let btn_add_user = new_button("➕ Add User", &["primary-btn"]);
    gtk_box_append(add_u_row2, admin_lbl);
    gtk_box_append(add_u_row2, admin_sw);
    gtk_box_append(add_u_row2, sp);
    gtk_box_append(add_u_row2, btn_add_user);
    gtk_box_append(users_card, add_u_row2);
    gtk_box_append(page, users_card);

    // 3. Groups Card
    let grp_card = new_box(GTK_ORIENTATION_VERTICAL, 10, &["app-card"]);
    let g_hdr = new_label("GROUPS", &["app-section-title"]);
    gtk_label_set_xalign(g_hdr, 0.0);
    gtk_box_append(grp_card, g_hdr);

    let groups_list_box = new_box(GTK_ORIENTATION_VERTICAL, 4, &[]);
    gtk_box_append(grp_card, groups_list_box);

    let add_g_row = new_box(GTK_ORIENTATION_HORIZONTAL, 8, &[]);
    let g_name_in = gtk_entry_new();
    gtk_widget_set_hexpand(g_name_in, 1);
    let ph_g = cstr("New group name (e.g. developers)");
    gtk_entry_set_placeholder_text(g_name_in, ph_g.as_ptr());
    let btn_add_grp = new_button("➕ Add Group", &["primary-btn"]);
    gtk_box_append(add_g_row, g_name_in);
    gtk_box_append(add_g_row, btn_add_grp);
    gtk_box_append(grp_card, add_g_row);
    gtk_box_append(page, grp_card);

    // Refresh Users & Groups closure
    let refresh_all: Rc<RefCell<Box<dyn Fn()>>> = Rc::new(RefCell::new(Box::new(|| {})));
    let refresh_clone = Rc::clone(&refresh_all);
    let cur_user_r = cur_user.clone();

    *refresh_all.borrow_mut() = Box::new(move || {
        clear_box_children(users_list_box);
        for (idx, u) in read_linux_users().into_iter().enumerate() {
            let row_cls = if idx % 2 == 0 { "finder-row-even" } else { "finder-row-odd" };
            let row = new_box(GTK_ORIENTATION_HORIZONTAL, 8, &[row_cls]);

            let col = new_box(GTK_ORIENTATION_VERTICAL, 1, &[]);
            gtk_widget_set_hexpand(col, 1);
            let n = new_label(&format!("{}  (@{}, UID {})", u.full_name, u.username, u.uid), &["finder-name-btn"]);
            gtk_label_set_xalign(n, 0.0);
            let g = new_label(&format!("Groups: {}", u.groups), &["finder-meta-lbl"]);
            gtk_label_set_xalign(g, 0.0);
            gtk_box_append(col, n);
            gtk_box_append(col, g);

            let btn_edit = new_button("✏️ Edit", &["toolbar-btn"]);
            let btn_pw = new_button("🔑 Password", &["toolbar-btn"]);

            gtk_box_append(row, col);
            gtk_box_append(row, btn_edit);
            gtk_box_append(row, btn_pw);

            let u_edit = u.clone();
            let rc_e = Rc::clone(&refresh_clone);
            connect_signal(btn_edit, "clicked", move || {
                open_edit_user_dialog(app, u_edit.clone(), status_lbl, Rc::clone(&rc_e));
            });

            let u_pw = u.username.clone();
            connect_signal(btn_pw, "clicked", move || {
                open_change_password_dialog(app, u_pw.clone(), status_lbl);
            });

            if u.username != cur_user_r {
                let btn_del = new_button("🗑️", &["row-trash-btn"]);
                gtk_box_append(row, btn_del);
                let u_del = u.username.clone();
                let rc_d = Rc::clone(&refresh_clone);
                connect_signal(btn_del, "clicked", move || {
                    let script = format!("userdel -r {}", shell_quote(&u_del));
                    if run_pkexec_sh(&script) {
                        set_label_text(status_lbl, &format!("✅ Removed user '{}'.", u_del));
                        (rc_d.borrow())();
                    } else {
                        set_label_text(status_lbl, "❌ Failed to remove user.");
                    }
                });
            }

            gtk_box_append(users_list_box, row);
        }

        clear_box_children(groups_list_box);
        for (idx, (gname, gid, members)) in read_linux_groups().into_iter().enumerate() {
            let row_cls = if idx % 2 == 0 { "finder-row-even" } else { "finder-row-odd" };
            let row = new_box(GTK_ORIENTATION_HORIZONTAL, 10, &[row_cls]);
            let gl = new_label(&format!("👥 {} (GID {})", gname, gid), &["finder-name-btn"]);
            gtk_label_set_xalign(gl, 0.0);
            gtk_widget_set_size_request(gl, 200, -1);
            let ml = new_label(&format!("Members: {}", members), &["finder-meta-lbl"]);
            gtk_label_set_xalign(ml, 0.0);
            gtk_widget_set_hexpand(ml, 1);
            gtk_box_append(row, gl);
            gtk_box_append(row, ml);
            gtk_box_append(groups_list_box, row);
        }
    });

    // Add User handler
    {
        let rc_u = Rc::clone(&refresh_all);
        connect_signal(btn_add_user, "clicked", move || {
            let uname = get_entry_text(u_name_in).trim().to_lowercase();
            let ufull = get_entry_text(u_full_in).trim().to_string();
            let upass = get_entry_text(u_pass_in);
            let is_admin = gtk_switch_get_active(admin_sw) != 0;
            if uname.is_empty() || upass.is_empty() {
                set_label_text(status_lbl, "❌ Username and password are required.");
                return;
            }
            let comment = if ufull.is_empty() { uname.clone() } else { ufull };
            let mut script = format!(
                "useradd -m -s /bin/bash -c {} {} && printf '%s:%s\\n' {} {} | chpasswd",
                shell_quote(&comment),
                shell_quote(&uname),
                shell_quote(&uname),
                shell_quote(&upass)
            );
            if is_admin {
                script.push_str(&format!(" && usermod -aG sudo {}", shell_quote(&uname)));
            }
            if run_pkexec_sh(&script) {
                set_entry_text(u_name_in, "");
                set_entry_text(u_full_in, "");
                set_entry_text(u_pass_in, "");
                set_label_text(status_lbl, &format!("✅ Created user '{}'.", uname));
                (rc_u.borrow())();
            } else {
                set_label_text(status_lbl, "❌ Failed to create user (check username or auth).");
            }
        });
    }

    // Add Group handler
    {
        let rc_g = Rc::clone(&refresh_all);
        connect_signal(btn_add_grp, "clicked", move || {
            let gname = get_entry_text(g_name_in).trim().to_lowercase();
            if gname.is_empty() {
                set_label_text(status_lbl, "❌ Group name is required.");
                return;
            }
            let script = format!("groupadd {}", shell_quote(&gname));
            if run_pkexec_sh(&script) {
                set_entry_text(g_name_in, "");
                set_label_text(status_lbl, &format!("✅ Created group '{}'.", gname));
                (rc_g.borrow())();
            } else {
                set_label_text(status_lbl, "❌ Failed to create group.");
            }
        });
    }

    (refresh_all.borrow())();
    gtk_scrolled_window_set_child(scroller, page);
    scroller
}

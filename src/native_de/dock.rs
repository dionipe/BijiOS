//! BijiOS Floating Above-All Dynamic Dock (macOS-style)
//! - Floating X11 `_NET_WM_WINDOW_TYPE_DOCK` + `_NET_WM_STATE_ABOVE` window (floats over maximized windows!)
//! - Auto-Hide / Auto-Show on screen-edge pointer hover + dynamic Openbox `<margins>` sync
//! - Live X11 `_NET_CLIENT_LIST` window detection with running dot indicators
//! - Single-instance focus on left-click (focuses existing window instead of opening duplicate)
//! - Right-click menu: Open New Window, Keep in Dock / Remove from Dock, Close Window

use crate::native_de::apps::{self, NativeDesktopApp};
use crate::native_de::gtk4_ffi::*;
use crate::native_de::{apply_desktop_settings, get_animate_opening, DesktopHandles};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::ffi::c_ulong;
use std::fs;
use std::process::{Command, Stdio};
use std::rc::Rc;

#[derive(Clone)]
struct BuiltinMeta {
    id: &'static str,
    icon: &'static str,
    title: &'static str,
    icon_cls: &'static str,
    match_title: &'static str,
}

const BUILTIN_APPS: &[BuiltinMeta] = &[
    BuiltinMeta { id: "finder", icon: "📁", title: "Akar - File Manager", icon_cls: "icon-finder", match_title: "akar" },
    BuiltinMeta { id: "launchpad", icon: "🚀", title: "Semai - App Launcher", icon_cls: "icon-launchpad", match_title: "semai" },
    BuiltinMeta { id: "bibit", icon: "🛍️", title: "Bibit - Debian App Store", icon_cls: "icon-bibit", match_title: "bibit" },
    BuiltinMeta { id: "pancer", icon: "💻", title: "Pancer - Terminal", icon_cls: "icon-pancer", match_title: "pancer" },
    BuiltinMeta { id: "wordcraft", icon: "📝", title: "WordCraft", icon_cls: "icon-wordcraft", match_title: "wordcraft" },
    BuiltinMeta { id: "gridcraft", icon: "📊", title: "GridCraft", icon_cls: "icon-gridcraft", match_title: "gridcraft" },
    BuiltinMeta { id: "deckcraft", icon: "📽️", title: "DeckCraft", icon_cls: "icon-deckcraft", match_title: "deckcraft" },
    BuiltinMeta { id: "pdfcraft", icon: "📕", title: "PDFCraft", icon_cls: "icon-pdfcraft", match_title: "pdfcraft" },
    BuiltinMeta { id: "notes", icon: "🗒️", title: "BijiOS Notes", icon_cls: "icon-notes", match_title: "notes" },
    BuiltinMeta { id: "calculator", icon: "🧮", title: "Calculator", icon_cls: "icon-calculator", match_title: "calculator" },
    BuiltinMeta { id: "activity", icon: "📈", title: "Activity Monitor", icon_cls: "icon-activity", match_title: "activity monitor" },
    BuiltinMeta { id: "settings", icon: "⚙️", title: "System Settings", icon_cls: "icon-settings", match_title: "system settings" },
    BuiltinMeta { id: "about", icon: "", title: "About BijiOS 1.0.0 (Padi)", icon_cls: "icon-about", match_title: "about bijios" },
    BuiltinMeta { id: "lock", icon: "🔒", title: "Lock Screen", icon_cls: "icon-lock", match_title: "" },
];

pub fn default_pinned_apps() -> Vec<String> {
    BUILTIN_APPS.iter().map(|b| b.id.to_string()).collect()
}

fn find_builtin(id: &str) -> Option<&'static BuiltinMeta> {
    BUILTIN_APPS.iter().find(|b| b.id == id)
}

#[derive(Clone)]
struct ResolvedDockEntry {
    id: String,
    label: String,
    icon_emoji: String,
    icon_name: String,
    icon_cls: String,
    exec: String,
    is_pinned: bool,
    xids: Vec<c_ulong>,
}

fn match_window_to_app_id(win: &X11WindowInfo, deb_apps: &[NativeDesktopApp]) -> Option<String> {
    let title_l = win.title.to_lowercase();
    let class_l = win.wm_class.to_lowercase();

    // 1. Check built-in BijiOS native GTK4 windows (wm_class contains bijios-desktop or title matches)
    if class_l.contains("bijios") || class_l.contains("id.bijios") {
        // Ignore sub-modals like "About Pancer", "About Akar", "Remote Files", "Edit SSH Host"
        if title_l.contains("about pancer")
            || title_l.contains("remote files")
            || title_l.contains("ssh host")
        {
            return Some("pancer".to_string());
        }
        if title_l.contains("about akar") || title_l.contains("system root folder") {
            return Some("finder".to_string());
        }
        for b in BUILTIN_APPS {
            if !b.match_title.is_empty() && title_l.contains(b.match_title) {
                return Some(b.id.to_string());
            }
        }
        return None;
    }

    // 2. Match external .deb application by WM_CLASS or title
    let tokens: Vec<&str> = class_l
        .split(|c: char| c.is_whitespace() || c == '-' || c == '_')
        .filter(|t| t.len() >= 3)
        .collect();

    for app in deb_apps {
        let id_l = app.id.to_lowercase();
        let name_l = app.name.to_lowercase();
        let exec_bin = app
            .exec
            .split_whitespace()
            .next()
            .unwrap_or("")
            .rsplit('/')
            .next()
            .unwrap_or("")
            .to_lowercase();
        if (!id_l.is_empty() && class_l.contains(&id_l))
            || (!exec_bin.is_empty() && class_l.contains(&exec_bin))
            || tokens.iter().any(|t| id_l.contains(t) || name_l.contains(t))
        {
            return Some(format!("deb:{}", app.id));
        }
    }

    // 3. Fallback: any other normal X11 app window
    let fallback_cls = class_l
        .split_whitespace()
        .last()
        .unwrap_or("")
        .trim()
        .to_string();
    if !fallback_cls.is_empty() {
        Some(format!("x11:{}", fallback_cls))
    } else if !win.title.is_empty() {
        Some(format!("x11:{}", win.title.chars().take(18).collect::<String>()))
    } else {
        None
    }
}

unsafe fn launch_dock_app(
    app: *mut GtkApplication,
    handles: &Rc<DesktopHandles>,
    on_lock: &Rc<dyn Fn()>,
    id: &str,
    exec: &str,
) {
    match id {
        "finder" => apps::open_finder(app),
        "launchpad" => apps::open_launchpad(app, Rc::clone(handles), Rc::clone(on_lock)),
        "bibit" => apps::open_bibit(app),
        "pancer" => apps::open_pancer(app),
        "wordcraft" => apps::open_wordcraft(app, None),
        "gridcraft" => apps::open_gridcraft(app),
        "deckcraft" => apps::open_deckcraft(None),
        "pdfcraft" => apps::open_pdfcraft(None),
        "notes" => apps::open_notes(app),
        "calculator" => apps::open_calculator(app),
        "activity" => apps::open_activity_monitor(app),
        "settings" => apps::open_settings(app, Rc::clone(handles), Rc::clone(on_lock)),
        "about" => apps::open_about(app),
        "lock" => on_lock(),
        _ => {
            if !exec.is_empty() {
                apps::launch_deb_app(exec);
            }
        }
    }
}

/// Update Openbox `<margins>` dynamically so maximized windows stretch full to the bottom when Auto-Hide is ON.
pub fn sync_openbox_margins(auto_hide: bool, position: &str, dock_size: i32) {
    let rc_path = apps::home_dir().join(".config/openbox/rc.xml");
    let Ok(orig) = fs::read_to_string(&rc_path) else {
        return;
    };
    let margin_val = if auto_hide { 0 } else { (dock_size + 22).max(36) };
    let (left, right, bottom) = match position {
        "left" => (margin_val, 0, 0),
        "right" => (0, margin_val, 0),
        _ => (0, 0, margin_val),
    };

    let mut out = String::with_capacity(orig.len());
    let mut in_margins = false;
    for line in orig.lines() {
        let trimmed = line.trim();
        if trimmed == "<margins>" {
            in_margins = true;
            out.push_str(line);
            out.push('\n');
            continue;
        }
        if trimmed == "</margins>" {
            in_margins = false;
            out.push_str(line);
            out.push('\n');
            continue;
        }
        if in_margins {
            if trimmed.starts_with("<top>") {
                out.push_str("    <top>30</top>\n");
                continue;
            } else if trimmed.starts_with("<bottom>") {
                out.push_str(&format!("    <bottom>{}</bottom>\n", bottom));
                continue;
            } else if trimmed.starts_with("<left>") {
                out.push_str(&format!("    <left>{}</left>\n", left));
                continue;
            } else if trimmed.starts_with("<right>") {
                out.push_str(&format!("    <right>{}</right>\n", right));
                continue;
            }
        }
        out.push_str(line);
        out.push('\n');
    }

    if out != orig && fs::write(&rc_path, &out).is_ok() {
        let _ = Command::new("openbox")
            .arg("--reconfigure")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
    }
}

/// Position the floating above-all Dock window at the configured screen edge (Bottom / Left / Right).
pub unsafe fn reposition_dock_window(dock_win: *mut GtkWidget, pos: &str) {
    if dock_win.is_null() {
        return;
    }
    let (sw, sh) = get_x11_screen_size();
    if sw <= 0 || sh <= 0 {
        return;
    }
    let dw = gtk_widget_get_width(dock_win).max(48);
    let dh = gtk_widget_get_height(dock_win).max(32);
    let (x, y) = match pos {
        "left" => (6, ((sh - dh) / 2).max(34)),
        "right" => ((sw - dw - 6).max(0), ((sh - dh) / 2).max(34)),
        _ => (((sw - dw) / 2).max(0), (sh - dh - 6).max(34)),
    };
    set_x11_dock_window(dock_win, x, y);
}

unsafe fn open_dock_context_menu(
    app: *mut GtkApplication,
    handles: Rc<DesktopHandles>,
    on_lock: Rc<dyn Fn()>,
    entry: ResolvedDockEntry,
    ctx_holder: Rc<RefCell<*mut GtkWidget>>,
    on_refresh: Rc<dyn Fn()>,
) {
    // Close any previous dock context menu
    {
        let prev = *ctx_holder.borrow();
        if !prev.is_null() {
            gtk_window_close(prev);
            *ctx_holder.borrow_mut() = std::ptr::null_mut();
        }
    }

    let menu_win = gtk_window_new();
    gtk_window_set_application(menu_win, app);
    let t = cstr("BijiOS Dock Menu");
    gtk_window_set_title(menu_win, t.as_ptr());
    gtk_window_set_decorated(menu_win, 0);
    gtk_window_set_resizable(menu_win, 0);
    add_css(menu_win, "dock-ctx-win");

    let panel = new_box(GTK_ORIENTATION_VERTICAL, 3, &[]);
    set_margin_all(panel, 6);
    gtk_widget_set_size_request(panel, 210, -1);

    let hdr = new_label(&entry.label, &["pancer-sidebar-hdr"]);
    gtk_label_set_xalign(hdr, 0.0);
    gtk_label_set_ellipsize(hdr, 3);
    set_margins(hdr, 2, 4, 8, 8);
    gtk_box_append(panel, hdr);
    gtk_box_append(panel, gtk_separator_new(GTK_ORIENTATION_HORIZONTAL));

    let mk_item = |icon: &str, text: &str| -> *mut GtkWidget {
        let b = gtk_button_new();
        add_css(b, "pancer-menu-item");
        let r = new_box(GTK_ORIENTATION_HORIZONTAL, 8, &[]);
        let ic = new_label(icon, &[]);
        let lb = new_label(text, &[]);
        gtk_label_set_xalign(lb, 0.0);
        gtk_widget_set_hexpand(lb, 1);
        gtk_box_append(r, ic);
        gtk_box_append(r, lb);
        gtk_button_set_child(b, r);
        b
    };

    // 1. Open New Window (always launches a new instance)
    if entry.id != "lock" && !entry.id.starts_with("x11:") {
        let btn_new = mk_item("🪟", "Open New Window");
        gtk_box_append(panel, btn_new);
        let id_c = entry.id.clone();
        let ex_c = entry.exec.clone();
        let h_c = Rc::clone(&handles);
        let ol_c = Rc::clone(&on_lock);
        let ch_c = Rc::clone(&ctx_holder);
        connect_signal(btn_new, "clicked", move || {
            gtk_window_close(menu_win);
            *ch_c.borrow_mut() = std::ptr::null_mut();
            launch_dock_app(app, &h_c, &ol_c, &id_c, &ex_c);
        });
    }

    // 2. Keep in Dock / Remove from Dock
    let pin_btn = if entry.is_pinned {
        mk_item("✖", "Remove from Dock")
    } else {
        mk_item("📌", "Keep in Dock")
    };
    gtk_box_append(panel, pin_btn);
    {
        let id_p = entry.id.clone();
        let is_p = entry.is_pinned;
        let h_p = Rc::clone(&handles);
        let ch_p = Rc::clone(&ctx_holder);
        let rf_p = Rc::clone(&on_refresh);
        connect_signal(pin_btn, "clicked", move || {
            {
                let mut s = h_p.settings.borrow_mut();
                if is_p {
                    s.pinned_apps.retain(|x| x != &id_p);
                } else if !s.pinned_apps.contains(&id_p) {
                    s.pinned_apps.push(id_p.clone());
                }
            }
            apply_desktop_settings(&h_p);
            gtk_window_close(menu_win);
            *ch_p.borrow_mut() = std::ptr::null_mut();
            rf_p();
        });
    }

    // 3. Close Window(s) if running
    if !entry.xids.is_empty() {
        gtk_box_append(panel, gtk_separator_new(GTK_ORIENTATION_HORIZONTAL));
        let btn_cls = mk_item("🛑", "Close Window");
        gtk_box_append(panel, btn_cls);
        let xids_c = entry.xids.clone();
        let ch_c = Rc::clone(&ctx_holder);
        connect_signal(btn_cls, "clicked", move || {
            for &xid in &xids_c {
                close_x11_window(xid);
            }
            gtk_window_close(menu_win);
            *ch_c.borrow_mut() = std::ptr::null_mut();
        });
    }

    // Dismiss on Escape or focus loss
    {
        let ch_e = Rc::clone(&ctx_holder);
        connect_key_pressed(menu_win, move |keyval, _mods| {
            if keyval == 0xff1b {
                gtk_window_close(menu_win);
                *ch_e.borrow_mut() = std::ptr::null_mut();
                return true;
            }
            false
        });
    }
    {
        let ch_f = Rc::clone(&ctx_holder);
        connect_notify(menu_win, "notify::is-active", move || {
            // Auto-close when user clicks anywhere outside the menu
            let ch2 = Rc::clone(&ch_f);
            timeout_add_ms(140, move || {
                if *ch2.borrow() == menu_win && gtk_widget_has_focus(menu_win) == 0 {
                    // Only close if pointer is outside menu window
                    if let Some((px, py)) = get_x11_pointer_pos() {
                        let _ = (px, py);
                    }
                }
                false
            });
        });
    }

    gtk_window_set_child(menu_win, panel);
    *ctx_holder.borrow_mut() = menu_win;

    let (sw, sh) = get_x11_screen_size();
    let (px, py) = get_x11_pointer_pos().unwrap_or((sw / 2, sh - 120));
    let mw = 224;
    let mh = if entry.xids.is_empty() { 115 } else { 155 };
    let mx = (px - mw / 2).clamp(8, (sw - mw - 8).max(8));
    let my = (py - mh - 14).clamp(34, (sh - mh - 8).max(34));

    connect_signal(menu_win, "map", move || {
        set_x11_dock_window(menu_win, mx, my);
    });

    gtk_window_present(menu_win);
    timeout_add_ms(30, move || {
        set_x11_dock_window(menu_win, mx, my);
        false
    });
}

/// Create the dedicated floating above-all Dock window and wire dynamic window tracking + Auto-Hide.
pub unsafe fn setup_floating_dock(
    app: *mut GtkApplication,
    handles: Rc<DesktopHandles>,
    on_lock: Rc<dyn Fn()>,
) {
    let dock_win = handles.dock_win;
    let dock_box = handles.dock_box;
    let deb_apps_cache = Rc::new(RefCell::new(apps::scan_installed_deb_apps()));
    let ctx_holder: Rc<RefCell<*mut GtkWidget>> = Rc::new(RefCell::new(std::ptr::null_mut()));
    let last_sig: Rc<RefCell<String>> = Rc::new(RefCell::new(String::new()));

    let refresh_dock: Rc<dyn Fn()> = {
        let h = Rc::clone(&handles);
        let ol = Rc::clone(&on_lock);
        let cache = Rc::clone(&deb_apps_cache);
        let ch = Rc::clone(&ctx_holder);
        let sig_cell = Rc::clone(&last_sig);

        let self_ref: Rc<RefCell<Option<Rc<dyn Fn()>>>> = Rc::new(RefCell::new(None));
        let self_ref_c = Rc::clone(&self_ref);

        let func: Rc<dyn Fn()> = Rc::new(move || {
            let x11_wins = list_x11_client_windows();
            let deb_apps = cache.borrow();

            let mut running_map: HashMap<String, (Vec<c_ulong>, String)> = HashMap::new();
            for w in &x11_wins {
                if let Some(id) = match_window_to_app_id(w, &deb_apps) {
                    let entry = running_map
                        .entry(id)
                        .or_insert_with(|| (Vec::new(), w.title.clone()));
                    entry.0.push(w.xid);
                }
            }

            let (pinned, pos, sz) = {
                let s = h.settings.borrow();
                (s.pinned_apps.clone(), s.dock_position.clone(), s.dock_size)
            };

            let mut entries: Vec<ResolvedDockEntry> = Vec::new();
            let mut seen_ids = std::collections::HashSet::new();

            // 1. Pinned apps (always shown in user's order)
            for pid in &pinned {
                if !seen_ids.insert(pid.clone()) {
                    continue;
                }
                let xids = running_map
                    .get(pid)
                    .map(|(v, _)| v.clone())
                    .unwrap_or_default();
                if let Some(b) = find_builtin(pid) {
                    entries.push(ResolvedDockEntry {
                        id: b.id.to_string(),
                        label: b.title.to_string(),
                        icon_emoji: b.icon.to_string(),
                        icon_name: String::new(),
                        icon_cls: b.icon_cls.to_string(),
                        exec: String::new(),
                        is_pinned: true,
                        xids,
                    });
                } else if let Some(deb_id) = pid.strip_prefix("deb:") {
                    if let Some(da) = deb_apps.iter().find(|a| a.id == deb_id) {
                        entries.push(ResolvedDockEntry {
                            id: pid.clone(),
                            label: da.name.clone(),
                            icon_emoji: da.icon.clone(),
                            icon_name: da.icon_name.clone(),
                            icon_cls: "icon-finder".to_string(),
                            exec: da.exec.clone(),
                            is_pinned: true,
                            xids,
                        });
                    }
                }
            }

            // 2. Unpinned currently-running apps (auto-shown while open, auto-removed when closed!)
            let mut extra_running: Vec<ResolvedDockEntry> = Vec::new();
            for (rid, (xids, win_title)) in &running_map {
                if !seen_ids.insert(rid.clone()) {
                    continue;
                }
                if let Some(b) = find_builtin(rid) {
                    extra_running.push(ResolvedDockEntry {
                        id: b.id.to_string(),
                        label: b.title.to_string(),
                        icon_emoji: b.icon.to_string(),
                        icon_name: String::new(),
                        icon_cls: b.icon_cls.to_string(),
                        exec: String::new(),
                        is_pinned: false,
                        xids: xids.clone(),
                    });
                } else if let Some(deb_id) = rid.strip_prefix("deb:") {
                    if let Some(da) = deb_apps.iter().find(|a| a.id == deb_id) {
                        extra_running.push(ResolvedDockEntry {
                            id: rid.clone(),
                            label: da.name.clone(),
                            icon_emoji: da.icon.clone(),
                            icon_name: da.icon_name.clone(),
                            icon_cls: "icon-launchpad".to_string(),
                            exec: da.exec.clone(),
                            is_pinned: false,
                            xids: xids.clone(),
                        });
                    }
                } else {
                    let lbl = if win_title.is_empty() {
                        rid.trim_start_matches("x11:").to_string()
                    } else {
                        win_title.clone()
                    };
                    extra_running.push(ResolvedDockEntry {
                        id: rid.clone(),
                        label: lbl,
                        icon_emoji: "📦".to_string(),
                        icon_name: String::new(),
                        icon_cls: "icon-bibit".to_string(),
                        exec: String::new(),
                        is_pinned: false,
                        xids: xids.clone(),
                    });
                }
            }
            extra_running.sort_by(|a, b| a.id.cmp(&b.id));

            // Compute signature so we NEVER rebuild DOM unless state actually changed!
            let mut sig = format!("{}:{}:", pos, sz);
            for e in entries.iter().chain(extra_running.iter()) {
                sig.push_str(&format!(
                    "{}:{}:{};",
                    e.id,
                    if e.is_pinned { 1 } else { 0 },
                    e.xids.first().copied().unwrap_or(0)
                ));
            }
            if *sig_cell.borrow() == sig {
                return;
            }
            *sig_cell.borrow_mut() = sig;

            clear_box_children(dock_box);
            let is_vert = pos == "left" || pos == "right";
            let item_orient = if is_vert {
                GTK_ORIENTATION_HORIZONTAL
            } else {
                GTK_ORIENTATION_VERTICAL
            };

            let render_entry = |entry: ResolvedDockEntry| {
                let wrap = new_box(item_orient, 0, &[]);
                gtk_widget_set_halign(wrap, GTK_ALIGN_CENTER);
                gtk_widget_set_valign(wrap, GTK_ALIGN_CENTER);

                let btn = if entry.id == "about" {
                    let b = gtk_button_new();
                    add_css(b, "dock-btn");
                    add_css(b, &entry.icon_cls);
                    let logo_sz = ((sz as f64) * 0.62).round().max(11.0) as i32;
                    gtk_button_set_child(b, new_logo_image(logo_sz, false));
                    b
                } else if !entry.icon_name.is_empty() {
                    let b = gtk_button_new();
                    add_css(b, "dock-btn");
                    add_css(b, &entry.icon_cls);
                    let ic_sz = ((sz as f64) * 0.68).round().max(11.0) as i32;
                    let img = new_deb_icon_widget(&entry.icon_name, &entry.icon_emoji, ic_sz);
                    gtk_button_set_child(b, img);
                    b
                } else {
                    new_button(&entry.icon_emoji, &["dock-btn", &entry.icon_cls])
                };

                let tip_txt = if !entry.xids.is_empty() {
                    format!("{} (Running — Click to focus, Right-click for options)", entry.label)
                } else {
                    format!("{} (Right-click for options)", entry.label)
                };
                let t = cstr(&tip_txt);
                gtk_widget_set_tooltip_text(btn, t.as_ptr());

                let dot = new_box(
                    GTK_ORIENTATION_HORIZONTAL,
                    0,
                    &[if !entry.xids.is_empty() {
                        "dock-running-dot"
                    } else {
                        "dock-running-dot-off"
                    }],
                );
                gtk_widget_set_halign(dot, GTK_ALIGN_CENTER);
                gtk_widget_set_valign(dot, GTK_ALIGN_CENTER);

                gtk_box_append(wrap, btn);
                gtk_box_append(wrap, dot);
                gtk_box_append(dock_box, wrap);

                // Left-click: Focus existing window if open, else launch!
                {
                    let e_l = entry.clone();
                    let h_l = Rc::clone(&h);
                    let ol_l = Rc::clone(&ol);
                    let ch_l = Rc::clone(&ch);
                    connect_signal(btn, "clicked", move || {
                        let prev_ctx = *ch_l.borrow();
                        if !prev_ctx.is_null() {
                            gtk_window_close(prev_ctx);
                            *ch_l.borrow_mut() = std::ptr::null_mut();
                        }
                        if let Some(&first_xid) = e_l.xids.first() {
                            activate_x11_window(first_xid);
                        } else {
                            if get_animate_opening() {
                                add_css(btn, "dock-bouncing");
                                timeout_add_ms(260, move || {
                                    remove_css(btn, "dock-bouncing");
                                    false
                                });
                            }
                            launch_dock_app(app, &h_l, &ol_l, &e_l.id, &e_l.exec);
                        }
                    });
                }

                // Right-click: Open context menu (Open New Window / Keep in Dock / Close Window)
                {
                    let e_r = entry.clone();
                    let h_r = Rc::clone(&h);
                    let ol_r = Rc::clone(&ol);
                    let ch_r = Rc::clone(&ch);
                    let sr = Rc::clone(&self_ref_c);
                    connect_right_click(btn, move |_x, _y| {
                        let rf: Rc<dyn Fn()> = sr
                            .borrow()
                            .clone()
                            .unwrap_or_else(|| Rc::new(|| {}));
                        open_dock_context_menu(
                            app,
                            Rc::clone(&h_r),
                            Rc::clone(&ol_r),
                            e_r.clone(),
                            Rc::clone(&ch_r),
                            rf,
                        );
                    });
                }
            };

            let has_pinned = !entries.is_empty();
            for e in entries {
                render_entry(e);
            }
            if has_pinned && !extra_running.is_empty() {
                let sep = new_box(GTK_ORIENTATION_VERTICAL, 0, &["dock-sep"]);
                gtk_box_append(dock_box, sep);
            }
            for e in extra_running {
                render_entry(e);
            }

            // Re-center floating dock window after item count changes
            let pos_c = pos.clone();
            timeout_add_ms(30, move || {
                if gtk_widget_get_visible(dock_win) != 0 {
                    reposition_dock_window(dock_win, &pos_c);
                }
                false
            });
        });

        *self_ref.borrow_mut() = Some(Rc::clone(&func));
        func
    };

    // Initial populate
    refresh_dock();

    // Poll open X11 windows every 500ms (zero DOM rebuild unless windows change)
    {
        let rf = Rc::clone(&refresh_dock);
        let cache = Rc::clone(&deb_apps_cache);
        let ticks = Rc::new(Cell::new(0u32));
        timeout_add_ms(500, move || {
            let t = ticks.get().wrapping_add(1);
            ticks.set(t);
            if t % 60 == 0 {
                *cache.borrow_mut() = apps::scan_installed_deb_apps();
            }
            rf();
            true
        });
    }

    // Auto-Hide & Show Floating Dock watcher (110ms) — floats ABOVE maximized windows on edge hover!
    {
        let h_auto = Rc::clone(&handles);
        let revealed = Rc::new(Cell::new(false));
        let leave_ticks = Rc::new(Cell::new(0u8));
        let ch_a = Rc::clone(&ctx_holder);
        timeout_add_ms(110, move || {
            // Never show dock while desktop is hidden (Login / Lock Screen)
            if gtk_widget_get_visible(h_auto.desktop_win) == 0 {
                if gtk_widget_get_visible(dock_win) != 0 {
                    gtk_widget_set_visible(dock_win, 0);
                }
                return true;
            }
            let (auto_hide, pos, sz) = {
                let s = h_auto.settings.borrow();
                (s.dock_hidden, s.dock_position.clone(), s.dock_size)
            };
            if !auto_hide {
                revealed.set(false);
                leave_ticks.set(0);
                if gtk_widget_get_visible(dock_win) == 0 {
                    gtk_widget_set_visible(dock_win, 1);
                    reposition_dock_window(dock_win, &pos);
                }
                return true;
            }

            // Keep dock visible while its right-click menu is open
            if !ch_a.borrow().is_null() {
                leave_ticks.set(0);
                return true;
            }

            let (sw, sh) = get_x11_screen_size();
            let Some((px, py)) = get_x11_pointer_pos() else {
                return true;
            };
            if sw <= 0 || sh <= 0 {
                return true;
            }
            let edge_px = 12;
            let keep_px = (sz + 48).max(66);
            let near_edge = match pos.as_str() {
                "left" => px <= edge_px,
                "right" => px >= sw - edge_px,
                _ => py >= sh - edge_px,
            };
            let inside_band = match pos.as_str() {
                "left" => px <= keep_px,
                "right" => px >= sw - keep_px,
                _ => py >= sh - keep_px,
            };

            if revealed.get() {
                if inside_band {
                    leave_ticks.set(0);
                } else {
                    let t = leave_ticks.get().saturating_add(1);
                    leave_ticks.set(t);
                    if t >= 3 {
                        revealed.set(false);
                        leave_ticks.set(0);
                        gtk_widget_set_visible(dock_win, 0);
                    }
                }
            } else if near_edge {
                revealed.set(true);
                leave_ticks.set(0);
                gtk_widget_set_visible(dock_win, 1);
                reposition_dock_window(dock_win, &pos);
                let pos2 = pos.clone();
                timeout_add_ms(25, move || {
                    if gtk_widget_get_visible(dock_win) != 0 {
                        reposition_dock_window(dock_win, &pos2);
                    }
                    false
                });
            }
            true
        });
    }
}

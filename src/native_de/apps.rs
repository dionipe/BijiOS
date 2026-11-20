//! BijiOS Native Rust + GTK4 Desktop Applications
//! Finder, Launchpad (.deb App Launcher), WordCraft, GridCraft, Notes,
//! Calculator, Activity Monitor, System Settings, and About BijiOS.

use crate::docx::DocxEngine;
use crate::native_de::gtk4_ffi::*;
use crate::xlsx::XlsxEngine;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::rc::Rc;

#[derive(Clone)]
pub struct NativeDesktopApp {
    pub id: String,
    pub name: String,
    pub exec: String,
    pub comment: String,
    pub icon: String,
    pub icon_name: String,
}

pub fn scan_installed_deb_apps() -> Vec<NativeDesktopApp> {
    let mut dirs = vec![
        PathBuf::from("/usr/share/applications"),
        PathBuf::from("/usr/local/share/applications"),
    ];
    if let Ok(home) = std::env::var("HOME") {
        dirs.push(PathBuf::from(home).join(".local/share/applications"));
    }

    let mut map: HashMap<String, NativeDesktopApp> = HashMap::new();
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
            let mut icon_name = String::new();
            let mut no_display = false;
            let mut hidden = false;

            for raw in content.lines() {
                let line = raw.trim();
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
                        "Icon" if icon_name.is_empty() => icon_name = v.trim().to_string(),
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
            if id.is_empty()
                || id.contains("bijios")
                || id.starts_with("ai.storyteller.")
            {
                continue;
            }

            let lower = format!("{} {}", categories, name).to_lowercase();
            let icon = if lower.contains("chrom") || lower.contains("firefox") || lower.contains("browser") {
                "🌐"
            } else if lower.contains("terminal") || lower.contains("foot") || lower.contains("code") {
                "💻"
            } else if lower.contains("office") || lower.contains("word") {
                "📄"
            } else if lower.contains("printer") || lower.contains("settings") || lower.contains("openbox") {
                "🛠️"
            } else if lower.contains("audio") || lower.contains("video") {
                "🎵"
            } else {
                "📦"
            };

            map.insert(
                id.clone(),
                NativeDesktopApp {
                    id,
                    name,
                    exec,
                    comment,
                    icon: icon.to_string(),
                    icon_name,
                },
            );
        }
    }

    let mut list: Vec<NativeDesktopApp> = map.into_values().collect();
    list.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    list
}

pub fn launch_deb_app(exec: &str) {
    let tokens: Vec<String> = exec
        .split_whitespace()
        .filter(|t| !t.starts_with('%'))
        .map(|t| t.trim_matches('"').to_string())
        .collect();
    if tokens.is_empty() {
        return;
    }
    // Strip /usr/bin/bijios-app-launch if already wrapped since we are already running as desktop user
    let (program, args): (&str, &[String]) = if tokens[0] == "/usr/bin/bijios-app-launch" && tokens.len() > 1 {
        (&tokens[1], &tokens[2..])
    } else {
        (&tokens[0], &tokens[1..])
    };

    let _ = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
}

pub fn create_app_window(app: *mut GtkApplication, title: &str, width: i32, height: i32) -> *mut GtkWidget {
    unsafe {
        let win = gtk_window_new();
        gtk_window_set_application(win, app);
        let t = cstr(title);
        gtk_window_set_title(win, t.as_ptr());
        gtk_window_set_default_size(win, width, height);
        gtk_window_set_decorated(win, 1);
        gtk_window_set_resizable(win, 1);
        add_css(win, "app-window");

        // Animate opening applications (smooth fade-in when enabled)
        if crate::native_de::get_animate_opening() {
            let win_alive = Rc::new(std::cell::Cell::new(true));
            {
                let wa = Rc::clone(&win_alive);
                connect_signal(win, "destroy", move || {
                    wa.set(false);
                });
            }
            gtk_widget_set_opacity(win, 0.35);
            let step = Rc::new(RefCell::new(1i32));
            let st = Rc::clone(&step);
            let wa = Rc::clone(&win_alive);
            timeout_add_ms(18, move || {
                if !wa.get() {
                    return false;
                }
                let mut c = st.borrow_mut();
                *c += 1;
                let op = (0.35 + *c as f64 * 0.22).min(1.0);
                gtk_widget_set_opacity(win, op);
                op < 1.0
            });
        }

        win
    }
}

pub fn home_dir() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/home/dionipe"))
}

// ============================================================================
// 1. Akar - File Manager (BijiOS Native Liquid Glass File Manager)
// ============================================================================
pub fn open_finder(app: *mut GtkApplication) {
    open_finder_at(app, home_dir());
}

fn read_sshfs_mounts() -> Vec<(String, PathBuf)> {
    let mut mounts = Vec::new();
    let base = home_dir().join("Mounts");
    let base_s = base.to_string_lossy().to_string();
    if let Ok(txt) = fs::read_to_string("/proc/mounts") {
        for line in txt.lines() {
            let p: Vec<&str> = line.split_whitespace().collect();
            if p.len() >= 3 {
                let target = p[1].replace("\\040", " ");
                let fstype = p[2];
                if fstype.contains("sshfs") || target.starts_with(&base_s) {
                    let path = PathBuf::from(&target);
                    let name = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("remote")
                        .to_string();
                    if !mounts.iter().any(|(_, mp): &(String, PathBuf)| mp == &path) {
                        mounts.push((name, path));
                    }
                }
            }
        }
    }
    mounts
}

#[derive(Clone)]
enum AkarClipOp {
    Copy(PathBuf),
    Move(PathBuf),
}

#[derive(Clone)]
struct AkarTab {
    id: u32,
    dir: PathBuf,
}

fn copy_path_recursive(src: &std::path::Path, dst: &std::path::Path) -> std::io::Result<()> {
    if src.is_dir() {
        fs::create_dir_all(dst)?;
        for ent in fs::read_dir(src)? {
            let ent = ent?;
            let child_src = ent.path();
            let child_dst = dst.join(ent.file_name());
            copy_path_recursive(&child_src, &child_dst)?;
        }
        Ok(())
    } else {
        fs::copy(src, dst).map(|_| ())
    }
}

fn unique_dest_path(dir: &std::path::Path, name: &str) -> PathBuf {
    let cand = dir.join(name);
    if !cand.exists() {
        return cand;
    }
    let p = std::path::Path::new(name);
    let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or(name);
    let ext = p.extension().and_then(|s| s.to_str());
    for i in 1..1000 {
        let suffix = if i == 1 {
            " (copy)".to_string()
        } else {
            format!(" (copy {})", i)
        };
        let fname = match ext {
            Some(e) if !e.is_empty() => format!("{}{}.{}", stem, suffix, e),
            _ => format!("{}{}", stem, suffix),
        };
        let next = dir.join(fname);
        if !next.exists() {
            return next;
        }
    }
    dir.join(format!("{}_copy", name))
}

fn move_path_safe(src: &std::path::Path, dst: &std::path::Path) -> std::io::Result<()> {
    if fs::rename(src, dst).is_ok() {
        return Ok(());
    }
    copy_path_recursive(src, dst)?;
    if src.is_dir() {
        fs::remove_dir_all(src)
    } else {
        fs::remove_file(src)
    }
}

unsafe fn open_akar_name_dialog(
    app: *mut GtkApplication,
    parent_win: *mut GtkWidget,
    title: &str,
    prompt: &str,
    initial: &str,
    btn_label: &str,
    on_submit: Box<dyn Fn(String)>,
) {
    let dlg = create_app_window(app, title, 400, 168);
    gtk_window_set_resizable(dlg, 0);
    gtk_window_set_modal(dlg, 1);
    gtk_window_set_transient_for(dlg, parent_win);

    let root = new_box(GTK_ORIENTATION_VERTICAL, 10, &[]);
    set_margin_all(root, 16);

    let lbl = new_label(prompt, &["app-section-title"]);
    gtk_label_set_xalign(lbl, 0.0);
    let entry = gtk_entry_new();
    set_entry_text(entry, initial);

    let btns = new_box(GTK_ORIENTATION_HORIZONTAL, 8, &[]);
    gtk_widget_set_halign(btns, GTK_ALIGN_END);
    let btn_cancel = new_button("Cancel", &["toolbar-btn"]);
    let btn_ok = new_button(btn_label, &["primary-btn"]);
    gtk_box_append(btns, btn_cancel);
    gtk_box_append(btns, btn_ok);

    gtk_box_append(root, lbl);
    gtk_box_append(root, entry);
    gtk_box_append(root, btns);

    connect_signal(btn_cancel, "clicked", move || gtk_window_close(dlg));

    let submit_cb: Rc<dyn Fn()> = Rc::new(move || {
        let name = get_entry_text(entry).trim().to_string();
        if !name.is_empty() && !name.contains('/') {
            on_submit(name);
            gtk_window_close(dlg);
        }
    });
    {
        let sc = Rc::clone(&submit_cb);
        connect_signal(btn_ok, "clicked", move || sc());
    }
    {
        let sc = Rc::clone(&submit_cb);
        connect_signal(entry, "activate", move || sc());
    }

    gtk_window_set_child(dlg, root);
    gtk_window_present(dlg);
    gtk_widget_grab_focus(entry);
}

fn open_item_in_app(app: *mut GtkApplication, p_open: &PathBuf) {
    let lower = p_open.to_string_lossy().to_lowercase();
    if lower.ends_with(".docx") || lower.ends_with(".doc") || lower.ends_with(".odt") || lower.ends_with(".rtf") {
        open_wordcraft(app, Some(p_open.clone()));
    } else if lower.ends_with(".xlsx") || lower.ends_with(".xls") || lower.ends_with(".csv") || lower.ends_with(".ods") {
        open_gridcraft_with_file(app, Some(p_open.clone()));
    } else if lower.ends_with(".pptx") || lower.ends_with(".ppt") || lower.ends_with(".odp") {
        open_deckcraft(Some(p_open.clone()));
    } else if lower.ends_with(".pdf") {
        open_pdfcraft(Some(p_open.clone()));
    } else if lower.ends_with(".txt") || lower.ends_with(".md") || lower.ends_with(".rs") || lower.ends_with(".json") || lower.ends_with(".toon") {
        open_wordcraft(app, Some(p_open.clone()));
    } else {
        let _ = Command::new("xdg-open")
            .arg(p_open)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
    }
}

fn is_system_root_path(path: &std::path::Path) -> bool {
    !path.starts_with(home_dir())
}

unsafe fn open_root_warning_dialog(
    app: *mut GtkApplication,
    parent_win: *mut GtkWidget,
    target_dir: &std::path::Path,
    on_proceed: Box<dyn Fn()>,
) {
    let dlg = create_app_window(app, "⚠️ System Root Folder Warning", 460, 215);
    gtk_window_set_resizable(dlg, 0);
    gtk_window_set_modal(dlg, 1);
    gtk_window_set_transient_for(dlg, parent_win);

    let root = new_box(GTK_ORIENTATION_VERTICAL, 12, &[]);
    set_margin_all(root, 18);

    let hdr = new_label("⚠️ Protected System Directory", &["app-heading"]);
    gtk_label_set_xalign(hdr, 0.0);

    let msg = new_label(
        &format!(
            "You are about to access a protected Linux system folder ({}).\nModifying, moving, or deleting system files can make BijiOS unstable or unbootable. Please proceed with caution.",
            target_dir.display()
        ),
        &["kv-val"],
    );
    gtk_label_set_xalign(msg, 0.0);
    gtk_label_set_wrap(msg, 1);

    let btns = new_box(GTK_ORIENTATION_HORIZONTAL, 8, &[]);
    gtk_widget_set_halign(btns, GTK_ALIGN_END);
    let btn_cancel = new_button("Cancel", &["toolbar-btn"]);
    let btn_ok = new_button("⚠️ Proceed Anyway", &["primary-btn"]);
    gtk_box_append(btns, btn_cancel);
    gtk_box_append(btns, btn_ok);

    gtk_box_append(root, hdr);
    gtk_box_append(root, msg);
    gtk_box_append(root, btns);

    connect_signal(btn_cancel, "clicked", move || gtk_window_close(dlg));
    connect_signal(btn_ok, "clicked", move || {
        on_proceed();
        gtk_window_close(dlg);
    });

    gtk_window_set_child(dlg, root);
    gtk_window_present(dlg);
}

unsafe fn open_about_akar(app: *mut GtkApplication, parent_win: *mut GtkWidget) {
    let dlg = create_app_window(app, "📁 About Akar", 580, 560);
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
    let title = new_label("Akar — BijiOS Native File Manager", &["app-heading"]);
    gtk_label_set_xalign(title, 0.0);
    let sub = new_label(
        "Version 1.0.0 (BijiOS \"Padi\")  •  Pure Rust + Native GTK4",
        &["app-muted"],
    );
    gtk_label_set_xalign(sub, 0.0);
    gtk_box_append(tcol, title);
    gtk_box_append(tcol, sub);
    gtk_box_append(hdr_card, logo);
    gtk_box_append(hdr_card, tcol);
    gtk_box_append(root, hdr_card);

    // Specifications Card
    let spec_card = new_box(GTK_ORIENTATION_VERTICAL, 6, &["app-card"]);
    let s_hdr = new_label("📋 SPECIFICATIONS", &["app-section-title"]);
    gtk_label_set_xalign(s_hdr, 0.0);
    gtk_box_append(spec_card, s_hdr);

    let specs = [
        ("Application", "Akar — Sovereign Liquid Glass File Manager"),
        ("Desktop Suite", "BijiOS Desktop v1.0.0 (Codename \"Padi\")"),
        ("Engine", "100% Pure Rust + Native GTK4 (Zero Chromium)"),
        ("View Modes", "List View, Expandable Tree View & Sidebar Folder Tree"),
        ("Remote Storage", "SSHFS Mounts & Direct Integration with Pancer"),
        ("System Safety", "Protected System Root (/) Warning & Banner"),
    ];
    for (k, v) in specs {
        let r = new_box(GTK_ORIENTATION_HORIZONTAL, 10, &[]);
        let kl = new_label(k, &["kv-key"]);
        gtk_label_set_xalign(kl, 0.0);
        gtk_widget_set_size_request(kl, 128, -1);
        let vl = new_label(v, &["kv-val"]);
        gtk_label_set_xalign(vl, 0.0);
        gtk_widget_set_hexpand(vl, 1);
        gtk_box_append(r, kl);
        gtk_box_append(r, vl);
        gtk_box_append(spec_card, r);
    }
    gtk_box_append(root, spec_card);

    // Keyboard Shortcuts Card
    let hk_card = new_box(GTK_ORIENTATION_VERTICAL, 6, &["app-card"]);
    let h_hdr = new_label("⌨️ KEYBOARD SHORTCUTS", &["app-section-title"]);
    gtk_label_set_xalign(h_hdr, 0.0);
    gtk_box_append(hk_card, h_hdr);

    let hk_grid = gtk_grid_new();
    gtk_grid_set_row_spacing(hk_grid, 4);
    gtk_grid_set_column_spacing(hk_grid, 16);
    gtk_grid_set_column_homogeneous(hk_grid, 1);

    let shortcuts: [(&str, &str); 8] = [
        ("Ctrl+T", "New Tab"),
        ("Ctrl+Shift+N", "New Folder"),
        ("Ctrl+C / X", "Copy / Move (Cut)"),
        ("Ctrl+V", "Paste into Folder"),
        ("F2", "Rename Selected"),
        ("Del", "Delete Selected"),
        ("Right-Click", "Context Menu"),
        ("Esc", "Close Menu"),
    ];
    for (i, (key, desc)) in shortcuts.iter().enumerate() {
        let r = new_box(GTK_ORIENTATION_HORIZONTAL, 6, &[]);
        let kl = new_label(key, &["act-proc-mono"]);
        gtk_label_set_xalign(kl, 0.0);
        gtk_widget_set_size_request(kl, 108, -1);
        let dl = new_label(desc, &["kv-val"]);
        gtk_label_set_xalign(dl, 0.0);
        gtk_box_append(r, kl);
        gtk_box_append(r, dl);
        gtk_grid_attach(hk_grid, r, (i % 2) as i32, (i / 2) as i32, 1, 1);
    }
    gtk_box_append(hk_card, hk_grid);
    gtk_box_append(root, hk_card);

    // Open-Source Links Card
    let cred_card = new_box(GTK_ORIENTATION_VERTICAL, 6, &["app-card"]);
    let c_hdr = new_label("📜 OPEN-SOURCE COMPONENTS", &["app-section-title"]);
    gtk_label_set_xalign(c_hdr, 0.0);
    gtk_box_append(cred_card, c_hdr);

    let c_pills = new_box(GTK_ORIENTATION_HORIZONTAL, 6, &[]);
    gtk_box_append(
        c_pills,
        new_link_button("BijiOS Project ↗", "https://github.com/dionipe/BijiOS", &["link-pill"]),
    );
    gtk_box_append(
        c_pills,
        new_link_button("GTK4 Toolkit ↗", "https://www.gtk.org", &["link-pill"]),
    );
    gtk_box_append(
        c_pills,
        new_link_button("SSHFS / FUSE ↗", "https://github.com/libfuse/sshfs", &["link-pill"]),
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

fn collect_tree_entries(
    dir: &std::path::Path,
    depth: usize,
    expanded: &HashSet<PathBuf>,
    out: &mut Vec<(String, PathBuf, bool, u64, usize)>,
) {
    if depth > 4 || out.len() >= 350 {
        return;
    }
    let mut entries: Vec<(String, PathBuf, bool, u64)> = Vec::new();
    if let Ok(rd) = fs::read_dir(dir) {
        for ent in rd.flatten() {
            let p = ent.path();
            let name = ent.file_name().to_string_lossy().to_string();
            if name.starts_with('.') {
                continue;
            }
            let meta = ent.metadata().ok();
            let is_dir = meta.as_ref().map(|m| m.is_dir()).unwrap_or(false);
            let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
            entries.push((name, p, is_dir, size));
        }
    }
    entries.sort_by(|a, b| b.2.cmp(&a.2).then(a.0.to_lowercase().cmp(&b.0.to_lowercase())));
    for (name, path, is_dir, size) in entries {
        if out.len() >= 350 {
            break;
        }
        let exp = is_dir && expanded.contains(&path);
        out.push((name, path.clone(), is_dir, size, depth));
        if exp {
            collect_tree_entries(&path, depth + 1, expanded, out);
        }
    }
}

pub fn open_finder_at(app: *mut GtkApplication, start_dir: PathBuf) {
    unsafe {
        let win = create_app_window(app, "📁  Akar - File Manager", 1000, 640);
        let overlay = gtk_overlay_new();
        let win_col = new_box(GTK_ORIENTATION_VERTICAL, 0, &[]);
        gtk_widget_set_hexpand(win_col, 1);
        gtk_widget_set_vexpand(win_col, 1);
        gtk_overlay_set_child(overlay, win_col);

        // ---------------------------------------------------------------------
        // 1. Akar Menu Bar (File | Edit | Help             [Clipboard Badge])
        // ---------------------------------------------------------------------
        let menubar = new_box(GTK_ORIENTATION_HORIZONTAL, 4, &["akar-menubar"]);
        let btn_file = new_button("File", &["akar-menu-btn"]);
        let btn_edit = new_button("Edit", &["akar-menu-btn"]);
        let btn_help = new_button("Help", &["akar-menu-btn"]);
        let mb_spacer = new_box(GTK_ORIENTATION_HORIZONTAL, 0, &[]);
        gtk_widget_set_hexpand(mb_spacer, 1);
        let clip_badge = new_label("", &["akar-clip-badge"]);
        gtk_widget_set_visible(clip_badge, 0);

        gtk_box_append(menubar, btn_file);
        gtk_box_append(menubar, btn_edit);
        gtk_box_append(menubar, btn_help);
        gtk_box_append(menubar, mb_spacer);
        gtk_box_append(menubar, clip_badge);
        gtk_box_append(win_col, menubar);

        // Transparent click-outside backdrop for dropdown & context menus
        let menu_backdrop = gtk_button_new();
        add_css(menu_backdrop, "pancer-menu-backdrop");
        gtk_widget_set_hexpand(menu_backdrop, 1);
        gtk_widget_set_vexpand(menu_backdrop, 1);
        gtk_widget_set_halign(menu_backdrop, GTK_ALIGN_FILL);
        gtk_widget_set_valign(menu_backdrop, GTK_ALIGN_FILL);
        gtk_widget_set_visible(menu_backdrop, 0);
        gtk_overlay_add_overlay(overlay, menu_backdrop);

        let make_dropdown = |margin_start: i32| -> *mut GtkWidget {
            let panel = new_box(GTK_ORIENTATION_VERTICAL, 2, &["akar-dropdown"]);
            gtk_widget_set_halign(panel, GTK_ALIGN_START);
            gtk_widget_set_valign(panel, GTK_ALIGN_START);
            set_margins(panel, 28, 0, margin_start, 0);
            gtk_widget_set_size_request(panel, 224, -1);
            gtk_widget_set_visible(panel, 0);
            gtk_overlay_add_overlay(overlay, panel);
            panel
        };

        let menu_file = make_dropdown(8);
        let menu_edit = make_dropdown(54);
        let menu_help = make_dropdown(100);
        let ctx_menu = make_dropdown(0);

        let any_menu_open = move || -> bool {
            gtk_widget_get_visible(menu_file) != 0
                || gtk_widget_get_visible(menu_edit) != 0
                || gtk_widget_get_visible(menu_help) != 0
                || gtk_widget_get_visible(ctx_menu) != 0
        };

        let hide_all_menus = move || {
            gtk_widget_set_visible(menu_backdrop, 0);
            gtk_widget_set_visible(menu_file, 0);
            gtk_widget_set_visible(menu_edit, 0);
            gtk_widget_set_visible(menu_help, 0);
            gtk_widget_set_visible(ctx_menu, 0);
        };

        connect_signal(menu_backdrop, "clicked", move || {
            hide_all_menus();
        });
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
        connect_signal(btn_help, "clicked", move || {
            let vis = gtk_widget_get_visible(menu_help) != 0;
            hide_all_menus();
            if !vis {
                gtk_widget_set_visible(menu_backdrop, 1);
                gtk_widget_set_visible(menu_help, 1);
            }
        });

        let new_menu_item = |icon: &str, text: &str, accel: &str| -> *mut GtkWidget {
            let btn = gtk_button_new();
            add_css(btn, "akar-menu-item");
            let row = new_box(GTK_ORIENTATION_HORIZONTAL, 8, &[]);
            gtk_widget_set_hexpand(row, 1);
            let ic = new_label(icon, &[]);
            let lbl = new_label(text, &[]);
            gtk_label_set_xalign(lbl, 0.0);
            gtk_widget_set_hexpand(lbl, 1);
            let acc = new_label(accel, &["akar-accel"]);
            gtk_label_set_xalign(acc, 1.0);
            gtk_box_append(row, ic);
            gtk_box_append(row, lbl);
            gtk_box_append(row, acc);
            gtk_button_set_child(btn, row);
            btn
        };

        // ---------------------------------------------------------------------
        // 2. Multi-Tab Bar (| 📁 Home × | 📁 Documents × | + |)
        // ---------------------------------------------------------------------
        let tabbar = new_box(GTK_ORIENTATION_HORIZONTAL, 5, &["akar-tabbar"]);
        let tabs_box = new_box(GTK_ORIENTATION_HORIZONTAL, 4, &[]);
        let btn_add_tab = new_button("+", &["akar-add-tab"]);
        let tip_at = cstr("New Tab (Ctrl+T)");
        gtk_widget_set_tooltip_text(btn_add_tab, tip_at.as_ptr());
        gtk_box_append(tabbar, tabs_box);
        gtk_box_append(tabbar, btn_add_tab);
        gtk_box_append(win_col, tabbar);

        // ---------------------------------------------------------------------
        // 3. Split Body: [ Sidebar | Main Area ]
        // ---------------------------------------------------------------------
        let root = new_box(GTK_ORIENTATION_HORIZONTAL, 0, &[]);
        gtk_widget_set_hexpand(root, 1);
        gtk_widget_set_vexpand(root, 1);
        gtk_box_append(win_col, root);

        // Sidebar
        let sidebar = new_box(GTK_ORIENTATION_VERTICAL, 3, &["app-sidebar"]);
        gtk_widget_set_size_request(sidebar, 210, -1);
        gtk_widget_set_hexpand(sidebar, 0);
        let sb_hdr1 = new_label("MY STORAGE", &["sidebar-section-hdr"]);
        gtk_label_set_xalign(sb_hdr1, 0.0);
        gtk_box_append(sidebar, sb_hdr1);

        let init_dir = if start_dir.is_dir() { start_dir } else { home_dir() };
        let cur_dir: Rc<RefCell<PathBuf>> = Rc::new(RefCell::new(init_dir.clone()));
        let selected_path: Rc<RefCell<Option<PathBuf>>> = Rc::new(RefCell::new(None));
        let clipboard: Rc<RefCell<Option<AkarClipOp>>> = Rc::new(RefCell::new(None));
        let tabs: Rc<RefCell<Vec<AkarTab>>> = Rc::new(RefCell::new(vec![AkarTab {
            id: 1,
            dir: init_dir,
        }]));
        let active_tab_id: Rc<std::cell::Cell<u32>> = Rc::new(std::cell::Cell::new(1));
        let next_tab_id: Rc<std::cell::Cell<u32>> = Rc::new(std::cell::Cell::new(2));

        let update_clip_badge: Rc<dyn Fn()> = {
            let cb = Rc::clone(&clipboard);
            Rc::new(move || {
                match cb.borrow().as_ref() {
                    Some(AkarClipOp::Copy(p)) => {
                        let nm = p.file_name().and_then(|s| s.to_str()).unwrap_or("item");
                        set_label_text(clip_badge, &format!("📋 Copy: {}", nm));
                        gtk_widget_set_visible(clip_badge, 1);
                    }
                    Some(AkarClipOp::Move(p)) => {
                        let nm = p.file_name().and_then(|s| s.to_str()).unwrap_or("item");
                        set_label_text(clip_badge, &format!("✂️ Move: {}", nm));
                        gtk_widget_set_visible(clip_badge, 1);
                    }
                    None => {
                        gtk_widget_set_visible(clip_badge, 0);
                    }
                }
            })
        };

        // Main area
        let main_col = new_box(GTK_ORIENTATION_VERTICAL, 0, &[]);
        gtk_widget_set_hexpand(main_col, 1);
        gtk_widget_set_vexpand(main_col, 1);

        // Toolbar (compact, proportional, uniform height)
        let toolbar = new_box(GTK_ORIENTATION_HORIZONTAL, 6, &["app-toolbar"]);
        let btn_up = new_button("⬆", &["toolbar-btn"]);
        let tip_up = cstr("Go to parent folder");
        gtk_widget_set_tooltip_text(btn_up, tip_up.as_ptr());

        let path_entry = gtk_entry_new();
        gtk_widget_set_hexpand(path_entry, 1);
        gtk_widget_set_size_request(path_entry, 260, -1);
        set_entry_text(path_entry, &cur_dir.borrow().to_string_lossy());

        let btn_tree_mode = new_button("🌳 Tree", &["toolbar-btn"]);
        let tip_tm = cstr("Switch between List View and Expandable Tree View");
        gtk_widget_set_tooltip_text(btn_tree_mode, tip_tm.as_ptr());

        let btn_new_folder = new_button("+ Folder", &["toolbar-btn"]);
        let tip_nf = cstr("Create New Folder (Ctrl+Shift+N)");
        gtk_widget_set_tooltip_text(btn_new_folder, tip_nf.as_ptr());

        let btn_new_doc = new_button("+ New Doc", &["primary-btn"]);
        let tip_nd = cstr("Create New WordCraft Document");
        gtk_widget_set_tooltip_text(btn_new_doc, tip_nd.as_ptr());

        let btn_refresh = new_button("🔄", &["toolbar-btn"]);
        let tip_rf = cstr("Refresh Directory");
        gtk_widget_set_tooltip_text(btn_refresh, tip_rf.as_ptr());

        gtk_box_append(toolbar, btn_up);
        gtk_box_append(toolbar, path_entry);
        gtk_box_append(toolbar, btn_tree_mode);
        gtk_box_append(toolbar, btn_new_folder);
        gtk_box_append(toolbar, btn_new_doc);
        gtk_box_append(toolbar, btn_refresh);
        gtk_box_append(main_col, toolbar);

        // System Root Warning Banner (visible whenever browsing outside $HOME)
        let root_banner = new_box(GTK_ORIENTATION_HORIZONTAL, 10, &["akar-root-banner"]);
        let root_banner_lbl = new_label(
            "⚠️ System Root Folder (/) — Protected Linux OS directory. Avoid modifying or deleting critical system files.",
            &["akar-root-banner-lbl"],
        );
        gtk_label_set_xalign(root_banner_lbl, 0.0);
        gtk_widget_set_hexpand(root_banner_lbl, 1);
        let btn_go_home = new_button("🏠 Go Home", &["toolbar-btn"]);
        gtk_box_append(root_banner, root_banner_lbl);
        gtk_box_append(root_banner, btn_go_home);
        gtk_widget_set_visible(root_banner, 0);
        gtk_box_append(main_col, root_banner);

        // Column Header Row (Name | Size | Kind)
        let col_hdr = new_box(GTK_ORIENTATION_HORIZONTAL, 10, &["finder-col-header"]);
        let hdr_name = new_label("Name", &["finder-col-lbl"]);
        gtk_label_set_xalign(hdr_name, 0.0);
        gtk_widget_set_hexpand(hdr_name, 1);
        let hdr_size = new_label("Size", &["finder-col-lbl"]);
        gtk_label_set_xalign(hdr_size, 1.0);
        gtk_widget_set_size_request(hdr_size, 84, -1);
        let hdr_kind = new_label("Kind", &["finder-col-lbl"]);
        gtk_label_set_xalign(hdr_kind, 0.0);
        gtk_widget_set_size_request(hdr_kind, 130, -1);
        let hdr_act = new_label("", &["finder-col-lbl"]);
        gtk_widget_set_size_request(hdr_act, 34, -1);

        gtk_box_append(col_hdr, hdr_name);
        gtk_box_append(col_hdr, hdr_size);
        gtk_box_append(col_hdr, hdr_kind);
        gtk_box_append(col_hdr, hdr_act);
        gtk_box_append(main_col, col_hdr);

        // Scrolled file list (clean zebra rows)
        let scroller = gtk_scrolled_window_new();
        gtk_widget_set_hexpand(scroller, 1);
        gtk_widget_set_vexpand(scroller, 1);
        let list_box = new_box(GTK_ORIENTATION_VERTICAL, 0, &[]);
        gtk_scrolled_window_set_child(scroller, list_box);
        gtk_box_append(main_col, scroller);

        // Bottom macOS Status Bar
        let status_bar = gtk_center_box_new();
        add_css(status_bar, "finder-statusbar");
        let status_lbl = new_label("0 items", &[]);
        let status_right = new_label("BijiOS Sovereign Storage", &[]);
        gtk_center_box_set_start_widget(status_bar, status_lbl);
        gtk_center_box_set_end_widget(status_bar, status_right);
        gtk_box_append(main_col, status_bar);

        // Forward-declared closures for populate, tab bar rebuild, & sidebar tree
        let populate: Rc<RefCell<Box<dyn Fn()>>> = Rc::new(RefCell::new(Box::new(|| {})));
        let rebuild_tabs: Rc<RefCell<Box<dyn Fn()>>> = Rc::new(RefCell::new(Box::new(|| {})));
        let populate_tree_sb: Rc<RefCell<Box<dyn Fn()>>> = Rc::new(RefCell::new(Box::new(|| {})));

        let tree_mode: Rc<std::cell::Cell<bool>> = Rc::new(std::cell::Cell::new(false));
        let main_expanded: Rc<RefCell<HashSet<PathBuf>>> = Rc::new(RefCell::new(HashSet::new()));
        let mut init_sb_exp = HashSet::new();
        init_sb_exp.insert(home_dir());
        let sb_expanded: Rc<RefCell<HashSet<PathBuf>>> = Rc::new(RefCell::new(init_sb_exp));
        let root_warn_ack: Rc<std::cell::Cell<bool>> = Rc::new(std::cell::Cell::new(false));

        // Centralized navigation with System Root (/) protection warning
        let navigate_to: Rc<dyn Fn(PathBuf)> = {
            let cd = Rc::clone(&cur_dir);
            let sel = Rc::clone(&selected_path);
            let pp = Rc::clone(&populate);
            let pt = Rc::clone(&populate_tree_sb);
            let ack = Rc::clone(&root_warn_ack);
            Rc::new(move |target: PathBuf| {
                hide_all_menus();
                if !target.is_dir() {
                    return;
                }
                if is_system_root_path(&target) && !ack.get() {
                    let cd2 = Rc::clone(&cd);
                    let sel2 = Rc::clone(&sel);
                    let pp2 = Rc::clone(&pp);
                    let pt2 = Rc::clone(&pt);
                    let ack2 = Rc::clone(&ack);
                    let t2 = target.clone();
                    open_root_warning_dialog(
                        app,
                        win,
                        &target,
                        Box::new(move || {
                            ack2.set(true);
                            *cd2.borrow_mut() = t2.clone();
                            *sel2.borrow_mut() = None;
                            (pp2.borrow())();
                            (pt2.borrow())();
                        }),
                    );
                    return;
                }
                *cd.borrow_mut() = target;
                *sel.borrow_mut() = None;
                (pp.borrow())();
                (pt.borrow())();
            })
        };

        {
            let nav_h = Rc::clone(&navigate_to);
            connect_signal(btn_go_home, "clicked", move || {
                nav_h(home_dir());
            });
        }
        {
            let tm = Rc::clone(&tree_mode);
            let pp = Rc::clone(&populate);
            connect_signal(btn_tree_mode, "clicked", move || {
                hide_all_menus();
                let next = !tm.get();
                tm.set(next);
                let lbl = cstr(if next { "☰ List" } else { "🌳 Tree" });
                gtk_button_set_label(btn_tree_mode, lbl.as_ptr());
                (pp.borrow())();
            });
        }

        // Core File / Edit Actions (shared by Menu Bar, Context Menu & Hotkeys)
        let act_new_folder: Rc<dyn Fn()> = {
            let cd = Rc::clone(&cur_dir);
            let pp = Rc::clone(&populate);
            Rc::new(move || {
                hide_all_menus();
                let base = cd.borrow().clone();
                let pp2 = Rc::clone(&pp);
                open_akar_name_dialog(
                    app,
                    win,
                    "📁 New Folder",
                    "Enter new folder name:",
                    "New Folder",
                    "Create",
                    Box::new(move |name| {
                        let target = unique_dest_path(&base, &name);
                        let _ = fs::create_dir_all(&target);
                        (pp2.borrow())();
                    }),
                );
            })
        };

        let act_new_tab: Rc<dyn Fn()> = {
            let tbs = Rc::clone(&tabs);
            let aid = Rc::clone(&active_tab_id);
            let nid = Rc::clone(&next_tab_id);
            let cd = Rc::clone(&cur_dir);
            let sel = Rc::clone(&selected_path);
            let pp = Rc::clone(&populate);
            let rt = Rc::clone(&rebuild_tabs);
            Rc::new(move || {
                hide_all_menus();
                let id = nid.get();
                nid.set(id + 1);
                let dir = cd.borrow().clone();
                tbs.borrow_mut().push(AkarTab { id, dir: dir.clone() });
                aid.set(id);
                *cd.borrow_mut() = dir;
                *sel.borrow_mut() = None;
                (rt.borrow())();
                (pp.borrow())();
            })
        };

        let act_copy: Rc<dyn Fn()> = {
            let sel = Rc::clone(&selected_path);
            let cb = Rc::clone(&clipboard);
            let ucb = Rc::clone(&update_clip_badge);
            Rc::new(move || {
                hide_all_menus();
                if let Some(p) = sel.borrow().clone() {
                    *cb.borrow_mut() = Some(AkarClipOp::Copy(p));
                    ucb();
                }
            })
        };

        let act_move: Rc<dyn Fn()> = {
            let sel = Rc::clone(&selected_path);
            let cb = Rc::clone(&clipboard);
            let ucb = Rc::clone(&update_clip_badge);
            Rc::new(move || {
                hide_all_menus();
                if let Some(p) = sel.borrow().clone() {
                    *cb.borrow_mut() = Some(AkarClipOp::Move(p));
                    ucb();
                }
            })
        };

        let act_paste: Rc<dyn Fn()> = {
            let cd = Rc::clone(&cur_dir);
            let cb = Rc::clone(&clipboard);
            let ucb = Rc::clone(&update_clip_badge);
            let pp = Rc::clone(&populate);
            Rc::new(move || {
                hide_all_menus();
                let op = cb.borrow().clone();
                let Some(op) = op else { return; };
                let dest_dir = cd.borrow().clone();
                match op {
                    AkarClipOp::Copy(src) => {
                        if let Some(fname) = src.file_name().and_then(|s| s.to_str()) {
                            let dst = unique_dest_path(&dest_dir, fname);
                            let _ = copy_path_recursive(&src, &dst);
                        }
                    }
                    AkarClipOp::Move(src) => {
                        if let Some(fname) = src.file_name().and_then(|s| s.to_str()) {
                            let dst = unique_dest_path(&dest_dir, fname);
                            if move_path_safe(&src, &dst).is_ok() {
                                *cb.borrow_mut() = None;
                                ucb();
                            }
                        }
                    }
                }
                (pp.borrow())();
            })
        };

        let act_rename: Rc<dyn Fn()> = {
            let sel = Rc::clone(&selected_path);
            let pp = Rc::clone(&populate);
            Rc::new(move || {
                hide_all_menus();
                let Some(target) = sel.borrow().clone() else { return; };
                let cur_name = target
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_string();
                let parent = target.parent().map(|p| p.to_path_buf());
                let Some(parent) = parent else { return; };
                let pp2 = Rc::clone(&pp);
                let sel2 = Rc::clone(&sel);
                open_akar_name_dialog(
                    app,
                    win,
                    "✏️ Rename Item",
                    &format!("Rename \"{}\" to:", cur_name),
                    &cur_name,
                    "Rename",
                    Box::new(move |new_name| {
                        let new_path = parent.join(&new_name);
                        if new_path != target && fs::rename(&target, &new_path).is_ok() {
                            *sel2.borrow_mut() = Some(new_path);
                            (pp2.borrow())();
                        }
                    }),
                );
            })
        };

        let act_delete: Rc<dyn Fn()> = {
            let sel = Rc::clone(&selected_path);
            let pp = Rc::clone(&populate);
            Rc::new(move || {
                hide_all_menus();
                let Some(target) = sel.borrow_mut().take() else { return; };
                if target.is_dir() {
                    let _ = fs::remove_dir_all(&target);
                } else {
                    let _ = fs::remove_file(&target);
                }
                (pp.borrow())();
            })
        };

        // Helper: show right-click context menu (for an item or empty space)
        let show_ctx_menu: Rc<dyn Fn(*mut GtkWidget, f64, f64, Option<(PathBuf, bool)>)> = {
            let cd = Rc::clone(&cur_dir);
            let sel = Rc::clone(&selected_path);
            let pp = Rc::clone(&populate);
            let f_nf = Rc::clone(&act_new_folder);
            let f_nt = Rc::clone(&act_new_tab);
            let f_cp = Rc::clone(&act_copy);
            let f_mv = Rc::clone(&act_move);
            let f_ps = Rc::clone(&act_paste);
            let f_rn = Rc::clone(&act_rename);
            let f_dl = Rc::clone(&act_delete);
            Rc::new(move |src_widget, x, y, item_opt| {
                hide_all_menus();
                clear_box_children(ctx_menu);

                if let Some((path, is_dir)) = item_opt {
                    *sel.borrow_mut() = Some(path.clone());

                    let mi_open = new_menu_item("📂", "Open", "");
                    let mi_cp = new_menu_item("📋", "Copy", "Ctrl+C");
                    let mi_mv = new_menu_item("✂️", "Move (Cut)", "Ctrl+X");
                    let mi_ps = new_menu_item("📥", "Paste", "Ctrl+V");
                    let mi_rn = new_menu_item("✏️", "Rename", "F2");
                    let mi_dl = new_menu_item("🗑️", "Delete", "Del");
                    let mi_nf = new_menu_item("📁", "New Folder", "Ctrl+Shift+N");
                    let mi_nt = new_menu_item("➕", "New Tab", "Ctrl+T");

                    gtk_box_append(ctx_menu, mi_open);
                    gtk_box_append(ctx_menu, gtk_separator_new(GTK_ORIENTATION_HORIZONTAL));
                    gtk_box_append(ctx_menu, mi_cp);
                    gtk_box_append(ctx_menu, mi_mv);
                    gtk_box_append(ctx_menu, mi_ps);
                    gtk_box_append(ctx_menu, mi_rn);
                    gtk_box_append(ctx_menu, mi_dl);
                    gtk_box_append(ctx_menu, gtk_separator_new(GTK_ORIENTATION_HORIZONTAL));
                    gtk_box_append(ctx_menu, mi_nf);
                    gtk_box_append(ctx_menu, mi_nt);

                    {
                        let p_o = path.clone();
                        let cd_o = Rc::clone(&cd);
                        let pp_o = Rc::clone(&pp);
                        connect_signal(mi_open, "clicked", move || {
                            hide_all_menus();
                            if is_dir {
                                *cd_o.borrow_mut() = p_o.clone();
                                (pp_o.borrow())();
                            } else {
                                open_item_in_app(app, &p_o);
                            }
                        });
                    }
                    {
                        let f = Rc::clone(&f_cp);
                        connect_signal(mi_cp, "clicked", move || f());
                    }
                    {
                        let f = Rc::clone(&f_mv);
                        connect_signal(mi_mv, "clicked", move || f());
                    }
                    {
                        let f = Rc::clone(&f_ps);
                        connect_signal(mi_ps, "clicked", move || f());
                    }
                    {
                        let f = Rc::clone(&f_rn);
                        connect_signal(mi_rn, "clicked", move || f());
                    }
                    {
                        let f = Rc::clone(&f_dl);
                        connect_signal(mi_dl, "clicked", move || f());
                    }
                    {
                        let f = Rc::clone(&f_nf);
                        connect_signal(mi_nf, "clicked", move || f());
                    }
                    {
                        let f = Rc::clone(&f_nt);
                        connect_signal(mi_nt, "clicked", move || f());
                    }

                    let (ox, oy) = widget_point_in_overlay(src_widget, overlay, x, y, 228, 290);
                    set_margins(ctx_menu, oy, 0, ox, 0);
                } else {
                    let mi_nf = new_menu_item("📁", "New Folder", "Ctrl+Shift+N");
                    let mi_ps = new_menu_item("📥", "Paste", "Ctrl+V");
                    let mi_nt = new_menu_item("➕", "New Tab", "Ctrl+T");
                    let mi_rf = new_menu_item("🔄", "Refresh", "");

                    gtk_box_append(ctx_menu, mi_nf);
                    gtk_box_append(ctx_menu, mi_ps);
                    gtk_box_append(ctx_menu, gtk_separator_new(GTK_ORIENTATION_HORIZONTAL));
                    gtk_box_append(ctx_menu, mi_nt);
                    gtk_box_append(ctx_menu, mi_rf);

                    {
                        let f = Rc::clone(&f_nf);
                        connect_signal(mi_nf, "clicked", move || f());
                    }
                    {
                        let f = Rc::clone(&f_ps);
                        connect_signal(mi_ps, "clicked", move || f());
                    }
                    {
                        let f = Rc::clone(&f_nt);
                        connect_signal(mi_nt, "clicked", move || f());
                    }
                    {
                        let pp_r = Rc::clone(&pp);
                        connect_signal(mi_rf, "clicked", move || {
                            hide_all_menus();
                            (pp_r.borrow())();
                        });
                    }

                    let (ox, oy) = widget_point_in_overlay(src_widget, overlay, x, y, 228, 150);
                    set_margins(ctx_menu, oy, 0, ox, 0);
                }

                gtk_widget_set_visible(menu_backdrop, 1);
                gtk_widget_set_visible(ctx_menu, 1);
            })
        };

        // Rebuild Tab Bar closure
        {
            let tbs = Rc::clone(&tabs);
            let aid = Rc::clone(&active_tab_id);
            let cd = Rc::clone(&cur_dir);
            let sel = Rc::clone(&selected_path);
            let pp = Rc::clone(&populate);
            let rt_self = Rc::clone(&rebuild_tabs);
            *rebuild_tabs.borrow_mut() = Box::new(move || {
                clear_box_children(tabs_box);
                let cur_id = aid.get();
                let snapshot = tbs.borrow().clone();
                let multi = snapshot.len() > 1;
                for tab in snapshot {
                    let is_act = tab.id == cur_id;
                    let pill = new_box(
                        GTK_ORIENTATION_HORIZONTAL,
                        2,
                        &[if is_act { "akar-tab-pill-active" } else { "akar-tab-pill" }],
                    );
                    let title = tab
                        .dir
                        .file_name()
                        .and_then(|s| s.to_str())
                        .filter(|s| !s.is_empty())
                        .map(|s| format!("📁 {}", s))
                        .unwrap_or_else(|| "💻 /".to_string());
                    let btn = new_button(&title, &["akar-tab-btn"]);
                    gtk_box_append(pill, btn);

                    {
                        let tid = tab.id;
                        let tdir = tab.dir.clone();
                        let aid_c = Rc::clone(&aid);
                        let cd_c = Rc::clone(&cd);
                        let sel_c = Rc::clone(&sel);
                        let pp_c = Rc::clone(&pp);
                        let rt_c = Rc::clone(&rt_self);
                        connect_signal(btn, "clicked", move || {
                            hide_all_menus();
                            aid_c.set(tid);
                            *cd_c.borrow_mut() = tdir.clone();
                            *sel_c.borrow_mut() = None;
                            (rt_c.borrow())();
                            (pp_c.borrow())();
                        });
                    }

                    if multi {
                        let btn_x = new_button("×", &["akar-tab-close"]);
                        gtk_box_append(pill, btn_x);
                        let tid = tab.id;
                        let tbs_c = Rc::clone(&tbs);
                        let aid_c = Rc::clone(&aid);
                        let cd_c = Rc::clone(&cd);
                        let sel_c = Rc::clone(&sel);
                        let pp_c = Rc::clone(&pp);
                        let rt_c = Rc::clone(&rt_self);
                        connect_signal(btn_x, "clicked", move || {
                            hide_all_menus();
                            let next_dir = {
                                let mut list = tbs_c.borrow_mut();
                                if list.len() <= 1 {
                                    return;
                                }
                                if let Some(pos) = list.iter().position(|t| t.id == tid) {
                                    list.remove(pos);
                                }
                                if aid_c.get() == tid {
                                    let fallback = list.last().cloned();
                                    if let Some(fb) = fallback {
                                        aid_c.set(fb.id);
                                        Some(fb.dir)
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }
                            };
                            if let Some(d) = next_dir {
                                *cd_c.borrow_mut() = d;
                                *sel_c.borrow_mut() = None;
                            }
                            (rt_c.borrow())();
                            (pp_c.borrow())();
                        });
                    }
                    gtk_box_append(tabs_box, pill);
                }
            });
        }

        // Populate directory function (supports both flat List View & expandable Tree View)
        let populate_clone = Rc::clone(&populate);
        let cur_dir_p = Rc::clone(&cur_dir);
        let sel_p = Rc::clone(&selected_path);
        let tbs_p = Rc::clone(&tabs);
        let aid_p = Rc::clone(&active_tab_id);
        let rt_p = Rc::clone(&rebuild_tabs);
        let ctx_p = Rc::clone(&show_ctx_menu);
        let tm_p = Rc::clone(&tree_mode);
        let mexp_p = Rc::clone(&main_expanded);
        let nav_p = Rc::clone(&navigate_to);

        *populate.borrow_mut() = Box::new(move || {
            clear_box_children(list_box);
            let dir = cur_dir_p.borrow().clone();
            set_entry_text(path_entry, &dir.to_string_lossy());
            gtk_widget_set_visible(root_banner, if is_system_root_path(&dir) { 1 } else { 0 });

            // Sync active tab's directory & label
            {
                let cur_id = aid_p.get();
                let mut changed = false;
                if let Ok(mut list) = tbs_p.try_borrow_mut() {
                    if let Some(t) = list.iter_mut().find(|t| t.id == cur_id) {
                        if t.dir != dir {
                            t.dir = dir.clone();
                            changed = true;
                        }
                    }
                }
                if changed {
                    (rt_p.borrow())();
                }
            }

            let cur_sel = sel_p.borrow().clone();
            let is_tree = tm_p.get();
            let mut entries: Vec<(String, PathBuf, bool, u64, usize)> = Vec::new();
            if is_tree {
                let exp_snap = mexp_p.borrow().clone();
                collect_tree_entries(&dir, 0, &exp_snap, &mut entries);
            } else {
                let mut flat: Vec<(String, PathBuf, bool, u64)> = Vec::new();
                if let Ok(rd) = fs::read_dir(&dir) {
                    for ent in rd.flatten() {
                        let p = ent.path();
                        let name = ent.file_name().to_string_lossy().to_string();
                        if name.starts_with('.') {
                            continue;
                        }
                        let meta = ent.metadata().ok();
                        let is_dir = meta.as_ref().map(|m| m.is_dir()).unwrap_or(false);
                        let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
                        flat.push((name, p, is_dir, size));
                    }
                }
                flat.sort_by(|a, b| b.2.cmp(&a.2).then(a.0.to_lowercase().cmp(&b.0.to_lowercase())));
                entries = flat
                    .into_iter()
                    .map(|(n, p, d, s)| (n, p, d, s, 0usize))
                    .collect();
            }

            set_label_text(
                status_lbl,
                &format!(
                    "{} items{}",
                    entries.len(),
                    if is_tree { " (Tree View)" } else { "" }
                ),
            );
            set_label_text(status_right, &format!("Location: {}", dir.display()));

            if entries.is_empty() {
                let empty = new_label("This folder is empty. Right-click to create a folder or paste.", &["app-muted"]);
                set_margin_all(empty, 28);
                gtk_box_append(list_box, empty);
                return;
            }

            for (idx, (name, path, is_dir, size, depth)) in entries.into_iter().enumerate() {
                let is_sel = cur_sel.as_ref() == Some(&path);
                let row_cls = if is_sel {
                    "finder-row-selected"
                } else if idx % 2 == 0 {
                    "finder-row-even"
                } else {
                    "finder-row-odd"
                };
                let row = new_box(GTK_ORIENTATION_HORIZONTAL, 8, &[row_cls]);

                if is_tree {
                    if depth > 0 {
                        let indent = new_box(GTK_ORIENTATION_HORIZONTAL, 0, &[]);
                        gtk_widget_set_size_request(indent, (depth as i32) * 18, -1);
                        gtk_box_append(row, indent);
                    }
                    if is_dir {
                        let is_exp = mexp_p.borrow().contains(&path);
                        let chev = new_button(if is_exp { "▾" } else { "▸" }, &["akar-tree-chevron"]);
                        let p_ex = path.clone();
                        let mexp_c = Rc::clone(&mexp_p);
                        let pop_ex = Rc::clone(&populate_clone);
                        connect_signal(chev, "clicked", move || {
                            hide_all_menus();
                            let mut set = mexp_c.borrow_mut();
                            if !set.remove(&p_ex) {
                                set.insert(p_ex.clone());
                            }
                            drop(set);
                            (pop_ex.borrow())();
                        });
                        gtk_box_append(row, chev);
                    } else {
                        let sp = new_box(GTK_ORIENTATION_HORIZONTAL, 0, &[]);
                        gtk_widget_set_size_request(sp, 20, -1);
                        gtk_box_append(row, sp);
                    }
                }

                let lower = name.to_lowercase();
                let (badge_txt, badge_cls, kind_str) = if is_dir {
                    ("📁", "badge-folder", "Folder")
                } else if lower.ends_with(".docx") || lower.ends_with(".doc") {
                    ("W", "badge-docx", "Word Document")
                } else if lower.ends_with(".xlsx") || lower.ends_with(".csv") {
                    ("X", "badge-xlsx", "Excel Spreadsheet")
                } else if lower.ends_with(".pdf") {
                    ("PDF", "badge-pdf", "PDF Document")
                } else if lower.ends_with(".png") || lower.ends_with(".jpg") || lower.ends_with(".jpeg") || lower.ends_with(".svg") {
                    ("IMG", "badge-img", "Image")
                } else if lower.ends_with(".txt") || lower.ends_with(".md") {
                    ("TXT", "badge-docx", "Text Document")
                } else if lower.ends_with(".deb") {
                    ("DEB", "badge-xlsx", "Debian Package")
                } else {
                    ("📄", "badge-file", "Document")
                };

                // Select toggle button (clicking badge selects folder/file for Edit menu)
                let sel_btn = gtk_button_new();
                add_css(sel_btn, "akar-tab-btn");
                let badge = new_label(badge_txt, &["file-badge", badge_cls]);
                gtk_widget_set_valign(badge, GTK_ALIGN_CENTER);
                gtk_button_set_child(sel_btn, badge);
                let tip_sel = cstr("Click to select item (or right-click row for menu)");
                gtk_widget_set_tooltip_text(sel_btn, tip_sel.as_ptr());

                let open_btn = new_left_button("", &name, &["finder-name-btn"]);
                gtk_widget_set_hexpand(open_btn, 1);

                let size_str = if is_dir {
                    "--".to_string()
                } else if size > 1024 * 1024 {
                    format!("{:.1} MB", size as f64 / (1024.0 * 1024.0))
                } else if size > 1024 {
                    format!("{:.0} KB", (size as f64 / 1024.0).max(1.0))
                } else {
                    format!("{} B", size)
                };
                let size_lbl = new_label(&size_str, &["finder-meta-lbl"]);
                gtk_label_set_xalign(size_lbl, 1.0);
                gtk_widget_set_size_request(size_lbl, 84, -1);

                let kind_lbl = new_label(kind_str, &["finder-meta-lbl"]);
                gtk_label_set_xalign(kind_lbl, 0.0);
                gtk_widget_set_size_request(kind_lbl, 130, -1);

                let del_btn = new_button("🗑️", &["row-trash-btn"]);
                let dt = cstr("Delete item");
                gtk_widget_set_tooltip_text(del_btn, dt.as_ptr());

                gtk_box_append(row, sel_btn);
                gtk_box_append(row, open_btn);
                gtk_box_append(row, size_lbl);
                gtk_box_append(row, kind_lbl);
                gtk_box_append(row, del_btn);
                gtk_box_append(list_box, row);

                // Left-click badge -> select/deselect item
                {
                    let sel_s = Rc::clone(&sel_p);
                    let pop_s = Rc::clone(&populate_clone);
                    let p_sel = path.clone();
                    connect_signal(sel_btn, "clicked", move || {
                        hide_all_menus();
                        let same = sel_s.borrow().as_ref() == Some(&p_sel);
                        *sel_s.borrow_mut() = if same { None } else { Some(p_sel.clone()) };
                        (pop_s.borrow())();
                    });
                }

                // Right-click row -> select item & show context menu
                {
                    let ctx_c = Rc::clone(&ctx_p);
                    let p_rc = path.clone();
                    connect_right_click(row, move |x, y| {
                        ctx_c(row, x, y, Some((p_rc.clone(), is_dir)));
                    });
                }

                let nav_o = Rc::clone(&nav_p);
                let sel_o = Rc::clone(&sel_p);
                let p_open = path.clone();
                connect_signal(open_btn, "clicked", move || {
                    hide_all_menus();
                    if is_dir {
                        nav_o(p_open.clone());
                    } else {
                        *sel_o.borrow_mut() = Some(p_open.clone());
                        open_item_in_app(app, &p_open);
                    }
                });

                let pop_d = Rc::clone(&populate_clone);
                let p_del = path.clone();
                connect_signal(del_btn, "clicked", move || {
                    hide_all_menus();
                    if is_dir {
                        let _ = fs::remove_dir_all(&p_del);
                    } else {
                        let _ = fs::remove_file(&p_del);
                    }
                    (pop_d.borrow())();
                });
            }
        });

        // Right-click on empty area of file list -> empty-space context menu
        {
            let ctx_e = Rc::clone(&show_ctx_menu);
            connect_right_click_bubble(scroller, move |x, y| {
                ctx_e(scroller, x, y, None);
            });
        }

        // Populate File & Edit Menu Dropdowns
        let mi_f_new_folder = new_menu_item("📁", "New Folder", "Ctrl+Shift+N");
        let mi_f_new_tab = new_menu_item("➕", "New Tab", "Ctrl+T");
        let mi_f_pancer = new_menu_item("💻", "Open in Pancer", "");
        gtk_box_append(menu_file, mi_f_new_folder);
        gtk_box_append(menu_file, mi_f_new_tab);
        gtk_box_append(menu_file, gtk_separator_new(GTK_ORIENTATION_HORIZONTAL));
        gtk_box_append(menu_file, mi_f_pancer);

        let mi_e_copy = new_menu_item("📋", "Copy", "Ctrl+C");
        let mi_e_move = new_menu_item("✂️", "Move (Cut)", "Ctrl+X");
        let mi_e_paste = new_menu_item("📥", "Paste", "Ctrl+V");
        let mi_e_rename = new_menu_item("✏️", "Rename", "F2");
        let mi_e_delete = new_menu_item("🗑️", "Delete", "Del");
        gtk_box_append(menu_edit, mi_e_copy);
        gtk_box_append(menu_edit, mi_e_move);
        gtk_box_append(menu_edit, mi_e_paste);
        gtk_box_append(menu_edit, mi_e_rename);
        gtk_box_append(menu_edit, gtk_separator_new(GTK_ORIENTATION_HORIZONTAL));
        gtk_box_append(menu_edit, mi_e_delete);

        let mi_about_akar = new_menu_item("🌱", "About Akar", "");
        gtk_box_append(menu_help, mi_about_akar);
        connect_signal(mi_about_akar, "clicked", move || {
            hide_all_menus();
            open_about_akar(app, win);
        });

        let wire = |btn: *mut GtkWidget, f: &Rc<dyn Fn()>| {
            let fc = Rc::clone(f);
            connect_signal(btn, "clicked", move || fc());
        };
        wire(mi_f_new_folder, &act_new_folder);
        wire(mi_f_new_tab, &act_new_tab);
        wire(btn_add_tab, &act_new_tab);
        wire(mi_e_copy, &act_copy);
        wire(mi_e_move, &act_move);
        wire(mi_e_paste, &act_paste);
        wire(mi_e_rename, &act_rename);
        wire(mi_e_delete, &act_delete);
        {
            let cd_p = Rc::clone(&cur_dir);
            connect_signal(mi_f_pancer, "clicked", move || {
                hide_all_menus();
                crate::native_de::pancer::open_pancer_with_dir(app, Some(cd_p.borrow().clone()));
            });
        }

        // Global Hotkeys for Akar (Esc, Ctrl+T, Ctrl+Shift+N, Ctrl+C/X/V, F2)
        {
            let f_nf = Rc::clone(&act_new_folder);
            let f_nt = Rc::clone(&act_new_tab);
            let f_cp = Rc::clone(&act_copy);
            let f_mv = Rc::clone(&act_move);
            let f_ps = Rc::clone(&act_paste);
            let f_rn = Rc::clone(&act_rename);
            let f_dl = Rc::clone(&act_delete);
            connect_key_pressed(win, move |keyval, mods| {
                if keyval == 0xff1b && any_menu_open() {
                    hide_all_menus();
                    return true;
                }
                // Don't hijack Ctrl+C/V/X while typing in path_entry
                if gtk_widget_has_focus(path_entry) != 0 {
                    return false;
                }
                if keyval == 0xffbf {
                    // F2 -> Rename
                    f_rn();
                    return true;
                }
                if keyval == 0xffff {
                    // Delete -> Delete selected item
                    f_dl();
                    return true;
                }
                let ctrl = (mods & 4) != 0;
                let shift = (mods & 1) != 0;
                if !ctrl {
                    return false;
                }
                let ch = char::from_u32(keyval).map(|c| c.to_ascii_uppercase());
                match (shift, ch) {
                    (true, Some('N')) => { f_nf(); true }
                    (false, Some('T')) => { f_nt(); true }
                    (false, Some('C')) => { f_cp(); true }
                    (false, Some('X')) => { f_mv(); true }
                    (false, Some('V')) => { f_ps(); true }
                    _ => false,
                }
            });
        }

        // Sidebar shortcuts (left-aligned with Hijau Tunas active highlight!)
        let shortcuts = [
            ("🏠", "All Files (Home)", home_dir(), true),
            ("📄", "Documents", home_dir().join("Documents"), false),
            ("⬇️", "Downloads", home_dir().join("Downloads"), false),
            ("🖥️", "Desktop", home_dir().join("Desktop"), false),
        ];
        let sys_shortcuts = [
            ("🗄️", "BijiOS Storage", PathBuf::from("/var/lib/bijios-desktop")),
            ("💻", "System Root (/)", PathBuf::from("/")),
        ];

        let all_sb_btns: Rc<RefCell<Vec<*mut GtkWidget>>> = Rc::new(RefCell::new(Vec::new()));

        for (icon, label, target, active) in shortcuts {
            let init_cls = if active { "sidebar-btn-active" } else { "sidebar-btn" };
            let btn = new_left_button(icon, label, &[init_cls]);
            gtk_box_append(sidebar, btn);
            all_sb_btns.borrow_mut().push(btn);

            let cur_dir_s = Rc::clone(&cur_dir);
            let pop_s = Rc::clone(&populate);
            let btns_ref = Rc::clone(&all_sb_btns);
            connect_signal(btn, "clicked", move || {
                for &b in btns_ref.borrow().iter() {
                    remove_css(b, "sidebar-btn-active");
                    add_css(b, "sidebar-btn");
                }
                remove_css(btn, "sidebar-btn");
                add_css(btn, "sidebar-btn-active");
                let _ = fs::create_dir_all(&target);
                if target.exists() {
                    *cur_dir_s.borrow_mut() = target.clone();
                    (pop_s.borrow())();
                }
            });
        }

        let sb_hdr2 = new_label("SYSTEM", &["sidebar-section-hdr"]);
        gtk_label_set_xalign(sb_hdr2, 0.0);
        gtk_box_append(sidebar, sb_hdr2);

        for (icon, label, target) in sys_shortcuts {
            let btn = new_left_button(icon, label, &["sidebar-btn"]);
            gtk_box_append(sidebar, btn);
            all_sb_btns.borrow_mut().push(btn);

            let nav_s = Rc::clone(&navigate_to);
            let btns_ref = Rc::clone(&all_sb_btns);
            connect_signal(btn, "clicked", move || {
                for &b in btns_ref.borrow().iter() {
                    remove_css(b, "sidebar-btn-active");
                    add_css(b, "sidebar-btn");
                }
                remove_css(btn, "sidebar-btn");
                add_css(btn, "sidebar-btn-active");
                if target.exists() {
                    nav_s(target.clone());
                }
            });
        }

        // Remote Mounts (SSHFS from Pancer)
        let sb_hdr3 = new_label("REMOTE MOUNTS (SSHFS)", &["sidebar-section-hdr"]);
        gtk_label_set_xalign(sb_hdr3, 0.0);
        gtk_box_append(sidebar, sb_hdr3);

        let mounts_sb_box = new_box(GTK_ORIENTATION_VERTICAL, 3, &[]);
        gtk_box_append(sidebar, mounts_sb_box);

        let populate_mounts_sb: Rc<RefCell<Box<dyn Fn()>>> = Rc::new(RefCell::new(Box::new(|| {})));
        let pop_m_c = Rc::clone(&populate_mounts_sb);
        {
            let cur_dir_m = Rc::clone(&cur_dir);
            let pop_m = Rc::clone(&populate);
            *populate_mounts_sb.borrow_mut() = Box::new(move || {
                clear_box_children(mounts_sb_box);
                let mounts = read_sshfs_mounts();
                if mounts.is_empty() {
                    let hint = new_label("No SSH mounts yet", &["finder-meta-lbl"]);
                    gtk_label_set_xalign(hint, 0.0);
                    set_margins(hint, 2, 2, 8, 4);
                    gtk_box_append(mounts_sb_box, hint);
                    return;
                }
                for (name, mpath) in mounts {
                    let row = new_box(GTK_ORIENTATION_HORIZONTAL, 2, &[]);
                    let btn = new_left_button("🔐", &name, &["sidebar-btn"]);
                    gtk_widget_set_hexpand(btn, 1);
                    let btn_um = new_button("⏏", &["row-trash-btn"]);
                    let tip_u = cstr(&format!("Unmount {}", name));
                    gtk_widget_set_tooltip_text(btn_um, tip_u.as_ptr());

                    gtk_box_append(row, btn);
                    gtk_box_append(row, btn_um);
                    gtk_box_append(mounts_sb_box, row);

                    let mp_open = mpath.clone();
                    let cd_o = Rc::clone(&cur_dir_m);
                    let pp_o = Rc::clone(&pop_m);
                    connect_signal(btn, "clicked", move || {
                        if mp_open.exists() {
                            *cd_o.borrow_mut() = mp_open.clone();
                            (pp_o.borrow())();
                        }
                    });

                    let mp_u = mpath.clone();
                    let cd_u = Rc::clone(&cur_dir_m);
                    let pp_u = Rc::clone(&pop_m);
                    let pm_u = Rc::clone(&pop_m_c);
                    connect_signal(btn_um, "clicked", move || {
                        let mp_str = mp_u.to_string_lossy().to_string();
                        let _ = Command::new("sh")
                            .args(["-c", &format!("fusermount3 -u '{0}' 2>/dev/null || fusermount -u '{0}' 2>/dev/null || umount '{0}'", mp_str.replace('\'', "'\\''"))])
                            .status();
                        if cd_u.borrow().starts_with(&mp_u) {
                            *cd_u.borrow_mut() = home_dir();
                            (pp_u.borrow())();
                        }
                        (pm_u.borrow())();
                    });
                }
            });
        }
        (populate_mounts_sb.borrow())();

        // Sidebar Expandable Folder Tree (🌳 FOLDER TREE)
        let sb_hdr4 = new_label("FOLDER TREE", &["sidebar-section-hdr"]);
        gtk_label_set_xalign(sb_hdr4, 0.0);
        gtk_box_append(sidebar, sb_hdr4);

        let tree_scroll = gtk_scrolled_window_new();
        gtk_scrolled_window_set_policy(tree_scroll, GTK_POLICY_AUTOMATIC, GTK_POLICY_AUTOMATIC);
        gtk_widget_set_vexpand(tree_scroll, 1);
        let tree_sb_box = new_box(GTK_ORIENTATION_VERTICAL, 1, &[]);
        gtk_scrolled_window_set_child(tree_scroll, tree_sb_box);
        gtk_box_append(sidebar, tree_scroll);

        {
            let cd_t = Rc::clone(&cur_dir);
            let exp_t = Rc::clone(&sb_expanded);
            let nav_t = Rc::clone(&navigate_to);
            let pt_self = Rc::clone(&populate_tree_sb);

            fn collect_sb_dirs(
                dir: &std::path::Path,
                depth: usize,
                exp: &HashSet<PathBuf>,
                out: &mut Vec<(String, PathBuf, usize, bool)>,
            ) {
                if depth > 4 || out.len() >= 140 {
                    return;
                }
                let mut subs: Vec<(String, PathBuf)> = Vec::new();
                if let Ok(rd) = fs::read_dir(dir) {
                    for ent in rd.flatten() {
                        let p = ent.path();
                        let name = ent.file_name().to_string_lossy().to_string();
                        if name.starts_with('.') {
                            continue;
                        }
                        if ent.metadata().map(|m| m.is_dir()).unwrap_or(false) {
                            subs.push((name, p));
                        }
                    }
                }
                subs.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
                for (name, p) in subs.into_iter().take(40) {
                    if out.len() >= 140 {
                        break;
                    }
                    let is_e = exp.contains(&p);
                    out.push((name, p.clone(), depth, is_e));
                    if is_e {
                        collect_sb_dirs(&p, depth + 1, exp, out);
                    }
                }
            }

            *populate_tree_sb.borrow_mut() = Box::new(move || {
                clear_box_children(tree_sb_box);
                let active_dir = cd_t.borrow().clone();
                let exp_snap = exp_t.borrow().clone();

                let mut nodes: Vec<(String, PathBuf, usize, bool)> = Vec::new();
                let home = home_dir();
                let home_exp = exp_snap.contains(&home);
                nodes.push(("🏠 Home".to_string(), home.clone(), 0, home_exp));
                if home_exp {
                    collect_sb_dirs(&home, 1, &exp_snap, &mut nodes);
                }
                let sys_root = PathBuf::from("/");
                let root_exp = exp_snap.contains(&sys_root);
                nodes.push(("💻 System Root (/)".to_string(), sys_root.clone(), 0, root_exp));
                if root_exp {
                    collect_sb_dirs(&sys_root, 1, &exp_snap, &mut nodes);
                }

                for (label, path, depth, is_exp) in nodes {
                    let row = new_box(GTK_ORIENTATION_HORIZONTAL, 2, &[]);
                    if depth > 0 {
                        let ind = new_box(GTK_ORIENTATION_HORIZONTAL, 0, &[]);
                        gtk_widget_set_size_request(ind, (depth as i32) * 12, -1);
                        gtk_box_append(row, ind);
                    }
                    let chev = new_button(if is_exp { "▾" } else { "▸" }, &["akar-tree-chevron"]);
                    let is_act = active_dir == path;
                    let txt = if depth == 0 { label } else { format!("📁 {}", label) };
                    let nbtn = new_left_button(
                        "",
                        &txt,
                        &[if is_act { "akar-tree-node-active" } else { "akar-tree-node" }],
                    );
                    gtk_widget_set_hexpand(nbtn, 1);

                    gtk_box_append(row, chev);
                    gtk_box_append(row, nbtn);
                    gtk_box_append(tree_sb_box, row);

                    {
                        let p_ex = path.clone();
                        let exp_c = Rc::clone(&exp_t);
                        let pt_c = Rc::clone(&pt_self);
                        connect_signal(chev, "clicked", move || {
                            let mut set = exp_c.borrow_mut();
                            if !set.remove(&p_ex) {
                                set.insert(p_ex.clone());
                            }
                            drop(set);
                            (pt_c.borrow())();
                        });
                    }
                    {
                        let p_nav = path.clone();
                        let nav_c = Rc::clone(&nav_t);
                        connect_signal(nbtn, "clicked", move || {
                            nav_c(p_nav.clone());
                        });
                    }
                }
            });
        }
        (populate_tree_sb.borrow())();

        // Toolbar handlers
        {
            let cur_dir_u = Rc::clone(&cur_dir);
            let nav_u = Rc::clone(&navigate_to);
            connect_signal(btn_up, "clicked", move || {
                let parent = cur_dir_u.borrow().parent().map(|p| p.to_path_buf());
                if let Some(p) = parent {
                    nav_u(p);
                }
            });
        }
        {
            let nav_e = Rc::clone(&navigate_to);
            connect_signal(path_entry, "activate", move || {
                let txt = get_entry_text(path_entry);
                let p = PathBuf::from(txt.trim());
                if p.is_dir() {
                    nav_e(p);
                }
            });
        }
        {
            let f_nf = Rc::clone(&act_new_folder);
            connect_signal(btn_new_folder, "clicked", move || {
                f_nf();
            });
        }
        connect_signal(btn_new_doc, "clicked", move || {
            hide_all_menus();
            open_wordcraft(app, None);
        });
        {
            let pop_r = Rc::clone(&populate);
            connect_signal(btn_refresh, "clicked", move || {
                hide_all_menus();
                (pop_r.borrow())();
            });
        }

        gtk_box_append(root, sidebar);
        gtk_box_append(root, main_col);
        gtk_window_set_child(win, overlay);
        (rebuild_tabs.borrow())();
        (populate.borrow())();
        gtk_window_present(win);
    }
}

// ============================================================================
// 2. Semai - App Launcher (Native BijiOS Apps + Installed .deb Applications)
// ============================================================================
pub fn open_launchpad(
    app: *mut GtkApplication,
    handles: Rc<crate::native_de::DesktopHandles>,
    on_lock: Rc<dyn Fn()>,
) {
    unsafe {
        let win = create_app_window(app, "🚀  Semai - App Launcher", 900, 620);
        let root = new_box(GTK_ORIENTATION_VERTICAL, 14, &[]);
        set_margin_all(root, 20);

        let header = new_label("Semai — BijiOS & Installed Linux (.deb) Applications", &["app-heading"]);
        gtk_label_set_xalign(header, 0.0);
        gtk_box_append(root, header);

        let search = gtk_entry_new();
        let ph = cstr("🔍 Search BijiOS apps or installed Linux (.deb) applications...");
        gtk_entry_set_placeholder_text(search, ph.as_ptr());
        gtk_box_append(root, search);

        let scroller = gtk_scrolled_window_new();
        gtk_widget_set_hexpand(scroller, 1);
        gtk_widget_set_vexpand(scroller, 1);

        let flow = gtk_flow_box_new();
        gtk_flow_box_set_max_children_per_line(flow, 5);
        gtk_flow_box_set_min_children_per_line(flow, 4);
        gtk_flow_box_set_row_spacing(flow, 14);
        gtk_flow_box_set_column_spacing(flow, 14);
        gtk_flow_box_set_selection_mode(flow, 0);
        gtk_scrolled_window_set_child(scroller, flow);
        gtk_box_append(root, scroller);

        let deb_apps = Rc::new(scan_installed_deb_apps());

        let render_grid: Rc<dyn Fn(&str)> = {
            let deb_apps = Rc::clone(&deb_apps);
            let on_lock = Rc::clone(&on_lock);
            let handles_g = Rc::clone(&handles);
            Rc::new(move |query: &str| {
                clear_flow_children(flow);
                let q = query.trim().to_lowercase();

                let builtins: Vec<(&str, &str, &str, &str)> = vec![
                    ("finder", "📁", "icon-finder", "Akar - File Manager"),
                    ("bibit", "🛍️", "icon-bibit", "Bibit - App Store"),
                    ("pancer", "💻", "icon-pancer", "Pancer - Terminal"),
                    ("wordcraft", "📝", "icon-wordcraft", "WordCraft"),
                    ("gridcraft", "📊", "icon-gridcraft", "GridCraft"),
                    ("deckcraft", "📽️", "icon-deckcraft", "DeckCraft"),
                    ("pdfcraft", "📕", "icon-pdfcraft", "PDFCraft"),
                    ("notes", "🗒️", "icon-notes", "BijiOS Notes"),
                    ("calculator", "🧮", "icon-calculator", "Calculator"),
                    ("activity", "📈", "icon-activity", "Activity Monitor"),
                    ("settings", "⚙️", "icon-settings", "System Settings"),
                    ("about", "🌱", "icon-about", "About BijiOS"),
                ];

                for (id, icon, icon_cls, name) in builtins {
                    if !q.is_empty() && !name.to_lowercase().contains(&q) && !id.contains(&q) {
                        continue;
                    }
                    let btn = if id == "about" {
                        let b = gtk_button_new();
                        add_css(b, "launchpad-tile");
                        let col = new_box(GTK_ORIENTATION_VERTICAL, 8, &[]);
                        gtk_widget_set_halign(col, GTK_ALIGN_CENTER);
                        gtk_widget_set_valign(col, GTK_ALIGN_CENTER);
                        gtk_box_append(col, new_logo_image(52, true));
                        gtk_box_append(col, new_label(name, &["launchpad-caption"]));
                        gtk_button_set_child(b, col);
                        b
                    } else {
                        new_icon_tile_button(icon, icon_cls, name, &["launchpad-tile"])
                    };
                    gtk_flow_box_append(flow, btn);
                    let id_s = id.to_string();
                    let on_lock_c = Rc::clone(&on_lock);
                    let handles_c = Rc::clone(&handles_g);
                    connect_signal(btn, "clicked", move || {
                        gtk_window_close(win);
                        match id_s.as_str() {
                            "finder" => open_finder(app),
                            "bibit" => open_bibit(app),
                            "pancer" => open_pancer(app),
                            "wordcraft" => open_wordcraft(app, None),
                            "gridcraft" => open_gridcraft(app),
                            "deckcraft" => open_deckcraft(None),
                            "pdfcraft" => open_pdfcraft(None),
                            "notes" => open_notes(app),
                            "calculator" => open_calculator(app),
                            "activity" => open_activity_monitor(app),
                            "settings" => open_settings(app, Rc::clone(&handles_c), Rc::clone(&on_lock_c)),
                            "about" => open_about(app),
                            _ => {}
                        }
                    });
                }

                for da in deb_apps.iter() {
                    if !q.is_empty()
                        && !da.name.to_lowercase().contains(&q)
                        && !da.id.to_lowercase().contains(&q)
                        && !da.comment.to_lowercase().contains(&q)
                    {
                        continue;
                    }
                    let short_name = if da.name.len() > 17 {
                        format!("{}…", &da.name[..16])
                    } else {
                        da.name.clone()
                    };
                    let icon_cls = match da.icon.as_str() {
                        "🌐" => "icon-finder",
                        "💻" => "icon-about",
                        "📄" => "icon-wordcraft",
                        "🛠️" => "icon-settings",
                        "🎵" => "icon-notes",
                        _ => "icon-gridcraft",
                    };
                    let btn = new_deb_icon_tile_button(
                        &da.icon_name,
                        &da.icon,
                        icon_cls,
                        &short_name,
                        &["launchpad-tile"],
                    );
                    let tip = cstr(&format!("{} ({})", da.name, da.comment));
                    gtk_widget_set_tooltip_text(btn, tip.as_ptr());
                    gtk_flow_box_append(flow, btn);
                    let exec = da.exec.clone();
                    connect_signal(btn, "clicked", move || {
                        gtk_window_close(win);
                        launch_deb_app(&exec);
                    });
                }
            })
        };

        render_grid("");
        {
            let rg = Rc::clone(&render_grid);
            connect_signal(search, "changed", move || {
                let q = get_entry_text(search);
                rg(&q);
            });
        }

        gtk_window_set_child(win, root);
        gtk_window_present(win);
        gtk_widget_grab_focus(search);
    }
}

fn spawn_bundled_storytold(bin_path: &str, file: Option<PathBuf>) -> bool {
    if !std::path::Path::new(bin_path).exists() {
        return false;
    }
    let mut cmd = Command::new(bin_path);
    if let Some(p) = file {
        cmd.arg(p);
    }
    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .is_ok()
}

pub fn open_deckcraft(initial_file: Option<PathBuf>) {
    let _ = spawn_bundled_storytold("/usr/bin/deckcraft", initial_file);
}

pub fn open_pdfcraft(initial_file: Option<PathBuf>) {
    let _ = spawn_bundled_storytold("/usr/bin/pdfcraft", initial_file);
}

// ============================================================================
// 3. WordCraft (Bundled Storytold Pure Rust Word Processor, Apache-2.0)
// ============================================================================
pub fn open_wordcraft(app: *mut GtkApplication, initial_file: Option<PathBuf>) {
    if spawn_bundled_storytold("/usr/bin/wordcraft", initial_file.clone()) {
        return;
    }
    unsafe {
        let win = create_app_window(app, "📝 BijiOS WordCraft", 900, 620);
        let root = new_box(GTK_ORIENTATION_VERTICAL, 0, &[]);

        let toolbar = new_box(GTK_ORIENTATION_HORIZONTAL, 8, &["app-toolbar"]);
        let name_entry = gtk_entry_new();
        gtk_widget_set_hexpand(name_entry, 1);
        let default_path = initial_file
            .clone()
            .unwrap_or_else(|| home_dir().join("Documents").join("BijiOS_Document.docx"));
        set_entry_text(name_entry, &default_path.to_string_lossy());

        let btn_new = new_button("✨ New", &["toolbar-btn"]);
        let btn_save = new_button("💾 Save", &["primary-btn"]);
        gtk_box_append(toolbar, new_label("File:", &["app-muted"]));
        gtk_box_append(toolbar, name_entry);
        gtk_box_append(toolbar, btn_new);
        gtk_box_append(toolbar, btn_save);
        gtk_box_append(root, toolbar);

        let scroller = gtk_scrolled_window_new();
        gtk_widget_set_hexpand(scroller, 1);
        gtk_widget_set_vexpand(scroller, 1);
        let tv = gtk_text_view_new();
        gtk_text_view_set_wrap_mode(tv, GTK_WRAP_WORD_CHAR);
        add_css(tv, "editor-view");
        gtk_scrolled_window_set_child(scroller, tv);
        gtk_box_append(root, scroller);

        let status = new_label("BijiOS WordCraft Native Editor — Ready", &["app-muted"]);
        gtk_label_set_xalign(status, 0.0);
        set_margins(status, 6, 8, 14, 14);
        gtk_box_append(root, status);

        if let Some(p) = initial_file {
            if let Ok(content) = fs::read_to_string(&p) {
                set_text_view_text(tv, &content);
                set_label_text(status, &format!("Opened: {}", p.display()));
            }
        } else {
            set_text_view_text(
                tv,
                "Welcome to BijiOS WordCraft (Pure Rust + Native GTK4)\n\nStart writing your document here and click 'Save' to export as .docx or .txt.",
            );
        }

        connect_signal(btn_new, "clicked", move || {
            set_text_view_text(tv, "");
            set_label_text(status, "New document created");
        });

        connect_signal(btn_save, "clicked", move || {
            let path_str = get_entry_text(name_entry);
            let p = PathBuf::from(path_str.trim());
            if let Some(parent) = p.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let text = get_text_view_text(tv);
            let res = if path_str.to_lowercase().ends_with(".docx") {
                let html = format!("<p>{}</p>", text.replace('\n', "</p><p>"));
                let bytes = DocxEngine::create_docx_package(&html);
                fs::write(&p, bytes)
            } else {
                fs::write(&p, text.as_bytes())
            };
            match res {
                Ok(_) => set_label_text(status, &format!("✅ Saved to {}", p.display())),
                Err(e) => set_label_text(status, &format!("❌ Save failed: {}", e)),
            }
        });

        gtk_window_set_child(win, root);
        gtk_window_present(win);
    }
}

// ============================================================================
// 4. GridCraft (Bundled Storytold Pure Rust Spreadsheet, Apache-2.0)
// ============================================================================
pub fn open_gridcraft(app: *mut GtkApplication) {
    open_gridcraft_with_file(app, None);
}

pub fn open_gridcraft_with_file(app: *mut GtkApplication, initial_file: Option<PathBuf>) {
    if spawn_bundled_storytold("/usr/bin/gridcraft", initial_file) {
        return;
    }
    unsafe {
        let win = create_app_window(app, "📊 BijiOS GridCraft", 920, 560);
        let root = new_box(GTK_ORIENTATION_VERTICAL, 0, &[]);

        let toolbar = new_box(GTK_ORIENTATION_HORIZONTAL, 8, &["app-toolbar"]);
        let path_entry = gtk_entry_new();
        gtk_widget_set_hexpand(path_entry, 1);
        set_entry_text(
            path_entry,
            &home_dir().join("Documents").join("BijiOS_Sheet.xlsx").to_string_lossy(),
        );
        let btn_sum = new_button("∑ Calculate Total", &["toolbar-btn"]);
        let btn_save = new_button("💾 Export .xlsx", &["primary-btn"]);
        gtk_box_append(toolbar, path_entry);
        gtk_box_append(toolbar, btn_sum);
        gtk_box_append(toolbar, btn_save);
        gtk_box_append(root, toolbar);

        let scroller = gtk_scrolled_window_new();
        gtk_widget_set_hexpand(scroller, 1);
        gtk_widget_set_vexpand(scroller, 1);

        let grid = gtk_grid_new();
        gtk_grid_set_row_spacing(grid, 4);
        gtk_grid_set_column_spacing(grid, 4);
        set_margin_all(grid, 14);

        let cols = ["A", "B", "C", "D", "E"];
        for (c, col_name) in cols.iter().enumerate() {
            let hdr = new_label(col_name, &["widget-title"]);
            gtk_grid_attach(grid, hdr, (c + 1) as i32, 0, 1, 1);
        }

        let cells: Rc<RefCell<Vec<Vec<*mut GtkWidget>>>> = Rc::new(RefCell::new(Vec::new()));
        let sample = [
            ["BijiOS Project Item", "Qty", "Unit Price", "Total", "Status"],
            ["BijiOS 1.0.0 Padi Core", "1", "1500", "1500", "Active"],
            ["TOON v4 Storage Engine", "2", "500", "1000", "Active"],
            ["GTK4 Liquid Glass Shell", "1", "850", "850", "Active"],
        ];

        for r in 0..8 {
            let r_lbl = new_label(&format!("{}", r + 1), &["widget-sub"]);
            gtk_grid_attach(grid, r_lbl, 0, (r + 1) as i32, 1, 1);
            let mut row_entries = Vec::new();
            for c in 0..5 {
                let e = gtk_entry_new();
                gtk_widget_set_hexpand(e, 1);
                if r < sample.len() {
                    set_entry_text(e, sample[r][c]);
                }
                gtk_grid_attach(grid, e, (c + 1) as i32, (r + 1) as i32, 1, 1);
                row_entries.push(e);
            }
            cells.borrow_mut().push(row_entries);
        }

        gtk_scrolled_window_set_child(scroller, grid);
        gtk_box_append(root, scroller);

        let status = new_label("BijiOS GridCraft Native Spreadsheet", &["app-muted"]);
        gtk_label_set_xalign(status, 0.0);
        set_margins(status, 6, 8, 14, 14);
        gtk_box_append(root, status);

        {
            let cells_s = Rc::clone(&cells);
            connect_signal(btn_sum, "clicked", move || {
                let rows = cells_s.borrow();
                let mut grand_total = 0.0f64;
                for r in 1..7 {
                    let qty: f64 = get_entry_text(rows[r][1]).trim().parse().unwrap_or(0.0);
                    let price: f64 = get_entry_text(rows[r][2]).trim().parse().unwrap_or(0.0);
                    if qty > 0.0 && price > 0.0 {
                        let tot = qty * price;
                        set_entry_text(rows[r][3], &format!("{:.0}", tot));
                        grand_total += tot;
                    } else if let Ok(v) = get_entry_text(rows[r][3]).trim().parse::<f64>() {
                        grand_total += v;
                    }
                }
                set_entry_text(rows[7][0], "TOTAL");
                set_entry_text(rows[7][3], &format!("{:.0}", grand_total));
                set_label_text(status, &format!("✅ Calculated total: {:.0}", grand_total));
            });
        }

        {
            let cells_e = Rc::clone(&cells);
            connect_signal(btn_save, "clicked", move || {
                let p = PathBuf::from(get_entry_text(path_entry).trim().to_string());
                if let Some(parent) = p.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                let rows = cells_e.borrow();
                let mut cell_entries = Vec::new();
                for (r, row) in rows.iter().enumerate() {
                    for (c, entry) in row.iter().enumerate() {
                        let val = get_entry_text(*entry);
                        if !val.trim().is_empty() {
                            let coord = format!("{}{}", cols[c], r + 1);
                            let escaped = val.replace('\\', "\\\\").replace('"', "\\\"");
                            cell_entries.push(format!("\"{}\":{{\"value\":\"{}\"}}", coord, escaped));
                        }
                    }
                }
                let json = format!(
                    r#"{{"title":"BijiOS Sheet","sheets":[{{"name":"Sheet1","cells":{{{}}}}}]}}"#,
                    cell_entries.join(",")
                );
                let bytes = XlsxEngine::create_xlsx_package(&json);
                match fs::write(&p, bytes) {
                    Ok(_) => set_label_text(status, &format!("✅ Spreadsheet exported to {}", p.display())),
                    Err(e) => set_label_text(status, &format!("❌ Export failed: {}", e)),
                }
            });
        }

        gtk_window_set_child(win, root);
        gtk_window_present(win);
    }
}

// ============================================================================
// 5. BijiOS Notes (Native Markdown Notes)
// ============================================================================
pub fn open_notes(app: *mut GtkApplication) {
    unsafe {
        let win = create_app_window(app, "🗒️ BijiOS Notes", 860, 560);
        let notes_dir = home_dir().join("Documents").join("BijiOS-Notes");
        let _ = fs::create_dir_all(&notes_dir);

        let root = new_box(GTK_ORIENTATION_HORIZONTAL, 0, &[]);
        let sidebar = new_box(GTK_ORIENTATION_VERTICAL, 6, &["app-sidebar"]);
        let btn_new = new_button("➕ New Note", &["primary-btn"]);
        gtk_box_append(sidebar, btn_new);

        let list_col = new_box(GTK_ORIENTATION_VERTICAL, 4, &[]);
        gtk_box_append(sidebar, list_col);

        let editor_col = new_box(GTK_ORIENTATION_VERTICAL, 0, &[]);
        gtk_widget_set_hexpand(editor_col, 1);
        gtk_widget_set_vexpand(editor_col, 1);

        let toolbar = new_box(GTK_ORIENTATION_HORIZONTAL, 8, &["app-toolbar"]);
        let title_entry = gtk_entry_new();
        gtk_widget_set_hexpand(title_entry, 1);
        set_entry_text(title_entry, "BijiOS_Note.md");
        let btn_save = new_button("💾 Save Note", &["primary-btn"]);
        gtk_box_append(toolbar, title_entry);
        gtk_box_append(toolbar, btn_save);
        gtk_box_append(editor_col, toolbar);

        let scroller = gtk_scrolled_window_new();
        gtk_widget_set_hexpand(scroller, 1);
        gtk_widget_set_vexpand(scroller, 1);
        let tv = gtk_text_view_new();
        gtk_text_view_set_wrap_mode(tv, GTK_WRAP_WORD_CHAR);
        add_css(tv, "editor-view");
        gtk_scrolled_window_set_child(scroller, tv);
        gtk_box_append(editor_col, scroller);

        let status = new_label("BijiOS Notes — Locally synced", &["app-muted"]);
        gtk_label_set_xalign(status, 0.0);
        set_margins(status, 6, 8, 12, 12);
        gtk_box_append(editor_col, status);

        let refresh_notes: Rc<RefCell<Box<dyn Fn()>>> = Rc::new(RefCell::new(Box::new(|| {})));
        let notes_dir_r = notes_dir.clone();
        *refresh_notes.borrow_mut() = Box::new(move || {
            clear_box_children(list_col);
            if let Ok(rd) = fs::read_dir(&notes_dir_r) {
                let mut files: Vec<PathBuf> = rd.flatten().map(|e| e.path()).collect();
                files.sort();
                for f in files {
                    if let Some(name) = f.file_name().and_then(|s| s.to_str()) {
                        let b = new_left_button("🗒️", name, &["sidebar-btn"]);
                        gtk_box_append(list_col, b);
                        let fp = f.clone();
                        let fname = name.to_string();
                        connect_signal(b, "clicked", move || {
                            set_entry_text(title_entry, &fname);
                            if let Ok(txt) = fs::read_to_string(&fp) {
                                set_text_view_text(tv, &txt);
                                set_label_text(status, &format!("Opened {}", fname));
                            }
                        });
                    }
                }
            }
        });

        connect_signal(btn_new, "clicked", move || {
            let ts = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
                % 10000;
            set_entry_text(title_entry, &format!("Note_{}.md", ts));
            set_text_view_text(tv, "# New BijiOS Note\n\n");
        });

        {
            let nd = notes_dir.clone();
            let rn = Rc::clone(&refresh_notes);
            connect_signal(btn_save, "clicked", move || {
                let mut fname = get_entry_text(title_entry).trim().to_string();
                if fname.is_empty() {
                    fname = "BijiOS_Note.md".to_string();
                }
                if !fname.ends_with(".md") && !fname.ends_with(".txt") {
                    fname.push_str(".md");
                }
                let content = get_text_view_text(tv);
                if fs::write(nd.join(&fname), content).is_ok() {
                    set_label_text(status, &format!("✅ Saved: {}", fname));
                    (rn.borrow())();
                }
            });
        }

        (refresh_notes.borrow())();
        gtk_box_append(root, sidebar);
        gtk_box_append(root, editor_col);
        gtk_window_set_child(win, root);
        gtk_window_present(win);
    }
}

// ============================================================================
// 6. BijiOS Calculator (Native GTK4 Calculator)
// ============================================================================
pub fn open_calculator(app: *mut GtkApplication) {
    unsafe {
        let win = create_app_window(app, "🧮 BijiOS Calculator", 310, 420);
        gtk_window_set_resizable(win, 0);
        let root = new_box(GTK_ORIENTATION_VERTICAL, 10, &[]);
        set_margin_all(root, 14);

        let display = new_label("0", &["calc-display"]);
        gtk_label_set_xalign(display, 1.0);
        gtk_box_append(root, display);

        #[derive(Default)]
        struct CalcState {
            current: String,
            acc: f64,
            op: Option<char>,
            reset_next: bool,
        }
        let state = Rc::new(RefCell::new(CalcState {
            current: "0".to_string(),
            ..Default::default()
        }));

        let grid = gtk_grid_new();
        gtk_grid_set_row_spacing(grid, 8);
        gtk_grid_set_column_spacing(grid, 8);
        gtk_grid_set_column_homogeneous(grid, 1);

        let keys = [
            ["C", "±", "%", "÷"],
            ["7", "8", "9", "×"],
            ["4", "5", "6", "-"],
            ["1", "2", "3", "+"],
            ["0", ".", "⌫", "="],
        ];

        for (r, row) in keys.iter().enumerate() {
            for (c, key) in row.iter().enumerate() {
                let is_op = matches!(*key, "÷" | "×" | "-" | "+" | "=");
                let css: &[&str] = if is_op { &["calc-btn", "calc-op"] } else { &["calc-btn"] };
                let btn = new_button(key, css);
                gtk_grid_attach(grid, btn, c as i32, r as i32, 1, 1);

                let st = Rc::clone(&state);
                let k = (*key).to_string();
                connect_signal(btn, "clicked", move || {
                    let mut s = st.borrow_mut();
                    match k.as_str() {
                        "C" => {
                            s.current = "0".to_string();
                            s.acc = 0.0;
                            s.op = None;
                            s.reset_next = false;
                        }
                        "⌫" => {
                            s.current.pop();
                            if s.current.is_empty() || s.current == "-" {
                                s.current = "0".to_string();
                            }
                        }
                        "±" => {
                            if let Ok(v) = s.current.parse::<f64>() {
                                s.current = format_calc(-v);
                            }
                        }
                        "%" => {
                            if let Ok(v) = s.current.parse::<f64>() {
                                s.current = format_calc(v / 100.0);
                            }
                        }
                        "+" | "-" | "×" | "÷" => {
                            s.acc = s.current.parse::<f64>().unwrap_or(0.0);
                            s.op = k.chars().next();
                            s.reset_next = true;
                        }
                        "=" => {
                            let rhs = s.current.parse::<f64>().unwrap_or(0.0);
                            let res = match s.op {
                                Some('+') => s.acc + rhs,
                                Some('-') => s.acc - rhs,
                                Some('×') => s.acc * rhs,
                                Some('÷') => if rhs != 0.0 { s.acc / rhs } else { 0.0 },
                                _ => rhs,
                            };
                            s.current = format_calc(res);
                            s.op = None;
                            s.reset_next = true;
                        }
                        "." => {
                            if s.reset_next {
                                s.current = "0.".to_string();
                                s.reset_next = false;
                            } else if !s.current.contains('.') {
                                s.current.push('.');
                            }
                        }
                        digit => {
                            if s.reset_next || s.current == "0" {
                                s.current = digit.to_string();
                                s.reset_next = false;
                            } else {
                                s.current.push_str(digit);
                            }
                        }
                    }
                    set_label_text(display, &s.current);
                });
            }
        }

        gtk_box_append(root, grid);
        gtk_window_set_child(win, root);
        gtk_window_present(win);
    }
}

fn format_calc(v: f64) -> String {
    if (v - v.round()).abs() < 1e-9 {
        format!("{:.0}", v)
    } else {
        format!("{:.6}", v).trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

// ============================================================================
// 7. BijiOS Activity Monitor (Real-Time System & Process Monitor)
// ============================================================================
pub fn read_mem_usage() -> (u64, u64, f64) {
    let mut total_kb = 0u64;
    let mut avail_kb = 0u64;
    if let Ok(txt) = fs::read_to_string("/proc/meminfo") {
        for line in txt.lines() {
            if line.starts_with("MemTotal:") {
                total_kb = line.split_whitespace().nth(1).and_then(|s| s.parse().ok()).unwrap_or(0);
            } else if line.starts_with("MemAvailable:") {
                avail_kb = line.split_whitespace().nth(1).and_then(|s| s.parse().ok()).unwrap_or(0);
            }
        }
    }
    let used_kb = total_kb.saturating_sub(avail_kb);
    let pct = if total_kb > 0 { used_kb as f64 / total_kb as f64 } else { 0.0 };
    (used_kb / 1024, total_kb / 1024, pct)
}

pub fn read_load_avg() -> String {
    fs::read_to_string("/proc/loadavg")
        .unwrap_or_else(|_| "0.00 0.00 0.00".to_string())
        .split_whitespace()
        .take(3)
        .collect::<Vec<_>>()
        .join("  ")
}

pub fn open_activity_monitor(app: *mut GtkApplication) {
    crate::native_de::activity::open_activity_monitor(app);
}

// ============================================================================
// 8. BijiOS System Settings (Delegates to macOS Sidebar Settings Module)
// ============================================================================
pub fn open_settings(
    app: *mut GtkApplication,
    handles: Rc<crate::native_de::DesktopHandles>,
    on_lock: Rc<dyn Fn()>,
) {
    crate::native_de::settings::open_system_settings(app, handles, on_lock);
}

// ============================================================================
// 9. About BijiOS (Detailed OS Info + BijiOS Logo + Host IDVE / KVM Logo)
// ============================================================================
pub fn read_os_pretty_name() -> String {
    if let Ok(txt) = fs::read_to_string("/etc/os-release") {
        for line in txt.lines() {
            if let Some(val) = line.strip_prefix("PRETTY_NAME=") {
                return val.trim().trim_matches('"').to_string();
            }
        }
    }
    "Debian GNU/Linux 13 (trixie)".to_string()
}

pub fn read_virt_host_info() -> String {
    let virt = Command::new("systemd-detect-virt")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_uppercase())
        .filter(|s| !s.is_empty() && s != "NONE")
        .unwrap_or_else(|| "KVM".to_string());
    let vendor = fs::read_to_string("/sys/class/dmi/id/sys_vendor")
        .unwrap_or_default()
        .trim()
        .to_string();
    let product = fs::read_to_string("/sys/class/dmi/id/product_name")
        .unwrap_or_default()
        .trim()
        .to_string();
    let model = if !vendor.is_empty() && !product.is_empty() {
        if product.to_lowercase().starts_with(&vendor.to_lowercase()) {
            product
        } else {
            format!("{} {}", vendor, product)
        }
    } else if !product.is_empty() {
        product
    } else {
        vendor
    };
    if !model.is_empty() {
        format!("IDVE Cloud / {} ({})", virt, model)
    } else {
        format!("IDVE Cloud / {} Virtualization", virt)
    }
}

pub fn read_arch_and_kernel() -> (String, String) {
    let arch = Command::new("uname")
        .arg("-m")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "x86_64".to_string());
    let kernel = Command::new("uname")
        .arg("-sr")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "Linux".to_string());
    (arch, kernel)
}

pub fn read_cpu_summary() -> String {
    let mut model = String::new();
    let mut cores = 0u32;
    if let Ok(txt) = fs::read_to_string("/proc/cpuinfo") {
        for line in txt.lines() {
            if line.starts_with("processor") {
                cores += 1;
            } else if model.is_empty() && line.starts_with("model name") {
                if let Some((_, v)) = line.split_once(':') {
                    model = v.trim().to_string();
                }
            }
        }
    }
    if model.is_empty() {
        model = "x86_64 Virtual CPU".to_string();
    }
    if cores > 0 {
        format!("{} ({} vCPU)", model, cores)
    } else {
        model
    }
}

pub fn open_about(app: *mut GtkApplication) {
    unsafe {
        let win = create_app_window(app, "About BijiOS", 660, 640);
        gtk_window_set_resizable(win, 0);

        let scroll = gtk_scrolled_window_new();
        gtk_scrolled_window_set_policy(scroll, GTK_POLICY_NEVER, GTK_POLICY_AUTOMATIC);
        gtk_widget_set_hexpand(scroll, 1);
        gtk_widget_set_vexpand(scroll, 1);

        let root = new_box(GTK_ORIENTATION_VERTICAL, 12, &[]);
        set_margin_all(root, 20);

        // Dual Brand Header: BijiOS OS Logo + Host IDVE / KVM Logo
        let brand_bar = new_box(GTK_ORIENTATION_HORIZONTAL, 16, &["app-card"]);
        let os_logo = new_logo_image(64, true);
        gtk_widget_set_valign(os_logo, GTK_ALIGN_CENTER);

        let title_col = new_box(GTK_ORIENTATION_VERTICAL, 3, &[]);
        gtk_widget_set_hexpand(title_col, 1);
        gtk_widget_set_valign(title_col, GTK_ALIGN_CENTER);
        let title = new_label("BijiOS Desktop", &["app-heading"]);
        gtk_label_set_xalign(title, 0.0);
        let sub = new_label("Version 1.0.0 (Codename: Padi) — Liquid Glass", &["app-muted"]);
        gtk_label_set_xalign(sub, 0.0);
        gtk_box_append(title_col, title);
        gtk_box_append(title_col, sub);

        let idve_col = new_box(GTK_ORIENTATION_VERTICAL, 3, &[]);
        gtk_widget_set_valign(idve_col, GTK_ALIGN_CENTER);
        gtk_widget_set_halign(idve_col, GTK_ALIGN_END);
        let idve_img = new_idve_logo(120, 36);
        let idve_cap = new_label("Powered by IDVE / KVM", &["finder-meta-lbl"]);
        gtk_label_set_xalign(idve_cap, 1.0);
        gtk_box_append(idve_col, idve_img);
        gtk_box_append(idve_col, idve_cap);

        gtk_box_append(brand_bar, os_logo);
        gtk_box_append(brand_bar, title_col);
        gtk_box_append(brand_bar, idve_col);
        gtk_box_append(root, brand_bar);

        // Detailed Operating System Card (2-column Key / Value rows)
        let os_card = new_box(GTK_ORIENTATION_VERTICAL, 6, &["app-card"]);
        let os_hdr = new_label("🌱 OPERATING SYSTEM DETAILS", &["app-section-title"]);
        gtk_label_set_xalign(os_hdr, 0.0);
        gtk_box_append(os_card, os_hdr);

        let hostname = fs::read_to_string("/etc/hostname").unwrap_or_else(|_| "BijiOS".into());
        let user = std::env::var("USER").unwrap_or_else(|_| "dionipe".into());
        let (used_mb, total_mb, _) = read_mem_usage();
        let (arch, kernel) = read_arch_and_kernel();

        let os_kv: [(&str, String); 8] = [
            ("OS Release", "BijiOS v1.0.0 (Codename: Padi)".to_string()),
            ("Base Distribution", read_os_pretty_name()),
            ("Kernel & Arch", format!("{} ({})", kernel, arch)),
            ("Desktop", "Pure Rust + Native GTK4 (Zero Chromium)".to_string()),
            ("Storage Engine", "TOON v4 (Token-Oriented Object Notation)".to_string()),
            ("Host / User", format!("{}  •  {}", hostname.trim(), user)),
            ("Processor", read_cpu_summary()),
            ("Memory (RAM)", format!("{} MB / {} MB", used_mb, total_mb)),
        ];
        for (k, v) in os_kv {
            let row = new_box(GTK_ORIENTATION_HORIZONTAL, 10, &[]);
            let kl = new_label(k, &["kv-key"]);
            gtk_label_set_xalign(kl, 0.0);
            gtk_widget_set_size_request(kl, 148, -1);
            let vl = new_label(&v, &["kv-val"]);
            gtk_label_set_xalign(vl, 0.0);
            gtk_label_set_wrap(vl, 1);
            gtk_widget_set_hexpand(vl, 1);
            gtk_box_append(row, kl);
            gtk_box_append(row, vl);
            gtk_box_append(os_card, row);
        }
        gtk_box_append(root, os_card);

        // Host Virtualization Card (IDVE / KVM) with clickable hyperlink
        let host_card = new_box(GTK_ORIENTATION_HORIZONTAL, 14, &["app-card"]);
        let host_logo = new_idve_logo(104, 32);
        gtk_widget_set_valign(host_logo, GTK_ALIGN_CENTER);

        let host_col = new_box(GTK_ORIENTATION_VERTICAL, 4, &[]);
        gtk_widget_set_hexpand(host_col, 1);
        let h_hdr = new_label("☁️ HOST INFRASTRUCTURE (IDVE / KVM)", &["app-section-title"]);
        gtk_label_set_xalign(h_hdr, 0.0);
        let h_virt_row = new_box(GTK_ORIENTATION_HORIZONTAL, 8, &[]);
        let h_vk = new_label("Hypervisor", &["kv-key"]);
        gtk_label_set_xalign(h_vk, 0.0);
        gtk_widget_set_size_request(h_vk, 92, -1);
        let h_vv = new_label(&read_virt_host_info(), &["kv-val"]);
        gtk_label_set_xalign(h_vv, 0.0);
        gtk_box_append(h_virt_row, h_vk);
        gtk_box_append(h_virt_row, h_vv);

        let h_url_row = new_box(GTK_ORIENTATION_HORIZONTAL, 8, &[]);
        let h_uk = new_label("Platform", &["kv-key"]);
        gtk_label_set_xalign(h_uk, 0.0);
        gtk_widget_set_size_request(h_uk, 92, -1);
        let h_uv = new_label("Infrastructure Data Virtual Environment —", &["kv-val"]);
        gtk_label_set_xalign(h_uv, 0.0);
        let h_link = new_link_button("https://idve.cloud/ ↗", "https://idve.cloud/", &["link-btn"]);
        gtk_box_append(h_url_row, h_uk);
        gtk_box_append(h_url_row, h_uv);
        gtk_box_append(h_url_row, h_link);

        gtk_box_append(host_col, h_hdr);
        gtk_box_append(host_col, h_virt_row);
        gtk_box_append(host_col, h_url_row);

        gtk_box_append(host_card, host_logo);
        gtk_box_append(host_card, host_col);
        gtk_box_append(root, host_card);

        // Bundled Open-Source Suite Credits (Storytold — Apache License 2.0)
        let cred_card = new_box(GTK_ORIENTATION_VERTICAL, 8, &["app-card"]);
        let c_hdr = new_label("📜 BUNDLED OPEN-SOURCE SUITE (APACHE-2.0)", &["app-section-title"]);
        gtk_label_set_xalign(c_hdr, 0.0);
        gtk_box_append(cred_card, c_hdr);

        let pills_row = new_box(GTK_ORIENTATION_HORIZONTAL, 6, &[]);
        for (lbl, url) in [
            ("📝 WordCraft v0.18.1 ↗", "https://github.com/storytold/wordcraft"),
            ("📊 GridCraft v0.19.1 ↗", "https://github.com/storytold/gridcraft"),
            ("📽️ DeckCraft v0.18.1 ↗", "https://github.com/storytold/deckcraft"),
            ("📕 PDFCraft v0.21.1 ↗", "https://github.com/storytold/pdfcraft"),
        ] {
            let pill = new_link_button(lbl, url, &["link-pill"]);
            gtk_box_append(pills_row, pill);
        }
        gtk_box_append(cred_card, pills_row);

        let by_row = new_box(GTK_ORIENTATION_HORIZONTAL, 6, &[]);
        let by_k = new_label("Upstream Author:", &["kv-key"]);
        gtk_label_set_xalign(by_k, 0.0);
        let by_link = new_link_button(
            "Storytold / ArtCraft ↗",
            "https://github.com/storytold",
            &["link-btn"],
        );
        let by_sep = new_label("•", &["finder-meta-lbl"]);
        let lic_link = new_link_button(
            "Apache License 2.0 ↗",
            "https://www.apache.org/licenses/LICENSE-2.0",
            &["link-btn"],
        );
        gtk_box_append(by_row, by_k);
        gtk_box_append(by_row, by_link);
        gtk_box_append(by_row, by_sep);
        gtk_box_append(by_row, lic_link);
        gtk_box_append(cred_card, by_row);

        let repo_row = new_box(GTK_ORIENTATION_HORIZONTAL, 6, &[]);
        let repo_k = new_label("Repository:", &["kv-key"]);
        gtk_label_set_xalign(repo_k, 0.0);
        let repo_link = new_link_button(
            "github.com/dionipe/BijiOS ↗",
            "https://github.com/dionipe/BijiOS",
            &["link-btn"],
        );
        let cont_k = new_label("•  Contributor:", &["kv-key"]);
        let cont_link = new_link_button("@dionipe ↗", "https://github.com/dionipe", &["link-btn"]);
        gtk_box_append(repo_row, repo_k);
        gtk_box_append(repo_row, repo_link);
        gtk_box_append(repo_row, cont_k);
        gtk_box_append(repo_row, cont_link);
        gtk_box_append(cred_card, repo_row);

        gtk_box_append(root, cred_card);

        // Footer with clickable IDVE Cloud link
        let footer = new_box(GTK_ORIENTATION_HORIZONTAL, 4, &[]);
        gtk_widget_set_halign(footer, GTK_ALIGN_CENTER);
        let copy_lbl = new_label("™ and © 2024–2027 BijiOS  •  Hosted on", &["finder-meta-lbl"]);
        let copy_link = new_link_button("IDVE Cloud ↗", "https://idve.cloud/", &["link-btn"]);
        gtk_box_append(footer, copy_lbl);
        gtk_box_append(footer, copy_link);
        gtk_box_append(root, footer);

        gtk_scrolled_window_set_child(scroll, root);
        gtk_window_set_child(win, scroll);
        gtk_window_present(win);
    }
}

// ============================================================================
// 10. Bibit - Debian App Store (Native GTK4 APT / .deb Package Store)
// ============================================================================
#[derive(Clone)]
struct BibitPkg {
    pkg: String,
    name: String,
    category: String,
    icon: String,
    badge_cls: String,
    desc: String,
}

fn is_deb_installed(pkg: &str) -> bool {
    Command::new("dpkg-query")
        .args(["-W", "-f=${Status}", pkg])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.contains("install ok installed"))
        .unwrap_or(false)
}

fn curated_bibit_catalog() -> Vec<BibitPkg> {
    let items = [
        ("chromium", "Chromium Web Browser", "Internet", "🌐", "badge-docx", "Fast, open-source web browser"),
        ("firefox-esr", "Firefox ESR", "Internet", "🦊", "badge-pdf", "Privacy-focused Mozilla web browser"),
        ("thunderbird", "Thunderbird Mail", "Internet", "✉️", "badge-docx", "Full-featured email & calendar client"),
        ("filezilla", "FileZilla FTP", "Internet", "📡", "badge-pdf", "Fast cross-platform FTP & SFTP client"),
        ("libreoffice", "LibreOffice Suite", "Office", "📄", "badge-xlsx", "Complete open-source office productivity suite"),
        ("evince", "Evince Document Viewer", "Office", "📕", "badge-pdf", "Clean PDF and PostScript document reader"),
        ("gimp", "GIMP Image Editor", "Graphics", "🎨", "badge-img", "Professional GNU image manipulation program"),
        ("inkscape", "Inkscape Vector", "Graphics", "✒️", "badge-img", "Scalable vector graphics (SVG) editor"),
        ("vlc", "VLC Media Player", "Graphics", "🎬", "badge-pdf", "Universal video & audio media player"),
        ("audacity", "Audacity Audio", "Graphics", "🎵", "badge-docx", "Multi-track audio editor and recorder"),
        ("geany", "Geany Code IDE", "Developer", "💻", "badge-xlsx", "Fast and lightweight GTK code editor"),
        ("git", "Git Version Control", "Developer", "🌿", "badge-xlsx", "Distributed source code version control"),
        ("foot", "Foot Terminal", "Developer", "⬛", "badge-file", "Fast, lightweight Linux terminal emulator"),
        ("gparted", "GParted Disk Manager", "System", "💾", "badge-file", "Graphical disk & partition editor"),
        ("htop", "htop Process Viewer", "System", "📊", "badge-xlsx", "Interactive terminal system process monitor"),
        ("synaptic", "Synaptic Package Mgr", "System", "📦", "badge-file", "Advanced APT Debian package manager"),
    ];
    items
        .iter()
        .map(|(pkg, name, cat, icon, bcls, desc)| BibitPkg {
            pkg: (*pkg).to_string(),
            name: (*name).to_string(),
            category: (*cat).to_string(),
            icon: (*icon).to_string(),
            badge_cls: (*bcls).to_string(),
            desc: (*desc).to_string(),
        })
        .collect()
}

fn search_apt_packages(query: &str) -> Vec<BibitPkg> {
    let Ok(out) = Command::new("apt-cache").args(["search", query]).output() else {
        return Vec::new();
    };
    let text = String::from_utf8_lossy(&out.stdout);
    let mut results = Vec::new();
    for line in text.lines().take(30) {
        if let Some((pkg, desc)) = line.split_once(" - ") {
            let pkg_s = pkg.trim().to_string();
            results.push(BibitPkg {
                name: pkg_s.clone(),
                pkg: pkg_s,
                category: "APT".to_string(),
                icon: "📦".to_string(),
                badge_cls: "badge-xlsx".to_string(),
                desc: desc.trim().to_string(),
            });
        }
    }
    results
}

pub fn open_bibit(app: *mut GtkApplication) {
    unsafe {
        let win = create_app_window(app, "🛍️  Bibit - Debian App Store", 980, 620);
        let root = new_box(GTK_ORIENTATION_HORIZONTAL, 0, &[]);

        // Sidebar Categories
        let sidebar = new_box(GTK_ORIENTATION_VERTICAL, 3, &["app-sidebar"]);
        gtk_widget_set_size_request(sidebar, 200, -1);
        gtk_widget_set_hexpand(sidebar, 0);
        let sb_hdr = new_label("BIBIT STORE", &["sidebar-section-hdr"]);
        gtk_label_set_xalign(sb_hdr, 0.0);
        gtk_box_append(sidebar, sb_hdr);

        let active_cat: Rc<RefCell<String>> = Rc::new(RefCell::new("Featured".to_string()));
        let search_query: Rc<RefCell<String>> = Rc::new(RefCell::new(String::new()));

        // Main Area
        let main_col = new_box(GTK_ORIENTATION_VERTICAL, 0, &[]);
        gtk_widget_set_hexpand(main_col, 1);
        gtk_widget_set_vexpand(main_col, 1);

        // Compact Toolbar
        let toolbar = new_box(GTK_ORIENTATION_HORIZONTAL, 6, &["app-toolbar"]);
        let search_entry = gtk_entry_new();
        gtk_widget_set_hexpand(search_entry, 1);
        let ph = cstr("🔍 Search Debian (.deb) packages via APT (press Enter)...");
        gtk_entry_set_placeholder_text(search_entry, ph.as_ptr());
        let btn_search = new_button("Search APT", &["primary-btn"]);
        let btn_refresh = new_button("🔄", &["toolbar-btn"]);
        let tip_rf = cstr("Refresh package status");
        gtk_widget_set_tooltip_text(btn_refresh, tip_rf.as_ptr());

        gtk_box_append(toolbar, search_entry);
        gtk_box_append(toolbar, btn_search);
        gtk_box_append(toolbar, btn_refresh);
        gtk_box_append(main_col, toolbar);

        // Column Header
        let col_hdr = new_box(GTK_ORIENTATION_HORIZONTAL, 10, &["finder-col-header"]);
        let hdr_app = new_label("Application / Debian Package", &["finder-col-lbl"]);
        gtk_label_set_xalign(hdr_app, 0.0);
        gtk_widget_set_hexpand(hdr_app, 1);
        let hdr_cat = new_label("Category", &["finder-col-lbl"]);
        gtk_label_set_xalign(hdr_cat, 0.0);
        gtk_widget_set_size_request(hdr_cat, 96, -1);
        let hdr_act = new_label("Action", &["finder-col-lbl"]);
        gtk_label_set_xalign(hdr_act, 0.5);
        gtk_widget_set_size_request(hdr_act, 100, -1);

        gtk_box_append(col_hdr, hdr_app);
        gtk_box_append(col_hdr, hdr_cat);
        gtk_box_append(col_hdr, hdr_act);
        gtk_box_append(main_col, col_hdr);

        // Scrolled Package List
        let scroller = gtk_scrolled_window_new();
        gtk_widget_set_hexpand(scroller, 1);
        gtk_widget_set_vexpand(scroller, 1);
        let list_box = new_box(GTK_ORIENTATION_VERTICAL, 0, &[]);
        gtk_scrolled_window_set_child(scroller, list_box);
        gtk_box_append(main_col, scroller);

        // Bottom Status Bar
        let status_bar = gtk_center_box_new();
        add_css(status_bar, "finder-statusbar");
        let status_left = new_label("Bibit Debian Package Store Ready", &[]);
        let status_right = new_label("APT / dpkg Repository", &[]);
        gtk_center_box_set_start_widget(status_bar, status_left);
        gtk_center_box_set_end_widget(status_bar, status_right);
        gtk_box_append(main_col, status_bar);

        let catalog = Rc::new(curated_bibit_catalog());

        let populate: Rc<RefCell<Box<dyn Fn()>>> = Rc::new(RefCell::new(Box::new(|| {})));
        let pop_clone = Rc::clone(&populate);
        let cat_p = Rc::clone(&active_cat);
        let sq_p = Rc::clone(&search_query);
        let catalog_p = Rc::clone(&catalog);

        *populate.borrow_mut() = Box::new(move || {
            clear_box_children(list_box);
            let q = sq_p.borrow().trim().to_string();
            let cat = cat_p.borrow().clone();

            let pkgs: Vec<BibitPkg> = if !q.is_empty() {
                set_label_text(status_right, &format!("APT search: \"{}\"", q));
                let mut found: Vec<BibitPkg> = catalog_p
                    .iter()
                    .filter(|p| {
                        p.name.to_lowercase().contains(&q.to_lowercase())
                            || p.pkg.to_lowercase().contains(&q.to_lowercase())
                            || p.desc.to_lowercase().contains(&q.to_lowercase())
                    })
                    .cloned()
                    .collect();
                for ap in search_apt_packages(&q) {
                    if !found.iter().any(|x| x.pkg == ap.pkg) {
                        found.push(ap);
                    }
                }
                found
            } else if cat == "Installed" {
                set_label_text(status_right, "Installed Desktop Packages");
                catalog_p
                    .iter()
                    .filter(|p| is_deb_installed(&p.pkg))
                    .cloned()
                    .collect()
            } else if cat == "Featured" {
                set_label_text(status_right, "Curated Debian Desktop Catalog");
                catalog_p.as_ref().clone()
            } else {
                set_label_text(status_right, &format!("Category: {}", cat));
                catalog_p
                    .iter()
                    .filter(|p| p.category == cat)
                    .cloned()
                    .collect()
            };

            set_label_text(status_left, &format!("{} packages", pkgs.len()));

            if pkgs.is_empty() {
                let empty = new_label("No matching Debian packages found.", &["app-muted"]);
                set_margin_all(empty, 28);
                gtk_box_append(list_box, empty);
                return;
            }

            for (idx, item) in pkgs.into_iter().enumerate() {
                let row_cls = if idx % 2 == 0 { "finder-row-even" } else { "finder-row-odd" };
                let row = new_box(GTK_ORIENTATION_HORIZONTAL, 10, &[row_cls]);

                let badge = new_label(&item.icon, &["file-badge", &item.badge_cls]);
                gtk_widget_set_valign(badge, GTK_ALIGN_CENTER);

                let info_col = new_box(GTK_ORIENTATION_VERTICAL, 1, &[]);
                gtk_widget_set_hexpand(info_col, 1);
                let name_lbl = new_label(&format!("{}  ({})", item.name, item.pkg), &["finder-name-btn"]);
                gtk_label_set_xalign(name_lbl, 0.0);
                let desc_lbl = new_label(&item.desc, &["finder-meta-lbl"]);
                gtk_label_set_xalign(desc_lbl, 0.0);
                gtk_box_append(info_col, name_lbl);
                gtk_box_append(info_col, desc_lbl);

                let cat_lbl = new_label(&item.category, &["finder-meta-lbl"]);
                gtk_label_set_xalign(cat_lbl, 0.0);
                gtk_widget_set_size_request(cat_lbl, 96, -1);

                let installed = is_deb_installed(&item.pkg);
                let act_btn = if installed {
                    new_button("Remove", &["danger-btn"])
                } else {
                    new_button("⬇ Install", &["primary-btn"])
                };
                gtk_widget_set_size_request(act_btn, 96, -1);
                gtk_widget_set_valign(act_btn, GTK_ALIGN_CENTER);

                gtk_box_append(row, badge);
                gtk_box_append(row, info_col);
                gtk_box_append(row, cat_lbl);
                gtk_box_append(row, act_btn);
                gtk_box_append(list_box, row);

                let pkg_name = item.pkg.clone();
                let pop_act = Rc::clone(&pop_clone);
                connect_signal(act_btn, "clicked", move || {
                    let action = if installed { "remove" } else { "install" };
                    open_bibit_auth_and_run(
                        app,
                        win,
                        pkg_name.clone(),
                        action.to_string(),
                        status_left,
                        Rc::clone(&pop_act),
                    );
                });
            }
        });

        // Sidebar category buttons
        let categories = [
            ("⭐", "Featured", "Featured", true),
            ("🌐", "Internet & Web", "Internet", false),
            ("📄", "Office & Writing", "Office", false),
            ("🎨", "Graphics & Media", "Graphics", false),
            ("💻", "Developer Tools", "Developer", false),
            ("⚙️", "System & Utilities", "System", false),
            ("✅", "Installed (.deb)", "Installed", false),
        ];

        let all_btns: Rc<RefCell<Vec<*mut GtkWidget>>> = Rc::new(RefCell::new(Vec::new()));
        for (icon, label, key, active) in categories {
            let init_cls = if active { "sidebar-btn-active" } else { "sidebar-btn" };
            let btn = new_left_button(icon, label, &[init_cls]);
            gtk_box_append(sidebar, btn);
            all_btns.borrow_mut().push(btn);

            let key_s = key.to_string();
            let cat_c = Rc::clone(&active_cat);
            let sq_c = Rc::clone(&search_query);
            let pop_c = Rc::clone(&populate);
            let btns_ref = Rc::clone(&all_btns);
            connect_signal(btn, "clicked", move || {
                for &b in btns_ref.borrow().iter() {
                    remove_css(b, "sidebar-btn-active");
                    add_css(b, "sidebar-btn");
                }
                remove_css(btn, "sidebar-btn");
                add_css(btn, "sidebar-btn-active");
                *cat_c.borrow_mut() = key_s.clone();
                *sq_c.borrow_mut() = String::new();
                set_entry_text(search_entry, "");
                (pop_c.borrow())();
            });
        }

        // Search & Refresh handlers
        let do_search: Rc<dyn Fn()> = {
            let sq_s = Rc::clone(&search_query);
            let pop_s = Rc::clone(&populate);
            Rc::new(move || {
                *sq_s.borrow_mut() = get_entry_text(search_entry);
                (pop_s.borrow())();
            })
        };
        {
            let ds = Rc::clone(&do_search);
            connect_signal(btn_search, "clicked", move || ds());
        }
        {
            let ds = Rc::clone(&do_search);
            connect_signal(search_entry, "activate", move || ds());
        }
        {
            let pop_r = Rc::clone(&populate);
            connect_signal(btn_refresh, "clicked", move || (pop_r.borrow())());
        }

        gtk_box_append(root, sidebar);
        gtk_box_append(root, main_col);
        gtk_window_set_child(win, root);
        (populate.borrow())();
        gtk_window_present(win);
    }
}

/// Native GTK4 Administrator Authentication Modal + Background APT Runner for Bibit Store
unsafe fn open_bibit_auth_and_run(
    app: *mut GtkApplication,
    parent_win: *mut GtkWidget,
    pkg_name: String,
    action: String,
    status_left: *mut GtkWidget,
    refresh_list: Rc<RefCell<Box<dyn Fn()>>>,
) {
    use std::io::Write;
    use std::sync::{Arc, Mutex};

    let dlg = create_app_window(app, "🔒 Administrator Authentication", 440, 270);
    gtk_window_set_resizable(dlg, 0);
    gtk_window_set_modal(dlg, 1);
    gtk_window_set_transient_for(dlg, parent_win);

    let root = new_box(GTK_ORIENTATION_VERTICAL, 12, &[]);
    set_margin_all(root, 20);

    let verb = if action == "remove" { "Remove" } else { "Install" };
    let hdr = new_label(&format!("🔒 {} '{}' via APT", verb, pkg_name), &["app-heading"]);
    gtk_label_set_xalign(hdr, 0.0);

    let user = std::env::var("USER").unwrap_or_else(|_| "dionipe".to_string());
    let sub = new_label(
        &format!(
            "Administrator privileges (sudo/root) are required to {} Debian packages.\nEnter password for '{}':",
            action, user
        ),
        &["app-muted"],
    );
    gtk_label_set_xalign(sub, 0.0);
    gtk_label_set_wrap(sub, 1);

    let pw_entry = gtk_entry_new();
    let ph = cstr("Enter sudo / login password");
    gtk_entry_set_placeholder_text(pw_entry, ph.as_ptr());
    gtk_entry_set_visibility(pw_entry, 0);

    let err_lbl = new_label("", &["finder-meta-lbl"]);
    gtk_label_set_xalign(err_lbl, 0.0);
    gtk_widget_set_visible(err_lbl, 0);

    let btn_row = new_box(GTK_ORIENTATION_HORIZONTAL, 8, &[]);
    gtk_widget_set_halign(btn_row, GTK_ALIGN_END);
    let btn_cancel = new_button("Cancel", &["toolbar-btn"]);
    let btn_auth = new_button(&format!("🔓 Authenticate & {}", verb), &["primary-btn"]);
    gtk_box_append(btn_row, btn_cancel);
    gtk_box_append(btn_row, btn_auth);

    gtk_box_append(root, hdr);
    gtk_box_append(root, sub);
    gtk_box_append(root, pw_entry);
    gtk_box_append(root, err_lbl);
    gtk_box_append(root, btn_row);

    connect_signal(btn_cancel, "clicked", move || {
        gtk_window_close(dlg);
    });

    let do_auth: Rc<dyn Fn()> = {
        let user_c = user.clone();
        let pkg_c = pkg_name.clone();
        let act_c = action.clone();
        let rf_c = Rc::clone(&refresh_list);
        Rc::new(move || {
            let pw = get_entry_text(pw_entry);
            if pw.is_empty() {
                set_label_text(err_lbl, "❌ Password cannot be empty.");
                gtk_widget_set_visible(err_lbl, 1);
                return;
            }
            if !crate::native_de::authenticate_via_daemon(&user_c, &pw) {
                set_label_text(err_lbl, "❌ Incorrect password. Please try again.");
                gtk_widget_set_visible(err_lbl, 1);
                set_entry_text(pw_entry, "");
                gtk_widget_grab_focus(pw_entry);
                return;
            }

            gtk_window_close(dlg);

            let gerund = if act_c == "remove" { "Removing" } else { "Installing" };
            set_label_text(
                status_left,
                &format!("⏳ {} '{}' via APT (please wait)...", gerund, pkg_c),
            );

            // Track background APT completion safely
            let result_state: Arc<Mutex<Option<(bool, String)>>> = Arc::new(Mutex::new(None));
            let res_bg = Arc::clone(&result_state);
            let pkg_bg = pkg_c.clone();
            let act_bg = act_c.clone();
            let pw_bg = pw;

            std::thread::spawn(move || {
                let mut child = match Command::new("sudo")
                    .args([
                        "-S",
                        "env",
                        "DEBIAN_FRONTEND=noninteractive",
                        "apt-get",
                        &act_bg,
                        "-y",
                        &pkg_bg,
                    ])
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn()
                {
                    Ok(c) => c,
                    Err(e) => {
                        if let Ok(mut g) = res_bg.lock() {
                            *g = Some((false, format!("Failed to spawn sudo: {}", e)));
                        }
                        return;
                    }
                };

                if let Some(mut stdin) = child.stdin.take() {
                    let _ = writeln!(stdin, "{}", pw_bg);
                }

                let out = child.wait_with_output();
                let (ok, detail) = match out {
                    Ok(o) if o.status.success() => {
                        let _ = Command::new("/usr/bin/bijios-app-sync")
                            .stdin(Stdio::null())
                            .stdout(Stdio::null())
                            .stderr(Stdio::null())
                            .status();
                        (true, String::new())
                    }
                    Ok(o) => {
                        let err_txt = String::from_utf8_lossy(&o.stderr);
                        let last = err_txt
                            .lines()
                            .rev()
                            .find(|l| !l.trim().is_empty() && !l.contains("[sudo]"))
                            .unwrap_or("APT returned non-zero exit code")
                            .trim()
                            .to_string();
                        (false, last)
                    }
                    Err(e) => (false, e.to_string()),
                };

                if let Ok(mut g) = res_bg.lock() {
                    *g = Some((ok, detail));
                }
            });

            // Poll completion on GTK main thread (destroy-safe)
            let parent_alive = Rc::new(std::cell::Cell::new(true));
            {
                let pa = Rc::clone(&parent_alive);
                connect_signal(parent_win, "destroy", move || pa.set(false));
            }
            let rf_poll = Rc::clone(&rf_c);
            let pkg_done = pkg_c.clone();
            let act_done = act_c.clone();
            timeout_add_ms(400, move || {
                if !parent_alive.get() {
                    return false;
                }
                let finished = result_state.lock().ok().and_then(|g| g.clone());
                if let Some((ok, detail)) = finished {
                    if ok {
                        let past = if act_done == "remove" { "removed" } else { "installed" };
                        (rf_poll.borrow())();
                        set_label_text(
                            status_left,
                            &format!("✅ Successfully {} '{}'!", past, pkg_done),
                        );
                    } else {
                        set_label_text(
                            status_left,
                            &format!("❌ APT {} failed for '{}': {}", act_done, pkg_done, detail),
                        );
                    }
                    return false;
                }
                true
            });
        })
    };

    {
        let da = Rc::clone(&do_auth);
        connect_signal(btn_auth, "clicked", move || da());
    }
    {
        let da = Rc::clone(&do_auth);
        connect_signal(pw_entry, "activate", move || da());
    }

    gtk_window_set_child(dlg, root);
    gtk_window_present(dlg);
    gtk_widget_grab_focus(pw_entry);
}

// ============================================================================
// 11. Pancer - Native Terminal (Delegates to Multi-Tab Split-Pane Pancer Module)
// ============================================================================
pub fn open_pancer(app: *mut GtkApplication) {
    crate::native_de::pancer::open_pancer(app);
}

//! BijiOS Native Rust + GTK4 Desktop Applications
//! Finder, Launchpad (.deb App Launcher), WordCraft, GridCraft, Notes,
//! Calculator, Activity Monitor, System Settings, and About BijiOS.

use crate::docx::DocxEngine;
use crate::native_de::gtk4_ffi::*;
use crate::xlsx::XlsxEngine;
use std::cell::RefCell;
use std::collections::HashMap;
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
            if id.is_empty() || id.contains("bijios") || id.contains("dokuhub") {
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
        add_css(win, "app-window");

        // Custom macOS Liquid Glass Titlebar (Traffic Lights + Integrated Tab Pill — ekspektasi/toolbar.png)
        let handle = gtk_window_handle_new();
        let tb = gtk_center_box_new();
        add_css(tb, "mac-titlebar");

        let start_row = new_box(GTK_ORIENTATION_HORIZONTAL, 8, &[]);
        gtk_widget_set_valign(start_row, GTK_ALIGN_CENTER);

        let lights = new_box(GTK_ORIENTATION_HORIZONTAL, 7, &[]);
        gtk_widget_set_valign(lights, GTK_ALIGN_CENTER);
        let btn_close = new_button("", &["traffic-btn", "traffic-close"]);
        let btn_min = new_button("", &["traffic-btn", "traffic-min"]);
        let btn_max = new_button("", &["traffic-btn", "traffic-max"]);
        let tc = cstr("Close");
        let tm = cstr("Minimize");
        let tx = cstr("Maximize / Restore");
        gtk_widget_set_tooltip_text(btn_close, tc.as_ptr());
        gtk_widget_set_tooltip_text(btn_min, tm.as_ptr());
        gtk_widget_set_tooltip_text(btn_max, tx.as_ptr());

        connect_signal(btn_close, "clicked", move || gtk_window_close(win));
        connect_signal(btn_min, "clicked", move || {
            let effect = crate::native_de::get_minimize_effect();
            let steps: i32 = if effect == "genie" { 5 } else { 3 };
            let step_counter = Rc::new(RefCell::new(0i32));
            let sc = Rc::clone(&step_counter);
            timeout_add_ms(22, move || {
                let mut cur = sc.borrow_mut();
                *cur += 1;
                if *cur <= steps {
                    let frac = 1.0 - (*cur as f64 / (steps as f64 + 0.5));
                    gtk_widget_set_opacity(win, frac.max(0.15));
                    true
                } else {
                    gtk_window_minimize(win);
                    gtk_widget_set_opacity(win, 1.0);
                    false
                }
            });
        });
        connect_signal(btn_max, "clicked", move || {
            if gtk_window_is_maximized(win) != 0 {
                gtk_window_unmaximize(win);
            } else {
                gtk_window_maximize(win);
            }
        });

        // Animate opening applications (smooth fade-in when enabled)
        if crate::native_de::get_animate_opening() {
            gtk_widget_set_opacity(win, 0.25);
            let step = Rc::new(RefCell::new(1i32));
            let st = Rc::clone(&step);
            timeout_add_ms(20, move || {
                let mut c = st.borrow_mut();
                *c += 1;
                let op = (*c as f64 * 0.2).min(1.0);
                gtk_widget_set_opacity(win, op);
                op < 1.0
            });
        }

        gtk_box_append(lights, btn_close);
        gtk_box_append(lights, btn_min);
        gtk_box_append(lights, btn_max);

        let pill_btn = new_button("▾", &["mac-title-pill-btn"]);
        let tip_p = cstr("Toggle Maximize");
        gtk_widget_set_tooltip_text(pill_btn, tip_p.as_ptr());
        connect_signal(pill_btn, "clicked", move || {
            if gtk_window_is_maximized(win) != 0 {
                gtk_window_unmaximize(win);
            } else {
                gtk_window_maximize(win);
            }
        });

        let tab_pill = new_box(GTK_ORIENTATION_HORIZONTAL, 8, &["mac-title-tab"]);
        let title_lbl = new_label(title, &["mac-titlebar-title"]);
        gtk_box_append(tab_pill, title_lbl);

        gtk_box_append(start_row, lights);
        gtk_box_append(start_row, pill_btn);
        gtk_box_append(start_row, tab_pill);

        gtk_center_box_set_start_widget(tb, start_row);

        gtk_window_handle_set_child(handle, tb);
        gtk_window_set_titlebar(win, handle);

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
    unsafe {
        let win = create_app_window(app, "📁  Akar - File Manager", 980, 620);
        let root = new_box(GTK_ORIENTATION_HORIZONTAL, 0, &[]);

        // Sidebar
        let sidebar = new_box(GTK_ORIENTATION_VERTICAL, 3, &["app-sidebar"]);
        gtk_widget_set_size_request(sidebar, 200, -1);
        gtk_widget_set_hexpand(sidebar, 0);
        let sb_hdr1 = new_label("MY STORAGE", &["sidebar-section-hdr"]);
        gtk_label_set_xalign(sb_hdr1, 0.0);
        gtk_box_append(sidebar, sb_hdr1);

        let cur_dir: Rc<RefCell<PathBuf>> = Rc::new(RefCell::new(home_dir()));

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

        let btn_new_folder = new_button("+ Folder", &["toolbar-btn"]);
        let tip_nf = cstr("Create New Folder");
        gtk_widget_set_tooltip_text(btn_new_folder, tip_nf.as_ptr());

        let btn_new_doc = new_button("+ New Doc", &["primary-btn"]);
        let tip_nd = cstr("Create New WordCraft Document");
        gtk_widget_set_tooltip_text(btn_new_doc, tip_nd.as_ptr());

        let btn_refresh = new_button("🔄", &["toolbar-btn"]);
        let tip_rf = cstr("Refresh Directory");
        gtk_widget_set_tooltip_text(btn_refresh, tip_rf.as_ptr());

        gtk_box_append(toolbar, btn_up);
        gtk_box_append(toolbar, path_entry);
        gtk_box_append(toolbar, btn_new_folder);
        gtk_box_append(toolbar, btn_new_doc);
        gtk_box_append(toolbar, btn_refresh);
        gtk_box_append(main_col, toolbar);

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

        // Populate directory function
        let populate: Rc<RefCell<Box<dyn Fn()>>> = Rc::new(RefCell::new(Box::new(|| {})));
        let populate_clone = Rc::clone(&populate);
        let cur_dir_p = Rc::clone(&cur_dir);

        *populate.borrow_mut() = Box::new(move || {
            clear_box_children(list_box);
            let dir = cur_dir_p.borrow().clone();
            set_entry_text(path_entry, &dir.to_string_lossy());

            let mut entries: Vec<(String, PathBuf, bool, u64)> = Vec::new();
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
                    entries.push((name, p, is_dir, size));
                }
            }
            entries.sort_by(|a, b| b.2.cmp(&a.2).then(a.0.to_lowercase().cmp(&b.0.to_lowercase())));

            set_label_text(status_lbl, &format!("{} items", entries.len()));
            set_label_text(status_right, &format!("Location: {}", dir.display()));

            if entries.is_empty() {
                let empty = new_label("This folder is empty.", &["app-muted"]);
                set_margin_all(empty, 28);
                gtk_box_append(list_box, empty);
                return;
            }

            for (idx, (name, path, is_dir, size)) in entries.into_iter().enumerate() {
                let row_cls = if idx % 2 == 0 { "finder-row-even" } else { "finder-row-odd" };
                let row = new_box(GTK_ORIENTATION_HORIZONTAL, 10, &[row_cls]);

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

                let badge = new_label(badge_txt, &["file-badge", badge_cls]);
                gtk_widget_set_valign(badge, GTK_ALIGN_CENTER);

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

                gtk_box_append(row, badge);
                gtk_box_append(row, open_btn);
                gtk_box_append(row, size_lbl);
                gtk_box_append(row, kind_lbl);
                gtk_box_append(row, del_btn);
                gtk_box_append(list_box, row);

                let cur_dir_c = Rc::clone(&cur_dir_p);
                let pop_c = Rc::clone(&populate_clone);
                let p_open = path.clone();
                connect_signal(open_btn, "clicked", move || {
                    if is_dir {
                        *cur_dir_c.borrow_mut() = p_open.clone();
                        (pop_c.borrow())();
                    } else {
                        let lower = p_open.to_string_lossy().to_lowercase();
                        if lower.ends_with(".txt") || lower.ends_with(".md") || lower.ends_with(".rs") || lower.ends_with(".json") || lower.ends_with(".toon") {
                            open_wordcraft(app, Some(p_open.clone()));
                        } else {
                            let _ = Command::new("xdg-open")
                                .arg(&p_open)
                                .stdin(Stdio::null())
                                .stdout(Stdio::null())
                                .stderr(Stdio::null())
                                .spawn();
                        }
                    }
                });

                let pop_d = Rc::clone(&populate_clone);
                let p_del = path.clone();
                connect_signal(del_btn, "clicked", move || {
                    if is_dir {
                        let _ = fs::remove_dir_all(&p_del);
                    } else {
                        let _ = fs::remove_file(&p_del);
                    }
                    (pop_d.borrow())();
                });
            }
        });

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
                if target.exists() {
                    *cur_dir_s.borrow_mut() = target.clone();
                    (pop_s.borrow())();
                }
            });
        }

        // Toolbar handlers
        {
            let cur_dir_u = Rc::clone(&cur_dir);
            let pop_u = Rc::clone(&populate);
            connect_signal(btn_up, "clicked", move || {
                let parent = cur_dir_u.borrow().parent().map(|p| p.to_path_buf());
                if let Some(p) = parent {
                    *cur_dir_u.borrow_mut() = p;
                    (pop_u.borrow())();
                }
            });
        }
        {
            let cur_dir_e = Rc::clone(&cur_dir);
            let pop_e = Rc::clone(&populate);
            connect_signal(path_entry, "activate", move || {
                let txt = get_entry_text(path_entry);
                let p = PathBuf::from(txt.trim());
                if p.is_dir() {
                    *cur_dir_e.borrow_mut() = p;
                    (pop_e.borrow())();
                }
            });
        }
        {
            let cur_dir_nf = Rc::clone(&cur_dir);
            let pop_nf = Rc::clone(&populate);
            connect_signal(btn_new_folder, "clicked", move || {
                let base = cur_dir_nf.borrow().clone();
                let ts = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()
                    % 10000;
                let _ = fs::create_dir_all(base.join(format!("New_Folder_{}", ts)));
                (pop_nf.borrow())();
            });
        }
        connect_signal(btn_new_doc, "clicked", move || {
            open_wordcraft(app, None);
        });
        {
            let pop_r = Rc::clone(&populate);
            connect_signal(btn_refresh, "clicked", move || {
                (pop_r.borrow())();
            });
        }

        gtk_box_append(root, sidebar);
        gtk_box_append(root, main_col);
        gtk_window_set_child(win, root);
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
                    let btn = new_icon_tile_button(&da.icon, icon_cls, &short_name, &["launchpad-tile"]);
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

// ============================================================================
// 3. BijiOS WordCraft (Native Document Editor)
// ============================================================================
pub fn open_wordcraft(app: *mut GtkApplication, initial_file: Option<PathBuf>) {
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
// 4. BijiOS GridCraft (Native Spreadsheet)
// ============================================================================
pub fn open_gridcraft(app: *mut GtkApplication) {
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
    unsafe {
        let win = create_app_window(app, "📈 BijiOS Activity Monitor", 780, 540);
        let root = new_box(GTK_ORIENTATION_VERTICAL, 12, &[]);
        set_margin_all(root, 18);

        let title = new_label("BijiOS Real-Time System & Process Monitor", &["app-heading"]);
        gtk_label_set_xalign(title, 0.0);
        gtk_box_append(root, title);

        // Stats cards
        let stats_row = new_box(GTK_ORIENTATION_HORIZONTAL, 12, &[]);
        gtk_box_set_homogeneous(stats_row, 1);

        let cpu_card = new_box(GTK_ORIENTATION_VERTICAL, 6, &["app-card"]);
        gtk_box_append(cpu_card, new_label("⚡ CPU Load Average", &["app-section-title"]));
        let cpu_lbl = new_label(&read_load_avg(), &["app-heading"]);
        gtk_box_append(cpu_card, cpu_lbl);

        let mem_card = new_box(GTK_ORIENTATION_VERTICAL, 6, &["app-card"]);
        gtk_box_append(mem_card, new_label("🧠 Memory (RAM)", &["app-section-title"]));
        let (used_mb, total_mb, pct) = read_mem_usage();
        let mem_lbl = new_label(&format!("{} MB / {} MB ({:.0}%)", used_mb, total_mb, pct * 100.0), &["app-muted"]);
        let mem_bar = gtk_progress_bar_new();
        gtk_progress_bar_set_fraction(mem_bar, pct);
        gtk_box_append(mem_card, mem_lbl);
        gtk_box_append(mem_card, mem_bar);

        gtk_box_append(stats_row, cpu_card);
        gtk_box_append(stats_row, mem_card);
        gtk_box_append(root, stats_row);

        let proc_title = new_label("Running Linux & BijiOS Processes (/proc)", &["app-section-title"]);
        gtk_label_set_xalign(proc_title, 0.0);
        gtk_box_append(root, proc_title);

        let scroller = gtk_scrolled_window_new();
        gtk_widget_set_hexpand(scroller, 1);
        gtk_widget_set_vexpand(scroller, 1);
        let proc_tv = gtk_text_view_new();
        gtk_text_view_set_editable(proc_tv, 0);
        gtk_text_view_set_monospace(proc_tv, 1);
        add_css(proc_tv, "editor-view");
        gtk_scrolled_window_set_child(scroller, proc_tv);
        gtk_box_append(root, scroller);

        let update_procs = move || {
            let (u, t, p) = read_mem_usage();
            set_label_text(cpu_lbl, &read_load_avg());
            set_label_text(mem_lbl, &format!("{} MB / {} MB ({:.0}%)", u, t, p * 100.0));
            gtk_progress_bar_set_fraction(mem_bar, p);

            let mut lines = vec![format!("{:<8} {:<26} {:<10}", "PID", "PROCESS", "RSS (MB)")];
            lines.push("------------------------------------------------".to_string());
            let mut procs: Vec<(u32, String, f64)> = Vec::new();
            if let Ok(rd) = fs::read_dir("/proc") {
                for ent in rd.flatten() {
                    if let Ok(pid) = ent.file_name().to_string_lossy().parse::<u32>() {
                        let comm = fs::read_to_string(ent.path().join("comm"))
                            .unwrap_or_default()
                            .trim()
                            .to_string();
                        let mut rss_kb = 0u64;
                        if let Ok(st) = fs::read_to_string(ent.path().join("status")) {
                            for l in st.lines() {
                                if l.starts_with("VmRSS:") {
                                    rss_kb = l.split_whitespace().nth(1).and_then(|s| s.parse().ok()).unwrap_or(0);
                                    break;
                                }
                            }
                        }
                        if !comm.is_empty() {
                            procs.push((pid, comm, rss_kb as f64 / 1024.0));
                        }
                    }
                }
            }
            procs.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
            for (pid, comm, rss) in procs.into_iter().take(35) {
                lines.push(format!("{:<8} {:<26} {:>7.1} MB", pid, comm, rss));
            }
            set_text_view_text(proc_tv, &lines.join("\n"));
        };

        update_procs();
        timeout_add_sec(2, move || {
            if gtk_widget_get_visible(win) == 0 {
                return false;
            }
            update_procs();
            true
        });

        gtk_window_set_child(win, root);
        gtk_window_present(win);
    }
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
    if !vendor.is_empty() || !product.is_empty() {
        format!("IDVE Cloud / {} ({} {})", virt, vendor, product).trim().to_string()
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
        let win = create_app_window(app, "About BijiOS", 620, 560);
        gtk_window_set_resizable(win, 0);
        let root = new_box(GTK_ORIENTATION_VERTICAL, 14, &[]);
        set_margin_all(root, 24);

        // Dual Brand Header: BijiOS OS Logo + Host IDVE / KVM Logo
        let brand_bar = new_box(GTK_ORIENTATION_HORIZONTAL, 18, &["app-card"]);
        let os_logo = new_logo_image(68, true);
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
        let idve_img = new_idve_logo(124, 38);
        let idve_cap = new_label("Powered by IDVE / KVM", &["finder-meta-lbl"]);
        gtk_label_set_xalign(idve_cap, 1.0);
        gtk_box_append(idve_col, idve_img);
        gtk_box_append(idve_col, idve_cap);

        gtk_box_append(brand_bar, os_logo);
        gtk_box_append(brand_bar, title_col);
        gtk_box_append(brand_bar, idve_col);
        gtk_box_append(root, brand_bar);

        // Detailed Operating System Card
        let os_card = new_box(GTK_ORIENTATION_VERTICAL, 6, &["app-card"]);
        let os_hdr = new_label("🌱 OPERATING SYSTEM DETAILS", &["app-section-title"]);
        gtk_label_set_xalign(os_hdr, 0.0);
        gtk_box_append(os_card, os_hdr);

        let hostname = fs::read_to_string("/etc/hostname").unwrap_or_else(|_| "BijiOS".into());
        let user = std::env::var("USER").unwrap_or_else(|_| "dionipe".into());
        let (used_mb, total_mb, _) = read_mem_usage();
        let (arch, kernel) = read_arch_and_kernel();

        let os_rows = [
            "OS Release: BijiOS v1.0.0 (Codename: Padi)".to_string(),
            format!("Base Distribution: {}", read_os_pretty_name()),
            format!("Kernel & Architecture: {} ({})", kernel, arch),
            "Desktop Environment: Pure Rust + Native GTK4 (Zero Chromium)".to_string(),
            "Storage Engine: TOON v4 (Token-Oriented Object Notation)".to_string(),
            format!("Hostname / Active User: {}  •  {}", hostname.trim(), user),
            format!("Processor: {}", read_cpu_summary()),
            format!("Memory (RAM): {} MB / {} MB", used_mb, total_mb),
        ];
        for r in os_rows {
            let l = new_label(&r, &["app-muted"]);
            gtk_label_set_xalign(l, 0.0);
            gtk_label_set_wrap(l, 1);
            gtk_box_append(os_card, l);
        }
        gtk_box_append(root, os_card);

        // Host Virtualization Card (IDVE / KVM)
        let host_card = new_box(GTK_ORIENTATION_HORIZONTAL, 14, &["app-card"]);
        let host_logo = new_idve_logo(108, 34);
        gtk_widget_set_valign(host_logo, GTK_ALIGN_CENTER);

        let host_col = new_box(GTK_ORIENTATION_VERTICAL, 3, &[]);
        gtk_widget_set_hexpand(host_col, 1);
        let h_hdr = new_label("☁️ HOST INFRASTRUCTURE (IDVE / KVM)", &["app-section-title"]);
        gtk_label_set_xalign(h_hdr, 0.0);
        let h_virt = new_label(&format!("Hypervisor: {}", read_virt_host_info()), &["app-muted"]);
        gtk_label_set_xalign(h_virt, 0.0);
        let h_url = new_label("Cloud Platform: Indonesia Virtual Environment — https://idve.cloud/", &["finder-meta-lbl"]);
        gtk_label_set_xalign(h_url, 0.0);
        gtk_box_append(host_col, h_hdr);
        gtk_box_append(host_col, h_virt);
        gtk_box_append(host_col, h_url);

        gtk_box_append(host_card, host_logo);
        gtk_box_append(host_card, host_col);
        gtk_box_append(root, host_card);

        let copy = new_label("™ and © 2024–2027 BijiOS • Hosted on IDVE Cloud (https://idve.cloud/)", &["finder-meta-lbl"]);
        gtk_box_append(root, copy);

        gtk_window_set_child(win, root);
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
                    set_label_text(
                        status_left,
                        &format!("Running APT {} for '{}' (authentication may be requested)...", action, pkg_name),
                    );
                    let pkg_bg = pkg_name.clone();
                    let act_bg = action.to_string();
                    std::thread::spawn(move || {
                        let _ = Command::new("pkexec")
                            .args(["apt-get", &act_bg, "-y", &pkg_bg])
                            .stdin(Stdio::null())
                            .stdout(Stdio::null())
                            .stderr(Stdio::null())
                            .status();
                        let _ = Command::new("/usr/bin/bijios-app-sync")
                            .stdin(Stdio::null())
                            .stdout(Stdio::null())
                            .stderr(Stdio::null())
                            .status();
                    });
                    let _ = &pop_act;
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

// ============================================================================
// 11. Pancer - Native Terminal (Real Interactive PTY/TTY Bash Shell via VTE GTK4)
// ============================================================================
pub fn open_pancer(app: *mut GtkApplication) {
    unsafe {
        let win = create_app_window(app, "💻  Pancer", 860, 520);
        let root = new_box(GTK_ORIENTATION_VERTICAL, 0, &[]);

        // Compact minimal toolbar (Copy, Paste, Clear, + New)
        let toolbar = new_box(GTK_ORIENTATION_HORIZONTAL, 6, &["pancer-toolbar"]);
        let badge = new_label("bash (pty/tty)", &["pancer-cwd-pill"]);
        let tb_spacer = new_box(GTK_ORIENTATION_HORIZONTAL, 0, &[]);
        gtk_widget_set_hexpand(tb_spacer, 1);

        let btn_copy = new_button("Copy", &["pancer-btn"]);
        let btn_paste = new_button("Paste", &["pancer-btn"]);
        let btn_clear = new_button("Clear", &["pancer-btn"]);
        let btn_new = new_button("+ New", &["pancer-btn"]);

        gtk_box_append(toolbar, badge);
        gtk_box_append(toolbar, tb_spacer);
        gtk_box_append(toolbar, btn_copy);
        gtk_box_append(toolbar, btn_paste);
        gtk_box_append(toolbar, btn_clear);
        gtk_box_append(toolbar, btn_new);
        gtk_box_append(root, toolbar);

        // Real VTE GTK4 Terminal Widget (Full PTY/TTY, ANSI 256-color, vim/htop/sudo/tab-complete)
        let vte = vte_terminal_new();
        gtk_widget_set_hexpand(vte, 1);
        gtk_widget_set_vexpand(vte, 1);
        vte_terminal_set_scrollback_lines(vte, 10000);

        // Monospace font
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
            GdkRGBA::hex(0x12, 0x33, 0x2A), // 0 black (Hutan Malam)
            GdkRGBA::hex(0xE0, 0x5A, 0x4F), // 1 red
            GdkRGBA::hex(0x2F, 0x7D, 0x4F), // 2 green (Hijau Tunas)
            GdkRGBA::hex(0xE8, 0xA3, 0x3D), // 3 yellow (Emas Benih)
            GdkRGBA::hex(0x4E, 0x9A, 0xF1), // 4 blue
            GdkRGBA::hex(0xB4, 0x8E, 0xAD), // 5 magenta
            GdkRGBA::hex(0x4F, 0xB8, 0xA6), // 6 cyan
            GdkRGBA::hex(0xE5, 0xDC, 0xC8), // 7 white
            GdkRGBA::hex(0x3B, 0x5C, 0x52), // 8 bright black
            GdkRGBA::hex(0xF8, 0x71, 0x71), // 9 bright red
            GdkRGBA::hex(0x7F, 0xBF, 0x6A), // 10 bright green (Hijau Muda)
            GdkRGBA::hex(0xF6, 0xC4, 0x69), // 11 bright yellow
            GdkRGBA::hex(0x7D, 0xD3, 0xFC), // 12 bright blue
            GdkRGBA::hex(0xD8, 0xB4, 0xFE), // 13 bright magenta
            GdkRGBA::hex(0x5E, 0xEA, 0xD4), // 14 bright cyan
            GdkRGBA::hex(0xFF, 0xFD, 0xF7), // 15 bright white
        ];
        vte_terminal_set_colors(vte, &fg, &bg, palette.as_ptr(), palette.len());
        vte_terminal_set_color_cursor(vte, &cursor);

        // Spawn real interactive /bin/bash shell in a Linux PTY
        let home = home_dir().to_string_lossy().to_string();
        let c_home = cstr(&home);
        let c_shell = cstr("/bin/bash");
        let argv = [c_shell.as_ptr(), std::ptr::null()];
        vte_terminal_spawn_async(
            vte,
            0, // VTE_PTY_DEFAULT
            c_home.as_ptr(),
            argv.as_ptr(),
            std::ptr::null(), // inherit environment
            0,                // G_SPAWN_DEFAULT
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            -1,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        );

        // Toolbar actions
        connect_signal(btn_copy, "clicked", move || {
            vte_terminal_copy_clipboard_format(vte, 1); // VTE_FORMAT_TEXT
            gtk_widget_grab_focus(vte);
        });
        connect_signal(btn_paste, "clicked", move || {
            vte_terminal_paste_clipboard(vte);
            gtk_widget_grab_focus(vte);
        });
        connect_signal(btn_clear, "clicked", move || {
            vte_terminal_reset(vte, 1, 1);
            gtk_widget_grab_focus(vte);
        });
        connect_signal(btn_new, "clicked", move || {
            open_pancer(app);
        });

        // Auto-close window when user types exit or Ctrl+D (child-exited has 3 args: vte, status, data)
        connect_notify(vte, "child-exited", move || {
            gtk_window_close(win);
        });

        gtk_box_append(root, vte);
        gtk_window_set_child(win, root);
        gtk_window_present(win);
        gtk_widget_grab_focus(vte);
    }
}

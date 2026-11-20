//! Pancer — BijiOS Native Interactive Multi-Tab & Split-Pane PTY/TTY Terminal
//! Powered by Pure Rust + GTK4 + VTE4 (`libvte-2.91-gtk4`)
//! Features:
//! - Menu Bar: File (New Window, New Tab), Edit (Copy, Paste, Clear),
//!   Shell (Duplicate Window, Duplicate Tab, Split Vertical, Split Horizontal, Close Tab)
//! - Dynamic Host Tab Bar: | host 1 | host 2 | + | (auto-detects local host vs active SSH host)

use crate::native_de::apps::{create_app_window, home_dir};
use crate::native_de::gtk4_ffi::*;
use std::cell::{Cell, RefCell};
use std::ffi::{c_int, CStr};
use std::fs;
use std::path::PathBuf;
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
    let argv = [c_shell.as_ptr(), std::ptr::null()];
    vte_terminal_spawn_async(
        vte,
        0,
        c_wd.as_ptr(),
        argv.as_ptr(),
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

pub fn open_pancer_with_dir(app: *mut GtkApplication, initial_dir: Option<PathBuf>) {
    unsafe {
        let win = create_app_window(app, "💻  Pancer", 920, 560);

        let overlay = gtk_overlay_new();
        let root = new_box(GTK_ORIENTATION_VERTICAL, 0, &[]);
        gtk_widget_set_hexpand(root, 1);
        gtk_widget_set_vexpand(root, 1);
        gtk_overlay_set_child(overlay, root);

        // ---------------------------------------------------------------------
        // 1. Pancer Menu Bar (File | Edit | Shell)
        // ---------------------------------------------------------------------
        let menubar = new_box(GTK_ORIENTATION_HORIZONTAL, 4, &["pancer-menubar"]);
        let btn_file = new_button("File", &["pancer-menu-btn"]);
        let btn_edit = new_button("Edit", &["pancer-menu-btn"]);
        let btn_shell = new_button("Shell", &["pancer-menu-btn"]);
        gtk_box_append(menubar, btn_file);
        gtk_box_append(menubar, btn_edit);
        gtk_box_append(menubar, btn_shell);
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

        let any_menu_open = move || -> bool {
            gtk_widget_get_visible(menu_file) != 0
                || gtk_widget_get_visible(menu_edit) != 0
                || gtk_widget_get_visible(menu_shell) != 0
        };

        let hide_all_menus = move || {
            gtk_widget_set_visible(menu_backdrop, 0);
            gtk_widget_set_visible(menu_file, 0);
            gtk_widget_set_visible(menu_edit, 0);
            gtk_widget_set_visible(menu_shell, 0);
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

        // ---------------------------------------------------------------------
        // 2. Dynamic Host Tab Bar: | 💻 BijiOS | 🔐 host2 | + |
        // ---------------------------------------------------------------------
        let tabbar = new_box(GTK_ORIENTATION_HORIZONTAL, 5, &["pancer-tabbar"]);
        let tabs_box = new_box(GTK_ORIENTATION_HORIZONTAL, 4, &[]);
        let btn_add_tab = new_button("+", &["pancer-add-tab"]);
        let tip_add = cstr("New Tab (Ctrl+Shift+T)");
        gtk_widget_set_tooltip_text(btn_add_tab, tip_add.as_ptr());

        gtk_box_append(tabbar, tabs_box);
        gtk_box_append(tabbar, btn_add_tab);
        gtk_box_append(root, tabbar);

        // ---------------------------------------------------------------------
        // 3. Terminal Tabs Stack & Lifecycle Guard
        // ---------------------------------------------------------------------
        let stack = gtk_stack_new();
        gtk_widget_set_hexpand(stack, 1);
        gtk_widget_set_vexpand(stack, 1);
        gtk_box_append(root, stack);

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
        let mi_close_tab = new_menu_item("✖", "Close Tab", "Ctrl+Shift+W");

        gtk_box_append(menu_shell, mi_dup_win);
        gtk_box_append(menu_shell, mi_dup_tab);
        gtk_box_append(menu_shell, gtk_separator_new(GTK_ORIENTATION_HORIZONTAL));
        gtk_box_append(menu_shell, mi_split_v);
        gtk_box_append(menu_shell, mi_split_h);
        gtk_box_append(menu_shell, gtk_separator_new(GTK_ORIENTATION_HORIZONTAL));
        gtk_box_append(menu_shell, mi_close_tab);

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
            let f_ct = Rc::clone(&act_close_tab);
            let st_k = Rc::clone(&state);
            connect_key_pressed(win, move |keyval, mods| {
                // Escape closes any open dropdown menu cleanly
                if keyval == 0xff1b && any_menu_open() {
                    hide_all_menus();
                    let pane = st_k.try_borrow().ok().and_then(|s| s.active_pane());
                    if let Some(p) = pane {
                        gtk_widget_grab_focus(p.vte);
                    }
                    return true;
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
                    Some('W') => { f_ct(); true }
                    _ => false,
                }
            });
        }

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

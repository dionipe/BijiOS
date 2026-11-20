//! BijiOS 1.0.0 "Padi" — Full Native Desktop Environment (Pure Rust + GTK4, Zero Chromium)
//! Official BijiOS Visual Identity + macOS Liquid Glass Edition (International English UI)

pub mod activity;
pub mod apps;
pub mod gtk4_ffi;
pub mod pancer;
pub mod settings;
pub mod sys_hw;
pub mod theme;

use gtk4_ffi::*;
use std::cell::RefCell;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::process::{Command, Stdio};
use std::rc::Rc;

fn now_time_str() -> String {
    Command::new("date")
        .env("LC_ALL", "C")
        .arg("+%H:%M")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "00:00".to_string())
}

fn now_date_str() -> String {
    Command::new("date")
        .env("LC_ALL", "C")
        .arg("+%A, %B %d, %Y")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "BijiOS Desktop".to_string())
}

fn now_short_clock() -> String {
    Command::new("date")
        .env("LC_ALL", "C")
        .arg("+%a %b %d  %H:%M")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "00:00".to_string())
}

/// Authenticate user credentials via local BijiOS PAM daemon (127.0.0.1:3000)
pub fn authenticate_via_daemon(username: &str, password: &str) -> bool {
    let try_once = |u: &str, p: &str| -> bool {
        let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
        let addr = format!("127.0.0.1:{}", port);
        let Ok(mut stream) = TcpStream::connect(&addr) else {
            return false;
        };
        let esc = |s: &str| s.replace('\\', "\\\\").replace('"', "\\\"");
        let body = format!(r#"{{"email":"{}","password":"{}"}}"#, esc(u), esc(p));
        let req = format!(
            "POST /api/auth/login HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        if stream.write_all(req.as_bytes()).is_err() {
            return false;
        }
        let mut resp = String::new();
        let _ = stream.read_to_string(&mut resp);
        resp.starts_with("HTTP/1.1 200") && resp.contains("\"success\": true")
    };

    if try_once(username, password) {
        return true;
    }
    let lower_u = username.to_lowercase();
    let lower_p = password.to_lowercase();
    if lower_u != username || lower_p != password {
        if try_once(&lower_u, &lower_p) {
            return true;
        }
    }
    false
}

pub fn trigger_power(action: &str) {
    if action == "logout" {
        let _ = Command::new("sh")
            .args([
                "-c",
                "openbox --exit || loginctl terminate-session \"$XDG_SESSION_ID\" || pkill -TERM -u \"$USER\" openbox",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
        return;
    }
    let cmd = match action {
        "sleep" => "suspend",
        "restart" => "reboot",
        "poweroff" => "poweroff",
        _ => return,
    };
    let _ = Command::new("systemctl")
        .arg(cmd)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
}

use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

// Global fast flags read by native windows for minimize & open animations
static ANIMATE_OPENING_FLAG: AtomicBool = AtomicBool::new(true);
// 0 = genie, 1 = scale
static MINIMIZE_EFFECT_FLAG: AtomicU8 = AtomicU8::new(0);

pub fn get_animate_opening() -> bool {
    ANIMATE_OPENING_FLAG.load(Ordering::Relaxed)
}

pub fn get_minimize_effect() -> &'static str {
    if MINIMIZE_EFFECT_FLAG.load(Ordering::Relaxed) == 1 {
        "scale"
    } else {
        "genie"
    }
}

#[derive(Clone)]
pub struct DesktopSettings {
    pub dock_size: i32,
    pub dock_hidden: bool,
    pub magnification_enabled: bool,
    pub magnification: f64,
    pub dock_position: String,   // "bottom", "left", "right"
    pub minimize_effect: String, // "genie", "scale"
    pub animate_opening: bool,
    pub wallpaper: String,       // "dynamic-padi" or "<name>.svg"
    pub dynamic_phase: String,   // "auto", "dawn", "day", "sunset", "night"
    pub user_avatar: String,     // "initial", "preset:<emoji>:<grad>", or "file:<path>"
}

impl Default for DesktopSettings {
    fn default() -> Self {
        Self {
            dock_size: 40,
            dock_hidden: false,
            magnification_enabled: true,
            magnification: 1.25,
            dock_position: "bottom".to_string(),
            minimize_effect: "genie".to_string(),
            animate_opening: true,
            wallpaper: "dynamic-padi".to_string(),
            dynamic_phase: "auto".to_string(),
            user_avatar: "initial".to_string(),
        }
    }
}

pub fn current_hour() -> u32 {
    Command::new("date")
        .env("LC_ALL", "C")
        .arg("+%H")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| s.trim().parse::<u32>().ok())
        .unwrap_or(12)
}

pub fn resolve_effective_wallpaper_file(s: &DesktopSettings) -> &'static str {
    if s.wallpaper == "dynamic-padi" {
        match s.dynamic_phase.as_str() {
            "dawn" => "coastal-dawn.svg",
            "day" => "padi-terraces.svg",
            "sunset" => "golden-harvest.svg",
            "night" => "hutan-malam.svg",
            _ => {
                let h = current_hour();
                if (5..9).contains(&h) {
                    "coastal-dawn.svg"
                } else if (9..16).contains(&h) {
                    "padi-terraces.svg"
                } else if (16..19).contains(&h) {
                    "golden-harvest.svg"
                } else {
                    "hutan-malam.svg"
                }
            }
        }
    } else {
        match s.wallpaper.as_str() {
            "padi-terraces.svg" => "padi-terraces.svg",
            "golden-harvest.svg" => "golden-harvest.svg",
            "hutan-malam.svg" => "hutan-malam.svg",
            "sequoia-mist.svg" => "sequoia-mist.svg",
            "coastal-dawn.svg" => "coastal-dawn.svg",
            _ => "padi-terraces.svg",
        }
    }
}

impl DesktopSettings {
    fn config_path() -> std::path::PathBuf {
        apps::home_dir()
            .join(".config")
            .join("bijios-desktop")
            .join("settings.conf")
    }

    pub fn load() -> Self {
        let mut s = Self::default();
        if let Ok(content) = fs::read_to_string(Self::config_path()) {
            for line in content.lines() {
                if let Some((k, v)) = line.split_once('=') {
                    let v = v.trim();
                    match k.trim() {
                        "dock_size" => {
                            if let Ok(n) = v.parse::<i32>() {
                                s.dock_size = n.clamp(15, 64);
                            }
                        }
                        "dock_hidden" => s.dock_hidden = v == "true",
                        "magnification_enabled" => s.magnification_enabled = v == "true",
                        "magnification" => {
                            if let Ok(f) = v.parse::<f64>() {
                                s.magnification = f.clamp(1.0, 1.6);
                            }
                        }
                        "dock_position" if matches!(v, "bottom" | "left" | "right") => {
                            s.dock_position = v.to_string();
                        }
                        "minimize_effect" if matches!(v, "genie" | "scale") => {
                            s.minimize_effect = v.to_string();
                        }
                        "animate_opening" => s.animate_opening = v == "true",
                        "wallpaper" if v == "dynamic-padi" || v.ends_with(".svg") => {
                            s.wallpaper = v.to_string();
                        }
                        "dynamic_phase" if matches!(v, "auto" | "dawn" | "day" | "sunset" | "night") => {
                            s.dynamic_phase = v.to_string();
                        }
                        "user_avatar" if !v.is_empty() => {
                            s.user_avatar = v.to_string();
                        }
                        _ => {}
                    }
                }
            }
        }
        s
    }

    pub fn save(&self) {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let body = format!(
            "dock_size={}\ndock_hidden={}\nmagnification_enabled={}\nmagnification={:.2}\ndock_position={}\nminimize_effect={}\nanimate_opening={}\nwallpaper={}\ndynamic_phase={}\nuser_avatar={}\n",
            self.dock_size,
            self.dock_hidden,
            self.magnification_enabled,
            self.magnification,
            self.dock_position,
            self.minimize_effect,
            self.animate_opening,
            self.wallpaper,
            self.dynamic_phase,
            self.user_avatar
        );
        let _ = fs::write(path, body);
    }
}

pub struct DesktopHandles {
    pub desktop_win: *mut GtkWidget,
    pub root_vbox: *mut GtkWidget,
    pub body_hbox: *mut GtkWidget,
    pub dock_wrap: *mut GtkWidget,
    pub dock_box: *mut GtkWidget,
    pub menubar_avatar_slot: *mut GtkWidget,
    pub login_avatar_slot: Rc<RefCell<*mut GtkWidget>>,
    pub active_user: Rc<RefCell<String>>,
    pub dynamic_css: *mut GtkCssProvider,
    pub settings: Rc<RefCell<DesktopSettings>>,
}

pub unsafe fn refresh_user_avatars(handles: &DesktopHandles) {
    let av = handles.settings.borrow().user_avatar.clone();
    let user = handles.active_user.borrow().clone();

    if !handles.menubar_avatar_slot.is_null() {
        clear_box_children(handles.menubar_avatar_slot);
        gtk_box_append(
            handles.menubar_avatar_slot,
            new_user_avatar_widget(&av, &user, 18),
        );
    }

    let login_slot = *handles.login_avatar_slot.borrow();
    if !login_slot.is_null() {
        clear_box_children(login_slot);
        if av == "initial" {
            let logo_img = new_logo_image(54, false);
            gtk_widget_set_halign(logo_img, GTK_ALIGN_CENTER);
            gtk_widget_set_valign(logo_img, GTK_ALIGN_CENTER);
            gtk_widget_set_hexpand(logo_img, 1);
            gtk_box_append(login_slot, logo_img);
        } else {
            let av_w = new_user_avatar_widget(&av, &user, 76);
            gtk_widget_set_halign(av_w, GTK_ALIGN_CENTER);
            gtk_widget_set_valign(av_w, GTK_ALIGN_CENTER);
            gtk_widget_set_hexpand(av_w, 1);
            gtk_box_append(login_slot, av_w);
        }
    }
}

pub unsafe fn apply_desktop_settings(handles: &DesktopHandles) {
    let s = handles.settings.borrow().clone();
    s.save();

    ANIMATE_OPENING_FLAG.store(s.animate_opening, Ordering::Relaxed);
    MINIMIZE_EFFECT_FLAG.store(
        if s.minimize_effect == "scale" { 1 } else { 0 },
        Ordering::Relaxed,
    );

    // 1. Resolve Nature / Dynamic SVG Wallpaper CSS rule
    let wp_file = resolve_effective_wallpaper_file(&s);
    let wp_css = if let Some(wp_path) = resolve_wallpaper_path(wp_file) {
        format!(
            "window.bijios-desktop-bg {{ background-image: url(\"file://{}\"); background-size: cover; background-position: center; }}",
            wp_path
        )
    } else {
        String::new()
    };

    // 2. Apply Dock Size (down to 15px) & Magnification via live user CSS provider
    let sz = s.dock_size.clamp(15, 64);
    let font_sz = ((sz as f64) * 0.52).round().max(8.0) as i32;
    let radius = ((sz as f64) * 0.28).round().max(4.0) as i32;
    let dock_pad = if sz <= 20 { 3 } else if sz <= 28 { 5 } else { 7 };
    let dock_rad = (radius + 6).min(18);
    let hover_rule = if s.magnification_enabled && s.magnification > 1.01 {
        let (tx, ty) = match s.dock_position.as_str() {
            "left" => (6, 0),
            "right" => (-6, 0),
            _ => (0, -6),
        };
        format!(
            "button.dock-btn:hover {{ transform: translate({}px, {}px) scale({:.2}); }}",
            tx, ty, s.magnification
        )
    } else {
        "button.dock-btn:hover { transform: none; }".to_string()
    };

    let bounce_rule = match s.dock_position.as_str() {
        "left" => "button.dock-btn.dock-bouncing { transform: translateX(10px) scale(1.12); }",
        "right" => "button.dock-btn.dock-bouncing { transform: translateX(-10px) scale(1.12); }",
        _ => "button.dock-btn.dock-bouncing { transform: translateY(-10px) scale(1.12); }",
    };

    let css_str = format!(
        "{}\n.floating-dock {{ padding: {}px; border-radius: {}px; }}\nbutton.dock-btn {{ min-width: {}px; min-height: {}px; padding: 0px; font-size: {}px; border-radius: {}px; transition: all 140ms ease; }}\n{}\n{}\n",
        wp_css, dock_pad, dock_rad, sz, sz, font_sz, radius, hover_rule, bounce_rule
    );
    let c_css = cstr(&css_str);
    gtk_css_provider_load_from_string(handles.dynamic_css, c_css.as_ptr());

    // 3. Apply Dock Visibility (Hide / Unhide) & Position (Bottom / Left / Right) live
    gtk_widget_set_visible(handles.dock_wrap, if s.dock_hidden { 0 } else { 1 });
    let parent = gtk_widget_get_parent(handles.dock_wrap);
    g_object_ref(handles.dock_wrap as GPointer);
    if parent == handles.root_vbox {
        gtk_box_remove(handles.root_vbox, handles.dock_wrap);
    } else if parent == handles.body_hbox {
        gtk_box_remove(handles.body_hbox, handles.dock_wrap);
    }

    match s.dock_position.as_str() {
        "left" => {
            gtk_orientable_set_orientation(handles.dock_box, GTK_ORIENTATION_VERTICAL);
            gtk_widget_set_halign(handles.dock_wrap, GTK_ALIGN_START);
            gtk_widget_set_valign(handles.dock_wrap, GTK_ALIGN_CENTER);
            set_margins(handles.dock_wrap, 0, 0, 8, 0);
            gtk_box_prepend(handles.body_hbox, handles.dock_wrap);
        }
        "right" => {
            gtk_orientable_set_orientation(handles.dock_box, GTK_ORIENTATION_VERTICAL);
            gtk_widget_set_halign(handles.dock_wrap, GTK_ALIGN_END);
            gtk_widget_set_valign(handles.dock_wrap, GTK_ALIGN_CENTER);
            set_margins(handles.dock_wrap, 0, 0, 0, 8);
            gtk_box_append(handles.body_hbox, handles.dock_wrap);
        }
        _ => {
            gtk_orientable_set_orientation(handles.dock_box, GTK_ORIENTATION_HORIZONTAL);
            gtk_widget_set_halign(handles.dock_wrap, GTK_ALIGN_CENTER);
            gtk_widget_set_valign(handles.dock_wrap, GTK_ALIGN_END);
            set_margins(handles.dock_wrap, 0, 8, 0, 0);
            gtk_box_append(handles.root_vbox, handles.dock_wrap);
        }
    }
    g_object_unref(handles.dock_wrap as GPointer);

    // 4. Refresh User Profile Avatars across Top Menu Bar & Login Screen
    refresh_user_avatars(handles);
}

unsafe fn load_bijios_css() -> *mut GtkCssProvider {
    let display = gdk_display_get_default();
    let dynamic_provider = gtk_css_provider_new();
    if display.is_null() {
        return dynamic_provider;
    }
    let provider = gtk_css_provider_new();
    let css = cstr(theme::BIJIOS_CSS);
    gtk_css_provider_load_from_string(provider, css.as_ptr());
    gtk_style_context_add_provider_for_display(
        display,
        provider,
        GTK_STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
    gtk_style_context_add_provider_for_display(
        display,
        dynamic_provider,
        GTK_STYLE_PROVIDER_PRIORITY_USER,
    );
    dynamic_provider
}

unsafe fn build_desktop_shell(
    app: *mut GtkApplication,
    active_user: Rc<RefCell<String>>,
    login_win_holder: Rc<RefCell<*mut GtkWidget>>,
    login_avatar_slot: Rc<RefCell<*mut GtkWidget>>,
    dynamic_css: *mut GtkCssProvider,
) -> *mut GtkWidget {
    let win = gtk_application_window_new(app);
    let title = cstr("BijiOS Desktop");
    gtk_window_set_title(win, title.as_ptr());
    gtk_window_set_decorated(win, 0);
    add_css(win, "bijios-desktop-bg");

    // Set official X11 _NET_WM_WINDOW_TYPE_DESKTOP + full screen (0,0 -> WxH) on realize/map
    // so Top Menu Bar & Floating Dock always show at screen edges while maximized apps respect <margins>
    connect_signal(win, "realize", move || {
        set_x11_desktop_window(win);
    });
    connect_signal(win, "map", move || {
        set_x11_desktop_window(win);
        timeout_add_ms(120, move || {
            set_x11_desktop_window(win);
            false
        });
    });

    let overlay = gtk_overlay_new();
    let root = new_box(GTK_ORIENTATION_VERTICAL, 0, &[]);
    gtk_widget_set_hexpand(root, 1);
    gtk_widget_set_vexpand(root, 1);
    gtk_overlay_set_child(overlay, root);

    // Body horizontal container (holds workspace + optional Left/Right Dock)
    let body_hbox = new_box(GTK_ORIENTATION_HORIZONTAL, 0, &[]);
    gtk_widget_set_hexpand(body_hbox, 1);
    gtk_widget_set_vexpand(body_hbox, 1);

    // Dock container & inner dock box (re-parented live by Dock Position setting)
    let dock_wrap = new_box(GTK_ORIENTATION_HORIZONTAL, 0, &[]);
    let dock = new_box(GTK_ORIENTATION_HORIZONTAL, 6, &["bijios-dock"]);
    gtk_box_append(dock_wrap, dock);

    // Top Menu Bar live user avatar slot
    let menubar_avatar_slot = new_box(GTK_ORIENTATION_HORIZONTAL, 0, &[]);
    gtk_widget_set_valign(menubar_avatar_slot, GTK_ALIGN_CENTER);

    let handles = Rc::new(DesktopHandles {
        desktop_win: win,
        root_vbox: root,
        body_hbox,
        dock_wrap,
        dock_box: dock,
        menubar_avatar_slot,
        login_avatar_slot,
        active_user: Rc::clone(&active_user),
        dynamic_css,
        settings: Rc::new(RefCell::new(DesktopSettings::load())),
    });

    let on_lock: Rc<dyn Fn()> = {
        let login_holder = Rc::clone(&login_win_holder);
        Rc::new(move || {
            let lw = *login_holder.borrow();
            if !lw.is_null() {
                // Hide desktop shell so Top Menu Bar, widgets, and Dock cannot be accessed while locked
                gtk_widget_set_visible(win, 0);
                gtk_widget_set_visible(lw, 1);
                gtk_window_fullscreen(lw);
                set_x11_lock_window(lw);
                gtk_window_present(lw);
                timeout_add_ms(80, move || {
                    if gtk_widget_get_visible(lw) != 0 {
                        set_x11_lock_window(lw);
                        gtk_window_present(lw);
                    }
                    false
                });
            }
        })
    };

    // =========================================================================
    // 1. Top Menu Bar + Official BijiOS Logo + Center Dynamic Island
    // =========================================================================
    let menubar = gtk_center_box_new();
    add_css(menubar, "bijios-menubar");

    let left_box = new_box(GTK_ORIENTATION_HORIZONTAL, 4, &[]);

    // BijiOS Brand Button (toggles in-window GtkOverlay dropdown — zero black X11 popup!)
    let bijios_menu_btn = gtk_button_new();
    let brand_row = new_box(GTK_ORIENTATION_HORIZONTAL, 5, &[]);
    let brand_logo = new_logo_image(15, false);
    let brand_lbl = new_label("BijiOS", &[]);
    gtk_box_append(brand_row, brand_logo);
    gtk_box_append(brand_row, brand_lbl);
    gtk_button_set_child(bijios_menu_btn, brand_row);

    // Floating in-window Liquid Glass Dropdown Menu (painted directly on desktop surface)
    let menu_panel = new_box(GTK_ORIENTATION_VERTICAL, 3, &["bijios-dropdown-menu"]);
    gtk_widget_set_halign(menu_panel, GTK_ALIGN_START);
    gtk_widget_set_valign(menu_panel, GTK_ALIGN_START);
    set_margins(menu_panel, 32, 0, 10, 0);
    gtk_widget_set_size_request(menu_panel, 215, -1);
    gtk_widget_set_visible(menu_panel, 0);

    let m_about = new_left_button("🌱", "About BijiOS", &["sidebar-btn"]);
    let m_activity = new_left_button("📈", "Activity Monitor...", &["sidebar-btn"]);
    let m_settings = new_left_button("⚙️", "System Settings...", &["sidebar-btn"]);
    let m_lock = new_left_button("🔒", "Lock Screen", &["sidebar-btn"]);
    let m_logout = new_left_button("🚪", "Log Out...", &["sidebar-btn"]);
    let m_sleep = new_left_button("💤", "Sleep", &["sidebar-btn"]);
    let m_restart = new_left_button("🔄", "Restart...", &["sidebar-btn"]);
    let m_shutdown = new_left_button("⏻", "Shut Down...", &["danger-btn"]);

    gtk_box_append(menu_panel, m_about);
    gtk_box_append(menu_panel, m_activity);
    gtk_box_append(menu_panel, m_settings);
    gtk_box_append(menu_panel, gtk_separator_new(GTK_ORIENTATION_HORIZONTAL));
    gtk_box_append(menu_panel, m_lock);
    gtk_box_append(menu_panel, m_logout);
    gtk_box_append(menu_panel, m_sleep);
    gtk_box_append(menu_panel, m_restart);
    gtk_box_append(menu_panel, m_shutdown);
    gtk_overlay_add_overlay(overlay, menu_panel);

    connect_signal(bijios_menu_btn, "clicked", move || {
        let vis = gtk_widget_get_visible(menu_panel) != 0;
        gtk_widget_set_visible(menu_panel, if vis { 0 } else { 1 });
    });

    connect_signal(m_about, "clicked", move || {
        gtk_widget_set_visible(menu_panel, 0);
        apps::open_about(app);
    });
    connect_signal(m_activity, "clicked", move || {
        gtk_widget_set_visible(menu_panel, 0);
        apps::open_activity_monitor(app);
    });
    {
        let ol = Rc::clone(&on_lock);
        let h = Rc::clone(&handles);
        connect_signal(m_settings, "clicked", move || {
            gtk_widget_set_visible(menu_panel, 0);
            apps::open_settings(app, Rc::clone(&h), Rc::clone(&ol));
        });
    }
    {
        let ol = Rc::clone(&on_lock);
        connect_signal(m_lock, "clicked", move || {
            gtk_widget_set_visible(menu_panel, 0);
            ol();
        });
    }
    connect_signal(m_logout, "clicked", move || {
        gtk_widget_set_visible(menu_panel, 0);
        trigger_power("logout");
    });
    connect_signal(m_sleep, "clicked", move || {
        gtk_widget_set_visible(menu_panel, 0);
        trigger_power("sleep");
    });
    connect_signal(m_restart, "clicked", move || {
        gtk_widget_set_visible(menu_panel, 0);
        trigger_power("restart");
    });
    connect_signal(m_shutdown, "clicked", move || {
        gtk_widget_set_visible(menu_panel, 0);
        trigger_power("poweroff");
    });

    let btn_finder_top = new_button("Akar", &[]);
    let tip_akar = cstr("Akar - File Manager");
    gtk_widget_set_tooltip_text(btn_finder_top, tip_akar.as_ptr());

    let btn_launch_top = new_button("Semai", &[]);
    let tip_semai = cstr("Semai - App Launcher");
    gtk_widget_set_tooltip_text(btn_launch_top, tip_semai.as_ptr());

    let btn_bibit_top = new_button("Bibit", &[]);
    let tip_bibit = cstr("Bibit - Debian App Store");
    gtk_widget_set_tooltip_text(btn_bibit_top, tip_bibit.as_ptr());

    let btn_pancer_top = new_button("Pancer", &[]);
    let tip_pancer = cstr("Pancer - Native Terminal");
    gtk_widget_set_tooltip_text(btn_pancer_top, tip_pancer.as_ptr());

    let btn_settings_top = new_button("Settings", &[]);

    connect_signal(btn_finder_top, "clicked", move || apps::open_finder(app));
    {
        let ol = Rc::clone(&on_lock);
        let h = Rc::clone(&handles);
        connect_signal(btn_launch_top, "clicked", move || {
            apps::open_launchpad(app, Rc::clone(&h), Rc::clone(&ol));
        });
    }
    connect_signal(btn_bibit_top, "clicked", move || apps::open_bibit(app));
    connect_signal(btn_pancer_top, "clicked", move || apps::open_pancer(app));
    {
        let ol = Rc::clone(&on_lock);
        let h = Rc::clone(&handles);
        connect_signal(btn_settings_top, "clicked", move || {
            apps::open_settings(app, Rc::clone(&h), Rc::clone(&ol));
        });
    }

    gtk_box_append(left_box, bijios_menu_btn);
    gtk_box_append(left_box, btn_finder_top);
    gtk_box_append(left_box, btn_launch_top);
    gtk_box_append(left_box, btn_bibit_top);
    gtk_box_append(left_box, btn_pancer_top);
    gtk_box_append(left_box, btn_settings_top);

    // Right status indicators (with live User Profile Avatar pill)
    let right_box = new_box(GTK_ORIENTATION_HORIZONTAL, 8, &[]);
    let hostname = fs::read_to_string("/etc/hostname").unwrap_or_else(|_| "BijiOS".into());
    let host_pill = new_label(&format!("🖥️ {}", hostname.trim()), &["menubar-pill"]);
    let user_pill = new_box(GTK_ORIENTATION_HORIZONTAL, 6, &["menubar-pill"]);
    let user_name_lbl = new_label(&active_user.borrow(), &[]);
    gtk_box_append(user_pill, menubar_avatar_slot);
    gtk_box_append(user_pill, user_name_lbl);
    let top_clock = new_label(&now_short_clock(), &["menubar-clock"]);
    gtk_box_append(right_box, host_pill);
    gtk_box_append(right_box, user_pill);
    gtk_box_append(right_box, top_clock);

    gtk_center_box_set_start_widget(menubar, left_box);
    gtk_center_box_set_end_widget(menubar, right_box);
    gtk_box_append(root, menubar);

    // =========================================================================
    // 2. Clean 2-Column Liquid Glass Desktop Widgets (Compact & Airy)
    // =========================================================================
    let workspace = new_box(GTK_ORIENTATION_HORIZONTAL, 18, &[]);
    gtk_widget_set_vexpand(workspace, 1);
    gtk_widget_set_hexpand(workspace, 1);
    gtk_widget_set_valign(workspace, GTK_ALIGN_START);
    set_margins(workspace, 18, 12, 22, 22);

    // Left Widget Column
    let left_col = new_box(GTK_ORIENTATION_VERTICAL, 14, &[]);
    gtk_widget_set_halign(left_col, GTK_ALIGN_START);

    // Widget 1: Clock & Date Liquid Glass Card
    let clock_card = new_box(GTK_ORIENTATION_VERTICAL, 3, &["glass-card"]);
    gtk_widget_set_size_request(clock_card, 265, -1);
    let hdr_row = new_box(GTK_ORIENTATION_HORIZONTAL, 6, &[]);
    gtk_box_append(hdr_row, new_logo_image(14, false));
    let w_title = new_label("BIJIOS 1.0.0 \"PADI\"", &["widget-title"]);
    gtk_box_append(hdr_row, w_title);
    let w_clock = new_label(&now_time_str(), &["widget-clock-big"]);
    gtk_label_set_xalign(w_clock, 0.0);
    let w_date = new_label(&now_date_str(), &["widget-sub"]);
    gtk_label_set_xalign(w_date, 0.0);
    gtk_box_append(clock_card, hdr_row);
    gtk_box_append(clock_card, w_clock);
    gtk_box_append(clock_card, w_date);
    gtk_box_append(left_col, clock_card);

    // Widget 2: Live System Telemetry Liquid Glass Card
    let sys_card = new_box(GTK_ORIENTATION_VERTICAL, 6, &["glass-card"]);
    gtk_widget_set_size_request(sys_card, 265, -1);
    let sys_title = new_label("⚡ SYSTEM RESOURCES", &["widget-title"]);
    gtk_label_set_xalign(sys_title, 0.0);
    let (used_mb, total_mb, pct) = apps::read_mem_usage();
    let w_mem = new_label(
        &format!(
            "RAM: {} / {} MB ({:.0}%)  •  Load: {}",
            used_mb,
            total_mb,
            pct * 100.0,
            apps::read_load_avg()
        ),
        &["widget-sub"],
    );
    gtk_label_set_xalign(w_mem, 0.0);
    let w_bar = gtk_progress_bar_new();
    gtk_progress_bar_set_fraction(w_bar, pct);
    gtk_box_append(sys_card, sys_title);
    gtk_box_append(sys_card, w_mem);
    gtk_box_append(sys_card, w_bar);
    gtk_box_append(left_col, sys_card);

    let spacer = new_box(GTK_ORIENTATION_HORIZONTAL, 0, &[]);
    gtk_widget_set_hexpand(spacer, 1);

    // Right Widget Column
    let right_col = new_box(GTK_ORIENTATION_VERTICAL, 14, &[]);
    gtk_widget_set_halign(right_col, GTK_ALIGN_END);

    // Widget 3: BijiOS Atmosphere Card
    let weather_card = new_box(GTK_ORIENTATION_VERTICAL, 4, &["glass-card"]);
    gtk_widget_set_size_request(weather_card, 245, -1);
    let wt_hdr = new_label("☀️ WORKSPACE STATUS", &["widget-title"]);
    gtk_label_set_xalign(wt_hdr, 0.0);
    let wt_temp = new_label("26°C  Clear Sky", &["widget-temp-big"]);
    gtk_label_set_xalign(wt_temp, 0.0);
    let wt_sub = new_label("Pure Rust + GTK4 • Liquid Glass", &["widget-sub"]);
    gtk_label_set_xalign(wt_sub, 0.0);
    gtk_box_append(weather_card, wt_hdr);
    gtk_box_append(weather_card, wt_temp);
    gtk_box_append(weather_card, wt_sub);
    gtk_box_append(right_col, weather_card);

    // Widget 4: Emas Benih Glass Quick Sticky Note
    let note_card = new_box(GTK_ORIENTATION_VERTICAL, 6, &["sticky-note-card"]);
    gtk_widget_set_size_request(note_card, 245, 125);
    let note_hdr = new_label("🗒️ QUICK STICKY NOTE", &["widget-title-amber"]);
    gtk_label_set_xalign(note_hdr, 0.0);
    let note_tv = gtk_text_view_new();
    gtk_text_view_set_wrap_mode(note_tv, GTK_WRAP_WORD_CHAR);
    gtk_widget_set_vexpand(note_tv, 1);
    add_css(note_tv, "sticky-note-view");
    set_text_view_text(
        note_tv,
        "BijiOS 1.0.0 \"Padi\"\n• Semai: App Launcher\n• Bibit: Debian App Store",
    );
    gtk_box_append(note_card, note_hdr);
    gtk_box_append(note_card, note_tv);
    gtk_box_append(right_col, note_card);

    gtk_box_append(workspace, left_col);
    gtk_box_append(workspace, spacer);
    gtk_box_append(workspace, right_col);
    gtk_box_append(body_hbox, workspace);
    gtk_box_append(root, body_hbox);

    // =========================================================================
    // 3. Liquid Glass Floating Dock (Live Size, Magnification, Position & Bounce)
    // =========================================================================
    let dock_items: [(&str, &str, &str, &str); 14] = [
        ("finder", "📁", "Akar - File Manager", "icon-finder"),
        ("launchpad", "🚀", "Semai - App Launcher", "icon-launchpad"),
        ("bibit", "🛍️", "Bibit - Debian App Store", "icon-bibit"),
        ("pancer", "💻", "Pancer - Terminal", "icon-pancer"),
        ("wordcraft", "📝", "WordCraft (Storytold — Apache-2.0)", "icon-wordcraft"),
        ("gridcraft", "📊", "GridCraft (Storytold — Apache-2.0)", "icon-gridcraft"),
        ("deckcraft", "📽️", "DeckCraft (Storytold — Apache-2.0)", "icon-deckcraft"),
        ("pdfcraft", "📕", "PDFCraft (Storytold — Apache-2.0)", "icon-pdfcraft"),
        ("notes", "🗒️", "BijiOS Notes", "icon-notes"),
        ("calculator", "🧮", "Calculator", "icon-calculator"),
        ("activity", "📈", "Activity Monitor", "icon-activity"),
        ("settings", "⚙️", "System Settings", "icon-settings"),
        ("about", "", "About BijiOS 1.0.0 (Padi)", "icon-about"),
        ("lock", "🔒", "Lock Screen", "icon-lock"),
    ];

    for (id, icon, tip, icon_cls) in dock_items {
        let btn = if id == "about" {
            let b = gtk_button_new();
            add_css(b, "dock-btn");
            add_css(b, icon_cls);
            gtk_button_set_child(b, new_logo_image(24, false));
            b
        } else {
            new_button(icon, &["dock-btn", icon_cls])
        };
        let t = cstr(tip);
        gtk_widget_set_tooltip_text(btn, t.as_ptr());
        gtk_box_append(dock, btn);

        let id_s = id.to_string();
        let ol = Rc::clone(&on_lock);
        let h = Rc::clone(&handles);
        connect_signal(btn, "clicked", move || {
            // Animate opening applications (Dock icon bounce)
            if get_animate_opening() {
                add_css(btn, "dock-bouncing");
                timeout_add_ms(260, move || {
                    remove_css(btn, "dock-bouncing");
                    false
                });
            }
            match id_s.as_str() {
                "finder" => apps::open_finder(app),
                "launchpad" => apps::open_launchpad(app, Rc::clone(&h), Rc::clone(&ol)),
                "bibit" => apps::open_bibit(app),
                "pancer" => apps::open_pancer(app),
                "wordcraft" => apps::open_wordcraft(app, None),
                "gridcraft" => apps::open_gridcraft(app),
                "deckcraft" => apps::open_deckcraft(None),
                "pdfcraft" => apps::open_pdfcraft(None),
                "notes" => apps::open_notes(app),
                "calculator" => apps::open_calculator(app),
                "activity" => apps::open_activity_monitor(app),
                "settings" => apps::open_settings(app, Rc::clone(&h), Rc::clone(&ol)),
                "about" => apps::open_about(app),
                "lock" => ol(),
                _ => {}
            }
        });
    }

    // Apply persisted Dock Size, Magnification, Position, Wallpaper & Animation settings
    apply_desktop_settings(&handles);

    // Automatically Hide & Show the Dock when pointer approaches the Dock screen edge
    {
        let h_auto = Rc::clone(&handles);
        let revealed = Rc::new(std::cell::Cell::new(false));
        let leave_ticks = Rc::new(std::cell::Cell::new(0u8));
        timeout_add_ms(110, move || {
            let (auto_hide, pos, sz) = {
                let s = h_auto.settings.borrow();
                (s.dock_hidden, s.dock_position.clone(), s.dock_size)
            };
            if !auto_hide {
                if revealed.get() {
                    revealed.set(false);
                }
                leave_ticks.set(0);
                if gtk_widget_get_visible(h_auto.dock_wrap) == 0 {
                    gtk_widget_set_visible(h_auto.dock_wrap, 1);
                }
                return true;
            }
            if gtk_widget_get_visible(win) == 0 {
                return true;
            }
            let (sw, sh) = get_x11_screen_size();
            let Some((px, py)) = get_x11_pointer_pos() else {
                return true;
            };
            if sw <= 0 || sh <= 0 {
                return true;
            }
            let edge_px = 14;
            let keep_px = (sz + 46).max(64);
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
                        gtk_widget_set_visible(h_auto.dock_wrap, 0);
                    }
                }
            } else if near_edge {
                revealed.set(true);
                leave_ticks.set(0);
                gtk_widget_set_visible(h_auto.dock_wrap, 1);
            }
            true
        });
    }

    let active_user_t = Rc::clone(&active_user);
    let login_holder_t = Rc::clone(&login_win_holder);
    let last_screen = Rc::new(std::cell::Cell::new((0i32, 0i32)));
    timeout_add_sec(2, move || {
        set_label_text(top_clock, &now_short_clock());
        set_label_text(w_clock, &now_time_str());
        set_label_text(w_date, &now_date_str());
        set_label_text(user_name_lbl, &active_user_t.borrow());
        let (u, t, p) = apps::read_mem_usage();
        set_label_text(
            w_mem,
            &format!(
                "RAM: {} / {} MB ({:.0}%)  •  Load: {}",
                u,
                t,
                p * 100.0,
                apps::read_load_avg()
            ),
        );
        gtk_progress_bar_set_fraction(w_bar, p);

        // Auto-adapt Desktop & Lock Screen when VNC / xrandr dynamically resizes the screen
        let cur_sz = get_x11_screen_size();
        if cur_sz.0 > 0 && cur_sz.1 > 0 && cur_sz != last_screen.get() {
            last_screen.set(cur_sz);
            if gtk_widget_get_visible(win) != 0 {
                set_x11_desktop_window(win);
            }
            let lw = *login_holder_t.borrow();
            if !lw.is_null() && gtk_widget_get_visible(lw) != 0 {
                set_x11_lock_window(lw);
            }
        }
        true
    });

    gtk_window_set_child(win, overlay);
    win
}

unsafe fn build_login_screen(
    app: *mut GtkApplication,
    desktop_win: *mut GtkWidget,
    active_user: Rc<RefCell<String>>,
    login_avatar_slot: Rc<RefCell<*mut GtkWidget>>,
) -> *mut GtkWidget {
    let win = gtk_application_window_new(app);
    let title = cstr("BijiOS Login");
    gtk_window_set_title(win, title.as_ptr());
    gtk_window_set_decorated(win, 0);
    gtk_window_set_modal(win, 1);
    gtk_window_fullscreen(win);
    add_css(win, "bijios-login-window");

    // Always force true 100% fullscreen + ABOVE layer on realize/map so desktop & apps cannot be clicked
    connect_signal(win, "realize", move || {
        set_x11_lock_window(win);
    });
    connect_signal(win, "map", move || {
        set_x11_lock_window(win);
        timeout_add_ms(100, move || {
            if gtk_widget_get_visible(win) != 0 {
                set_x11_lock_window(win);
            }
            false
        });
    });

    let outer = new_box(GTK_ORIENTATION_VERTICAL, 0, &[]);
    gtk_widget_set_hexpand(outer, 1);
    gtk_widget_set_vexpand(outer, 1);
    set_margins(outer, 24, 28, 32, 32);

    // Top Liquid Glass Status Bar with official BijiOS Logo
    let top_bar = gtk_center_box_new();
    let left_pill = new_box(GTK_ORIENTATION_HORIZONTAL, 8, &["tahoe-glass-pill"]);
    gtk_box_append(left_pill, new_logo_image(18, false));
    gtk_box_append(left_pill, new_label("BijiOS 1.0.0 \"Padi\"  •  Liquid Glass", &[]));
    let right_pill = new_label("📶 Wi-Fi Connected  •  🔋 100%  •  EN", &["tahoe-glass-pill"]);
    gtk_center_box_set_start_widget(top_bar, left_pill);
    gtk_center_box_set_end_widget(top_bar, right_pill);
    gtk_box_append(outer, top_bar);

    // Center Stack: Clock + Date + Liquid Glass Card
    let center = new_box(GTK_ORIENTATION_VERTICAL, 18, &[]);
    gtk_widget_set_halign(center, GTK_ALIGN_CENTER);
    gtk_widget_set_valign(center, GTK_ALIGN_CENTER);
    gtk_widget_set_vexpand(center, 1);

    let clock_box = new_box(GTK_ORIENTATION_VERTICAL, 2, &[]);
    let clock_lbl = new_label(&now_time_str(), &["login-clock"]);
    let date_lbl = new_label(&now_date_str(), &["login-date"]);
    gtk_box_append(clock_box, clock_lbl);
    gtk_box_append(clock_box, date_lbl);
    gtk_box_append(center, clock_box);

    // Central Liquid Glass Login Card
    let card = new_box(GTK_ORIENTATION_VERTICAL, 12, &["login-card"]);
    gtk_widget_set_size_request(card, 400, -1);

    let avatar_ring = new_box(GTK_ORIENTATION_HORIZONTAL, 0, &["login-avatar-ring"]);
    gtk_widget_set_halign(avatar_ring, GTK_ALIGN_CENTER);
    *login_avatar_slot.borrow_mut() = avatar_ring;
    let init_settings = DesktopSettings::load();
    if init_settings.user_avatar == "initial" {
        let logo_img = new_logo_image(54, false);
        gtk_widget_set_halign(logo_img, GTK_ALIGN_CENTER);
        gtk_widget_set_valign(logo_img, GTK_ALIGN_CENTER);
        gtk_widget_set_hexpand(logo_img, 1);
        gtk_box_append(avatar_ring, logo_img);
    } else {
        let av_w = new_user_avatar_widget(&init_settings.user_avatar, &active_user.borrow(), 76);
        gtk_widget_set_halign(av_w, GTK_ALIGN_CENTER);
        gtk_widget_set_valign(av_w, GTK_ALIGN_CENTER);
        gtk_widget_set_hexpand(av_w, 1);
        gtk_box_append(avatar_ring, av_w);
    }

    let title_lbl = new_label("Sign In to BijiOS", &["login-title"]);
    let badge_lbl = new_label("Version 1.0.0 (Padi)  •  Sovereign Workspace", &["login-badge"]);
    gtk_widget_set_halign(badge_lbl, GTK_ALIGN_CENTER);

    let user_entry = gtk_entry_new();
    add_css(user_entry, "login-entry");
    let uph = cstr("Username");
    gtk_entry_set_placeholder_text(user_entry, uph.as_ptr());
    let default_user = std::env::var("USER").unwrap_or_else(|_| "dionipe".to_string());
    set_entry_text(user_entry, &default_user);

    let pass_row = new_box(GTK_ORIENTATION_HORIZONTAL, 10, &[]);
    let pass_entry = gtk_entry_new();
    gtk_widget_set_hexpand(pass_entry, 1);
    add_css(pass_entry, "login-entry");
    let pph = cstr("Enter Password");
    gtk_entry_set_placeholder_text(pass_entry, pph.as_ptr());
    gtk_entry_set_visibility(pass_entry, 0);

    let login_btn = new_button("➔", &["login-submit-btn"]);
    let tip = cstr("Unlock BijiOS Desktop");
    gtk_widget_set_tooltip_text(login_btn, tip.as_ptr());

    gtk_box_append(pass_row, pass_entry);
    gtk_box_append(pass_row, login_btn);

    let err_lbl = new_label("", &["login-error"]);
    gtk_widget_set_visible(err_lbl, 0);

    let hint_lbl = new_label("Press Enter ↵ to unlock session", &["login-subtitle"]);

    gtk_box_append(card, avatar_ring);
    gtk_box_append(card, title_lbl);
    gtk_box_append(card, badge_lbl);
    gtk_box_append(card, user_entry);
    gtk_box_append(card, pass_row);
    gtk_box_append(card, err_lbl);
    gtk_box_append(card, hint_lbl);
    gtk_box_append(center, card);

    gtk_box_append(outer, center);

    // Bottom Circular Liquid Glass Power Actions
    let power_row = new_box(GTK_ORIENTATION_HORIZONTAL, 28, &[]);
    gtk_widget_set_halign(power_row, GTK_ALIGN_CENTER);

    let make_power_action = |icon: &str, caption: &str, action: &'static str| -> *mut GtkWidget {
        let col = new_box(GTK_ORIENTATION_VERTICAL, 6, &[]);
        gtk_widget_set_halign(col, GTK_ALIGN_CENTER);
        let btn = new_button(icon, &["login-power-circle"]);
        gtk_widget_set_halign(btn, GTK_ALIGN_CENTER);
        let cap = new_label(caption, &["login-power-caption"]);
        gtk_box_append(col, btn);
        gtk_box_append(col, cap);
        connect_signal(btn, "clicked", move || trigger_power(action));
        col
    };

    gtk_box_append(power_row, make_power_action("🚪", "Log Out", "logout"));
    gtk_box_append(power_row, make_power_action("💤", "Sleep", "sleep"));
    gtk_box_append(power_row, make_power_action("🔄", "Restart", "restart"));
    gtk_box_append(power_row, make_power_action("⏻", "Shut Down", "poweroff"));
    gtk_box_append(outer, power_row);

    let do_login: Rc<dyn Fn()> = {
        let au = Rc::clone(&active_user);
        Rc::new(move || {
            let u = get_entry_text(user_entry).trim().to_string();
            let p = get_entry_text(pass_entry).trim().to_string();
            if u.is_empty() || p.is_empty() {
                set_label_text(err_lbl, "Please enter your username and password.");
                gtk_widget_set_visible(err_lbl, 1);
                return;
            }
            if authenticate_via_daemon(&u, &p) {
                *au.borrow_mut() = u.to_lowercase();
                set_entry_text(pass_entry, "");
                gtk_widget_set_visible(err_lbl, 0);
                gtk_widget_set_visible(win, 0);
                gtk_widget_set_visible(desktop_win, 1);
                set_x11_desktop_window(desktop_win);
                gtk_window_present(desktop_win);
            } else {
                set_label_text(err_lbl, "Invalid username or password. Please try again.");
                gtk_widget_set_visible(err_lbl, 1);
                set_entry_text(pass_entry, "");
                gtk_widget_grab_focus(pass_entry);
            }
        })
    };

    {
        let dl = Rc::clone(&do_login);
        connect_signal(login_btn, "clicked", move || dl());
    }
    {
        let dl = Rc::clone(&do_login);
        connect_signal(pass_entry, "activate", move || dl());
    }
    connect_signal(user_entry, "activate", move || {
        gtk_widget_grab_focus(pass_entry);
    });

    timeout_add_sec(5, move || {
        set_label_text(clock_lbl, &now_time_str());
        set_label_text(date_lbl, &now_date_str());
        true
    });

    gtk_window_set_child(win, outer);
    win
}

extern "C" fn on_activate(app: *mut GtkApplication, _user_data: GPointer) {
    unsafe {
        let dynamic_css = load_bijios_css();

        let default_user = std::env::var("USER").unwrap_or_else(|_| "dionipe".to_string());
        let active_user = Rc::new(RefCell::new(default_user));
        let login_win_holder: Rc<RefCell<*mut GtkWidget>> = Rc::new(RefCell::new(std::ptr::null_mut()));
        let login_avatar_slot: Rc<RefCell<*mut GtkWidget>> = Rc::new(RefCell::new(std::ptr::null_mut()));

        let desktop_win = build_desktop_shell(
            app,
            Rc::clone(&active_user),
            Rc::clone(&login_win_holder),
            Rc::clone(&login_avatar_slot),
            dynamic_css,
        );
        let login_win = build_login_screen(
            app,
            desktop_win,
            Rc::clone(&active_user),
            Rc::clone(&login_avatar_slot),
        );
        *login_win_holder.borrow_mut() = login_win;

        // Start on BijiOS Native Liquid Glass Login Screen (1x PAM login)
        gtk_widget_set_visible(desktop_win, 0);
        gtk_window_present(login_win);
    }
}

pub fn run_native_shell() -> i32 {
    // Force Cairo renderer on X11/VM environments where Vulkan/NGL renders black
    if std::env::var("GSK_RENDERER").is_err() {
        std::env::set_var("GSK_RENDERER", "cairo");
    }
    if std::env::var("GDK_BACKEND").is_err() {
        std::env::set_var("GDK_BACKEND", "x11");
    }

    unsafe {
        let app_id = cstr("id.bijios.desktop.shell");
        let app = gtk_application_new(app_id.as_ptr(), G_APPLICATION_NON_UNIQUE);
        let sig = cstr("activate");
        g_signal_connect_data(
            app as GPointer,
            sig.as_ptr(),
            on_activate as GPointer,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            0,
        );
        let status = g_application_run(app as *mut GApplication, 0, std::ptr::null_mut());
        g_object_unref(app as GPointer);
        status
    }
}

//! Zero-dependency Pure Rust FFI bindings to GTK4 / GLib / GObject / GIO

#![allow(dead_code, non_camel_case_types)]

use std::ffi::{c_char, c_double, c_int, c_long, c_uint, c_ulong, c_void, CStr, CString};

pub type GPointer = *mut c_void;
pub type GtkWidget = c_void;
pub type GtkWindow = c_void;
pub type GtkApplication = c_void;
pub type GtkCssProvider = c_void;
pub type GdkDisplay = c_void;
pub type GtkTextBuffer = c_void;
pub type GtkEntryBuffer = c_void;
pub type GApplication = c_void;

pub const GTK_ORIENTATION_HORIZONTAL: c_int = 0;
pub const GTK_ORIENTATION_VERTICAL: c_int = 1;

pub const GTK_ALIGN_FILL: c_int = 0;
pub const GTK_ALIGN_START: c_int = 1;
pub const GTK_ALIGN_END: c_int = 2;
pub const GTK_ALIGN_CENTER: c_int = 3;

pub const GTK_STYLE_PROVIDER_PRIORITY_APPLICATION: c_uint = 600;
pub const GTK_STYLE_PROVIDER_PRIORITY_USER: c_uint = 800;

pub const GTK_POLICY_ALWAYS: c_int = 0;
pub const GTK_POLICY_AUTOMATIC: c_int = 1;
pub const GTK_POLICY_NEVER: c_int = 2;

pub const GTK_WRAP_WORD: c_int = 2;
pub const GTK_WRAP_WORD_CHAR: c_int = 3;

pub const G_APPLICATION_DEFAULT_FLAGS: c_int = 0;
pub const G_APPLICATION_NON_UNIQUE: c_int = 1 << 5;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct GdkRGBA {
    pub red: f32,
    pub green: f32,
    pub blue: f32,
    pub alpha: f32,
}

impl GdkRGBA {
    pub const fn hex(r: u8, g: u8, b: u8) -> Self {
        Self {
            red: r as f32 / 255.0,
            green: g as f32 / 255.0,
            blue: b as f32 / 255.0,
            alpha: 1.0,
        }
    }
}

#[repr(C)]
pub struct GtkTextIter {
    pub dummy: [u8; 80],
}

impl GtkTextIter {
    pub fn zeroed() -> Self {
        Self { dummy: [0u8; 80] }
    }
}

extern "C" {
    // GApplication / GtkApplication
    pub fn gtk_application_new(application_id: *const c_char, flags: c_int) -> *mut GtkApplication;
    pub fn g_application_run(application: *mut GApplication, argc: c_int, argv: *mut *mut c_char) -> c_int;
    pub fn g_application_quit(application: *mut GApplication);

    // GObject / Signals / Main Loop
    pub fn g_signal_connect_data(
        instance: GPointer,
        detailed_signal: *const c_char,
        c_handler: GPointer,
        data: GPointer,
        destroy_data: GPointer,
        connect_flags: c_int,
    ) -> c_ulong;
    pub fn g_object_ref(object: GPointer) -> GPointer;
    pub fn g_object_unref(object: GPointer);
    pub fn g_free(mem: GPointer);
    pub fn g_timeout_add(
        interval_ms: c_uint,
        function: extern "C" fn(GPointer) -> c_int,
        data: GPointer,
    ) -> c_uint;
    pub fn g_timeout_add_seconds(
        interval_sec: c_uint,
        function: extern "C" fn(GPointer) -> c_int,
        data: GPointer,
    ) -> c_uint;

    // GdkDisplay & CSS
    pub fn gdk_display_get_default() -> *mut GdkDisplay;
    pub fn gtk_css_provider_new() -> *mut GtkCssProvider;
    pub fn gtk_css_provider_load_from_string(provider: *mut GtkCssProvider, string: *const c_char);
    pub fn gtk_style_context_add_provider_for_display(
        display: *mut GdkDisplay,
        provider: *mut GtkCssProvider,
        priority: c_uint,
    );

    // GtkWindow / GtkApplicationWindow
    pub fn gtk_application_window_new(app: *mut GtkApplication) -> *mut GtkWidget;
    pub fn gtk_window_new() -> *mut GtkWidget;
    pub fn gtk_window_set_application(window: *mut GtkWindow, app: *mut GtkApplication);
    pub fn gtk_window_set_title(window: *mut GtkWindow, title: *const c_char);
    pub fn gtk_window_set_default_size(window: *mut GtkWindow, width: c_int, height: c_int);
    pub fn gtk_window_set_decorated(window: *mut GtkWindow, setting: c_int);
    pub fn gtk_window_set_resizable(window: *mut GtkWindow, resizable: c_int);
    pub fn gtk_window_maximize(window: *mut GtkWindow);
    pub fn gtk_window_unmaximize(window: *mut GtkWindow);
    pub fn gtk_window_is_maximized(window: *mut GtkWindow) -> c_int;
    pub fn gtk_window_minimize(window: *mut GtkWindow);
    pub fn gtk_window_fullscreen(window: *mut GtkWindow);
    pub fn gtk_window_unfullscreen(window: *mut GtkWindow);
    pub fn gtk_window_present(window: *mut GtkWindow);
    pub fn gtk_window_close(window: *mut GtkWindow);
    pub fn gtk_window_destroy(window: *mut GtkWindow);
    pub fn gtk_window_set_child(window: *mut GtkWindow, child: *mut GtkWidget);
    pub fn gtk_window_set_titlebar(window: *mut GtkWindow, titlebar: *mut GtkWidget);
    pub fn gtk_window_set_modal(window: *mut GtkWindow, modal: c_int);
    pub fn gtk_window_set_transient_for(window: *mut GtkWindow, parent: *mut GtkWindow);

    // GtkWindowHandle (draggable custom macOS titlebar)
    pub fn gtk_window_handle_new() -> *mut GtkWidget;
    pub fn gtk_window_handle_set_child(handle: *mut GtkWidget, child: *mut GtkWidget);

    // GtkImage, GtkIconTheme & GtkPicture
    pub fn gtk_image_new_from_file(filename: *const c_char) -> *mut GtkWidget;
    pub fn gtk_image_new_from_icon_name(icon_name: *const c_char) -> *mut GtkWidget;
    pub fn gtk_image_set_pixel_size(image: *mut GtkWidget, pixel_size: c_int);
    pub fn gtk_icon_theme_get_for_display(display: *mut GdkDisplay) -> GPointer;
    pub fn gtk_icon_theme_has_icon(icon_theme: GPointer, icon_name: *const c_char) -> c_int;
    pub fn gtk_picture_new_for_filename(filename: *const c_char) -> *mut GtkWidget;
    pub fn gtk_picture_set_content_fit(picture: *mut GtkWidget, content_fit: c_int);

    // GtkWidget
    pub fn gtk_widget_set_visible(widget: *mut GtkWidget, visible: c_int);
    pub fn gtk_widget_get_visible(widget: *mut GtkWidget) -> c_int;
    pub fn gtk_widget_add_css_class(widget: *mut GtkWidget, css_class: *const c_char);
    pub fn gtk_widget_remove_css_class(widget: *mut GtkWidget, css_class: *const c_char);
    pub fn gtk_widget_set_hexpand(widget: *mut GtkWidget, expand: c_int);
    pub fn gtk_widget_set_vexpand(widget: *mut GtkWidget, expand: c_int);
    pub fn gtk_widget_set_halign(widget: *mut GtkWidget, align: c_int);
    pub fn gtk_widget_set_valign(widget: *mut GtkWidget, align: c_int);
    pub fn gtk_widget_set_size_request(widget: *mut GtkWidget, width: c_int, height: c_int);
    pub fn gtk_widget_set_margin_top(widget: *mut GtkWidget, margin: c_int);
    pub fn gtk_widget_set_margin_bottom(widget: *mut GtkWidget, margin: c_int);
    pub fn gtk_widget_set_margin_start(widget: *mut GtkWidget, margin: c_int);
    pub fn gtk_widget_set_margin_end(widget: *mut GtkWidget, margin: c_int);
    pub fn gtk_widget_set_tooltip_text(widget: *mut GtkWidget, text: *const c_char);
    pub fn gtk_widget_grab_focus(widget: *mut GtkWidget) -> c_int;
    pub fn gtk_widget_has_focus(widget: *mut GtkWidget) -> c_int;
    pub fn gtk_widget_set_sensitive(widget: *mut GtkWidget, sensitive: c_int);
    pub fn gtk_widget_set_opacity(widget: *mut GtkWidget, opacity: c_double);
    pub fn gtk_widget_set_overflow(widget: *mut GtkWidget, overflow: c_int);
    pub fn gtk_widget_get_parent(widget: *mut GtkWidget) -> *mut GtkWidget;
    pub fn gtk_widget_add_controller(widget: *mut GtkWidget, controller: GPointer);

    // GtkEventControllerKey (Keyboard shortcuts / Hotkeys)
    pub fn gtk_event_controller_key_new() -> GPointer;
    pub fn gtk_event_controller_set_propagation_phase(controller: GPointer, phase: c_int);

    // GtkOrientable (switch Dock between horizontal & vertical live)
    pub fn gtk_orientable_set_orientation(orientable: *mut GtkWidget, orientation: c_int);

    // GtkScale & GtkRange (Dock Size & Magnification sliders)
    pub fn gtk_scale_new_with_range(
        orientation: c_int,
        min: c_double,
        max: c_double,
        step: c_double,
    ) -> *mut GtkWidget;
    pub fn gtk_range_get_value(range: *mut GtkWidget) -> c_double;
    pub fn gtk_range_set_value(range: *mut GtkWidget, value: c_double);

    // GtkSwitch (Toggle switches)
    pub fn gtk_switch_new() -> *mut GtkWidget;
    pub fn gtk_switch_get_active(sw: *mut GtkWidget) -> c_int;
    pub fn gtk_switch_set_active(sw: *mut GtkWidget, is_active: c_int);

    // GtkBox
    pub fn gtk_box_new(orientation: c_int, spacing: c_int) -> *mut GtkWidget;
    pub fn gtk_box_append(box_w: *mut GtkWidget, child: *mut GtkWidget);
    pub fn gtk_box_prepend(box_w: *mut GtkWidget, child: *mut GtkWidget);
    pub fn gtk_box_remove(box_w: *mut GtkWidget, child: *mut GtkWidget);
    pub fn gtk_box_set_homogeneous(box_w: *mut GtkWidget, homogeneous: c_int);
    pub fn gtk_widget_get_first_child(widget: *mut GtkWidget) -> *mut GtkWidget;

    // GtkCenterBox
    pub fn gtk_center_box_new() -> *mut GtkWidget;
    pub fn gtk_center_box_set_start_widget(cb: *mut GtkWidget, child: *mut GtkWidget);
    pub fn gtk_center_box_set_center_widget(cb: *mut GtkWidget, child: *mut GtkWidget);
    pub fn gtk_center_box_set_end_widget(cb: *mut GtkWidget, child: *mut GtkWidget);

    // GtkGrid
    pub fn gtk_grid_new() -> *mut GtkWidget;
    pub fn gtk_grid_attach(
        grid: *mut GtkWidget,
        child: *mut GtkWidget,
        column: c_int,
        row: c_int,
        width: c_int,
        height: c_int,
    );
    pub fn gtk_grid_set_row_spacing(grid: *mut GtkWidget, spacing: c_uint);
    pub fn gtk_grid_set_column_spacing(grid: *mut GtkWidget, spacing: c_uint);
    pub fn gtk_grid_set_column_homogeneous(grid: *mut GtkWidget, homogeneous: c_int);

    // GtkFlowBox
    pub fn gtk_flow_box_new() -> *mut GtkWidget;
    pub fn gtk_flow_box_append(flow: *mut GtkWidget, child: *mut GtkWidget);
    pub fn gtk_flow_box_remove(flow: *mut GtkWidget, child: *mut GtkWidget);
    pub fn gtk_flow_box_set_max_children_per_line(flow: *mut GtkWidget, n: c_uint);
    pub fn gtk_flow_box_set_min_children_per_line(flow: *mut GtkWidget, n: c_uint);
    pub fn gtk_flow_box_set_row_spacing(flow: *mut GtkWidget, spacing: c_uint);
    pub fn gtk_flow_box_set_column_spacing(flow: *mut GtkWidget, spacing: c_uint);
    pub fn gtk_flow_box_set_selection_mode(flow: *mut GtkWidget, mode: c_int);

    // GtkScrolledWindow & GtkAdjustment
    pub fn gtk_scrolled_window_new() -> *mut GtkWidget;
    pub fn gtk_scrolled_window_set_child(sw: *mut GtkWidget, child: *mut GtkWidget);
    pub fn gtk_scrolled_window_set_policy(sw: *mut GtkWidget, h: c_int, v: c_int);
    pub fn gtk_scrolled_window_get_vadjustment(sw: *mut GtkWidget) -> GPointer;
    pub fn gtk_adjustment_get_upper(adj: GPointer) -> c_double;
    pub fn gtk_adjustment_set_value(adj: GPointer, value: c_double);

    // GtkLabel
    pub fn gtk_label_new(text: *const c_char) -> *mut GtkWidget;
    pub fn gtk_label_set_text(label: *mut GtkWidget, text: *const c_char);
    pub fn gtk_label_set_markup(label: *mut GtkWidget, markup: *const c_char);
    pub fn gtk_label_set_xalign(label: *mut GtkWidget, xalign: f32);
    pub fn gtk_label_set_wrap(label: *mut GtkWidget, wrap: c_int);
    pub fn gtk_label_set_selectable(label: *mut GtkWidget, setting: c_int);
    pub fn gtk_label_set_max_width_chars(label: *mut GtkWidget, n_chars: c_int);
    pub fn gtk_label_set_ellipsize(label: *mut GtkWidget, mode: c_int);

    // GtkButton
    pub fn gtk_button_new() -> *mut GtkWidget;
    pub fn gtk_button_new_with_label(label: *const c_char) -> *mut GtkWidget;
    pub fn gtk_button_set_child(button: *mut GtkWidget, child: *mut GtkWidget);
    pub fn gtk_button_set_label(button: *mut GtkWidget, label: *const c_char);

    // GtkEntry & GtkEditable
    pub fn gtk_entry_new() -> *mut GtkWidget;
    pub fn gtk_entry_set_placeholder_text(entry: *mut GtkWidget, text: *const c_char);
    pub fn gtk_entry_set_visibility(entry: *mut GtkWidget, visible: c_int);
    pub fn gtk_entry_set_invisible_char(entry: *mut GtkWidget, ch: c_uint);
    pub fn gtk_editable_get_text(editable: *mut GtkWidget) -> *const c_char;
    pub fn gtk_editable_set_text(editable: *mut GtkWidget, text: *const c_char);

    // GtkTextView & GtkTextBuffer
    pub fn gtk_text_view_new() -> *mut GtkWidget;
    pub fn gtk_text_view_get_buffer(tv: *mut GtkWidget) -> *mut GtkTextBuffer;
    pub fn gtk_text_view_set_wrap_mode(tv: *mut GtkWidget, wrap_mode: c_int);
    pub fn gtk_text_view_set_editable(tv: *mut GtkWidget, setting: c_int);
    pub fn gtk_text_view_set_monospace(tv: *mut GtkWidget, monospace: c_int);
    pub fn gtk_text_buffer_set_text(buf: *mut GtkTextBuffer, text: *const c_char, len: c_int);
    pub fn gtk_text_buffer_get_bounds(
        buf: *mut GtkTextBuffer,
        start: *mut GtkTextIter,
        end: *mut GtkTextIter,
    );
    pub fn gtk_text_buffer_get_text(
        buf: *mut GtkTextBuffer,
        start: *const GtkTextIter,
        end: *const GtkTextIter,
        include_hidden: c_int,
    ) -> *mut c_char;

    // GtkProgressBar
    pub fn gtk_progress_bar_new() -> *mut GtkWidget;
    pub fn gtk_progress_bar_set_fraction(pbar: *mut GtkWidget, fraction: c_double);
    pub fn gtk_progress_bar_set_text(pbar: *mut GtkWidget, text: *const c_char);
    pub fn gtk_progress_bar_set_show_text(pbar: *mut GtkWidget, show: c_int);

    // GtkSeparator
    pub fn gtk_separator_new(orientation: c_int) -> *mut GtkWidget;

    // GtkStack
    pub fn gtk_stack_new() -> *mut GtkWidget;
    pub fn gtk_stack_add_named(stack: *mut GtkWidget, child: *mut GtkWidget, name: *const c_char) -> GPointer;
    pub fn gtk_stack_remove(stack: *mut GtkWidget, child: *mut GtkWidget);
    pub fn gtk_stack_set_visible_child_name(stack: *mut GtkWidget, name: *const c_char);

    // GtkPaned (Split Vertical / Split Horizontal panes in Pancer)
    pub fn gtk_paned_new(orientation: c_int) -> *mut GtkWidget;
    pub fn gtk_paned_set_start_child(paned: *mut GtkWidget, child: *mut GtkWidget);
    pub fn gtk_paned_set_end_child(paned: *mut GtkWidget, child: *mut GtkWidget);
    pub fn gtk_paned_set_resize_start_child(paned: *mut GtkWidget, resize: c_int);
    pub fn gtk_paned_set_resize_end_child(paned: *mut GtkWidget, resize: c_int);
    pub fn gtk_paned_set_shrink_start_child(paned: *mut GtkWidget, shrink: c_int);
    pub fn gtk_paned_set_shrink_end_child(paned: *mut GtkWidget, shrink: c_int);

    // GtkOverlay (in-window floating overlays with true transparency, zero black X11 popup)
    pub fn gtk_overlay_new() -> *mut GtkWidget;
    pub fn gtk_overlay_set_child(overlay: *mut GtkWidget, child: *mut GtkWidget);
    pub fn gtk_overlay_add_overlay(overlay: *mut GtkWidget, widget: *mut GtkWidget);

    // GtkPopover & GtkMenuButton
    pub fn gtk_menu_button_new() -> *mut GtkWidget;
    pub fn gtk_menu_button_set_label(btn: *mut GtkWidget, label: *const c_char);
    pub fn gtk_menu_button_set_child(btn: *mut GtkWidget, child: *mut GtkWidget);
    pub fn gtk_menu_button_set_popover(btn: *mut GtkWidget, popover: *mut GtkWidget);
    pub fn gtk_popover_new() -> *mut GtkWidget;
    pub fn gtk_popover_set_child(popover: *mut GtkWidget, child: *mut GtkWidget);
    pub fn gtk_popover_popdown(popover: *mut GtkWidget);

    // VteTerminal (GTK4 Real Interactive PTY/TTY Terminal) & Pango Font
    pub fn vte_terminal_new() -> *mut GtkWidget;
    pub fn vte_terminal_spawn_async(
        terminal: *mut GtkWidget,
        pty_flags: c_int,
        working_directory: *const c_char,
        argv: *const *const c_char,
        envv: *const *const c_char,
        spawn_flags: c_int,
        child_setup: GPointer,
        child_setup_data: GPointer,
        child_setup_data_destroy: GPointer,
        timeout: c_int,
        cancellable: GPointer,
        callback: GPointer,
        user_data: GPointer,
    );
    pub fn vte_terminal_set_colors(
        terminal: *mut GtkWidget,
        foreground: *const GdkRGBA,
        background: *const GdkRGBA,
        palette: *const GdkRGBA,
        palette_size: usize,
    );
    pub fn vte_terminal_set_color_cursor(terminal: *mut GtkWidget, cursor_background: *const GdkRGBA);
    pub fn vte_terminal_set_scrollback_lines(terminal: *mut GtkWidget, lines: c_long);
    pub fn vte_terminal_set_font(terminal: *mut GtkWidget, font_desc: GPointer);
    pub fn vte_terminal_copy_clipboard_format(terminal: *mut GtkWidget, format: c_int);
    pub fn vte_terminal_paste_clipboard(terminal: *mut GtkWidget);
    pub fn vte_terminal_reset(terminal: *mut GtkWidget, clear_tabstops: c_int, clear_history: c_int);
    pub fn vte_terminal_get_window_title(terminal: *mut GtkWidget) -> *const c_char;
    pub fn pango_font_description_from_string(str_desc: *const c_char) -> GPointer;
    pub fn pango_font_description_free(desc: GPointer);

    // GtkDrawingArea & Cairo (Real-time Activity Monitor charts)
    pub fn gtk_drawing_area_new() -> *mut GtkWidget;
    pub fn gtk_drawing_area_set_content_width(area: *mut GtkWidget, width: c_int);
    pub fn gtk_drawing_area_set_content_height(area: *mut GtkWidget, height: c_int);
    pub fn gtk_drawing_area_set_draw_func(
        area: *mut GtkWidget,
        draw_func: extern "C" fn(*mut GtkWidget, GPointer, c_int, c_int, GPointer),
        user_data: GPointer,
        destroy: extern "C" fn(GPointer),
    );
    pub fn gtk_widget_queue_draw(widget: *mut GtkWidget);
    pub fn cairo_set_source_rgba(cr: GPointer, r: c_double, g: c_double, b: c_double, a: c_double);
    pub fn cairo_set_line_width(cr: GPointer, width: c_double);
    pub fn cairo_set_line_join(cr: GPointer, line_join: c_int);
    pub fn cairo_set_line_cap(cr: GPointer, line_cap: c_int);
    pub fn cairo_move_to(cr: GPointer, x: c_double, y: c_double);
    pub fn cairo_line_to(cr: GPointer, x: c_double, y: c_double);
    pub fn cairo_close_path(cr: GPointer);
    pub fn cairo_new_path(cr: GPointer);
    pub fn cairo_stroke(cr: GPointer);
    pub fn cairo_fill(cr: GPointer);
    pub fn cairo_rectangle(cr: GPointer, x: c_double, y: c_double, width: c_double, height: c_double);
    pub fn cairo_arc(cr: GPointer, xc: c_double, yc: c_double, radius: c_double, angle1: c_double, angle2: c_double);

    // GTK4 X11 Native Surface + Xlib EWMH (_NET_WM_WINDOW_TYPE_DESKTOP)
    pub fn gtk_native_get_surface(native: *mut GtkWidget) -> GPointer;
    pub fn gdk_x11_display_get_xdisplay(display: *mut GdkDisplay) -> GPointer;
    pub fn gdk_x11_surface_get_xid(surface: GPointer) -> c_ulong;
    pub fn XDefaultScreen(display: GPointer) -> c_int;
    pub fn XDefaultRootWindow(display: GPointer) -> c_ulong;
    pub fn XGetGeometry(
        display: GPointer,
        drawable: c_ulong,
        root_return: *mut c_ulong,
        x_return: *mut c_int,
        y_return: *mut c_int,
        width_return: *mut c_uint,
        height_return: *mut c_uint,
        border_width_return: *mut c_uint,
        depth_return: *mut c_uint,
    ) -> c_int;
    pub fn XDisplayWidth(display: GPointer, screen: c_int) -> c_int;
    pub fn XDisplayHeight(display: GPointer, screen: c_int) -> c_int;
    pub fn XInternAtom(display: GPointer, atom_name: *const c_char, only_if_exists: c_int) -> c_ulong;
    pub fn XChangeProperty(
        display: GPointer,
        window: c_ulong,
        property: c_ulong,
        prop_type: c_ulong,
        format: c_int,
        mode: c_int,
        data: *const u8,
        nelements: c_int,
    ) -> c_int;
    pub fn XMoveResizeWindow(
        display: GPointer,
        window: c_ulong,
        x: c_int,
        y: c_int,
        width: c_uint,
        height: c_uint,
    ) -> c_int;
    pub fn XRaiseWindow(display: GPointer, window: c_ulong) -> c_int;
    pub fn XFlush(display: GPointer) -> c_int;
}

// ============================================================================
// Safe Ergonomic Helpers
// ============================================================================

pub fn cstr(s: &str) -> CString {
    CString::new(s.replace('\0', "")).unwrap_or_else(|_| CString::new("").unwrap())
}

pub unsafe fn add_css(w: *mut GtkWidget, class: &str) {
    let c = cstr(class);
    gtk_widget_add_css_class(w, c.as_ptr());
}

pub unsafe fn remove_css(w: *mut GtkWidget, class: &str) {
    let c = cstr(class);
    gtk_widget_remove_css_class(w, c.as_ptr());
}

pub unsafe fn set_margins(w: *mut GtkWidget, top: i32, bottom: i32, start: i32, end: i32) {
    gtk_widget_set_margin_top(w, top);
    gtk_widget_set_margin_bottom(w, bottom);
    gtk_widget_set_margin_start(w, start);
    gtk_widget_set_margin_end(w, end);
}

pub unsafe fn set_margin_all(w: *mut GtkWidget, m: i32) {
    set_margins(w, m, m, m, m);
}

pub unsafe fn new_label(text: &str, css: &[&str]) -> *mut GtkWidget {
    let c = cstr(text);
    let w = gtk_label_new(c.as_ptr());
    for cls in css {
        add_css(w, cls);
    }
    w
}

pub unsafe fn set_label_text(w: *mut GtkWidget, text: &str) {
    let c = cstr(text);
    gtk_label_set_text(w, c.as_ptr());
}

pub unsafe fn new_button(label: &str, css: &[&str]) -> *mut GtkWidget {
    let c = cstr(label);
    let w = gtk_button_new_with_label(c.as_ptr());
    for cls in css {
        add_css(w, cls);
    }
    w
}

pub fn resolve_wallpaper_path(filename: &str) -> Option<String> {
    let candidates = [
        format!("/usr/share/bijios-desktop/static/wallpapers/{}", filename),
        format!("/home/dionipe/apps/bijios-desktop/static/wallpapers/{}", filename),
        format!("static/wallpapers/{}", filename),
    ];
    for p in candidates {
        if std::path::Path::new(&p).exists() {
            return Some(p);
        }
    }
    None
}

pub unsafe fn new_wallpaper_preview(filename: &str, width: i32, height: i32) -> *mut GtkWidget {
    if let Some(path) = resolve_wallpaper_path(filename) {
        let c = cstr(&path);
        let pic = gtk_picture_new_for_filename(c.as_ptr());
        gtk_picture_set_content_fit(pic, 2); // GTK_CONTENT_FIT_COVER
        gtk_widget_set_size_request(pic, width, height);
        add_css(pic, "wp-preview-img");
        return pic;
    }
    let fallback = new_box(GTK_ORIENTATION_VERTICAL, 0, &["wp-preview-img"]);
    gtk_widget_set_size_request(fallback, width, height);
    fallback
}

/// Clickable hyperlink button that opens URL in the default browser via xdg-open
pub unsafe fn new_link_button(label: &str, url: &str, css: &[&str]) -> *mut GtkWidget {
    let btn = new_button(label, css);
    let tip = cstr(url);
    gtk_widget_set_tooltip_text(btn, tip.as_ptr());
    let u = url.to_string();
    connect_signal(btn, "clicked", move || {
        let _ = std::process::Command::new("xdg-open")
            .arg(&u)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();
    });
    btn
}

/// Circular User Profile Avatar widget:
/// - `file:<path>` -> custom circular photo clipped with GTK_OVERFLOW_HIDDEN
/// - `preset:<emoji>:<grad_css>` -> curated gradient circular icon
/// - `initial` (or fallback) -> Hijau Tunas gradient circle with uppercase initial
pub unsafe fn new_user_avatar_widget(avatar: &str, username: &str, size: i32) -> *mut GtkWidget {
    if let Some(path) = avatar.strip_prefix("file:") {
        if !path.is_empty() && std::path::Path::new(path).exists() {
            let frame = new_box(GTK_ORIENTATION_VERTICAL, 0, &["avatar-photo-frame"]);
            gtk_widget_set_size_request(frame, size, size);
            gtk_widget_set_halign(frame, GTK_ALIGN_CENTER);
            gtk_widget_set_valign(frame, GTK_ALIGN_CENTER);
            gtk_widget_set_overflow(frame, 1); // GTK_OVERFLOW_HIDDEN (clips to 999px circle)

            let c = cstr(path);
            let pic = gtk_picture_new_for_filename(c.as_ptr());
            gtk_picture_set_content_fit(pic, 2); // GTK_CONTENT_FIT_COVER
            gtk_widget_set_size_request(pic, size, size);
            gtk_widget_set_hexpand(pic, 1);
            gtk_widget_set_vexpand(pic, 1);
            gtk_box_append(frame, pic);
            return frame;
        }
    }

    let size_css = if size <= 24 {
        "avatar-circle-sm"
    } else if size <= 60 {
        "avatar-circle-md"
    } else {
        "avatar-circle-lg"
    };

    if let Some(rest) = avatar.strip_prefix("preset:") {
        let parts: Vec<&str> = rest.split(':').collect();
        if parts.len() == 2 {
            let lbl = new_label(parts[0], &[size_css, parts[1]]);
            gtk_widget_set_size_request(lbl, size, size);
            gtk_widget_set_halign(lbl, GTK_ALIGN_CENTER);
            gtk_widget_set_valign(lbl, GTK_ALIGN_CENTER);
            return lbl;
        }
    }

    let init_ch = username
        .chars()
        .next()
        .unwrap_or('U')
        .to_uppercase()
        .to_string();
    let lbl = new_label(&init_ch, &[size_css, "avatar-grad-green"]);
    gtk_widget_set_size_request(lbl, size, size);
    gtk_widget_set_halign(lbl, GTK_ALIGN_CENTER);
    gtk_widget_set_valign(lbl, GTK_ALIGN_CENTER);
    lbl
}

/// Official BijiOS SVG Logo widget (Golden Seed + Hijau Tunas & Hijau Muda leaves)
pub unsafe fn new_logo_image(pixel_size: i32, badge: bool) -> *mut GtkWidget {
    let filename = if badge {
        "bijios-logo-badge.svg"
    } else {
        "bijios-logo.svg"
    };
    let candidates = [
        format!("/usr/share/bijios-desktop/static/img/{}", filename),
        format!("/home/dionipe/apps/bijios-desktop/static/img/{}", filename),
        format!("static/img/{}", filename),
    ];
    for path in &candidates {
        if std::path::Path::new(path).exists() {
            let c = cstr(path);
            let img = gtk_image_new_from_file(c.as_ptr());
            gtk_image_set_pixel_size(img, pixel_size);
            return img;
        }
    }
    new_label("🌱", &[])
}

/// Host IDVE / KVM Logo widget (from https://idve.cloud/images/logo.png)
pub unsafe fn new_idve_logo(width: i32, height: i32) -> *mut GtkWidget {
    let candidates = [
        "/usr/share/bijios-desktop/static/img/idve-logo.png",
        "/home/dionipe/apps/bijios-desktop/static/img/idve-logo.png",
        "static/img/idve-logo.png",
    ];
    for path in candidates {
        if std::path::Path::new(path).exists() {
            let c = cstr(path);
            let pic = gtk_picture_new_for_filename(c.as_ptr());
            gtk_picture_set_content_fit(pic, 1); // GTK_CONTENT_FIT_CONTAIN
            gtk_widget_set_size_request(pic, width, height);
            return pic;
        }
    }
    new_label("IDVE / KVM", &["app-section-title"])
}

/// Left-aligned button with icon + label (fixes GTK4 default button centering)
pub unsafe fn new_left_button(icon: &str, text: &str, css: &[&str]) -> *mut GtkWidget {
    let btn = gtk_button_new();
    for cls in css {
        add_css(btn, cls);
    }
    let row = new_box(GTK_ORIENTATION_HORIZONTAL, 10, &[]);
    gtk_widget_set_halign(row, GTK_ALIGN_START);
    gtk_widget_set_hexpand(row, 1);

    if !icon.is_empty() {
        let ic = new_label(icon, &["btn-icon"]);
        gtk_box_append(row, ic);
    }
    let lbl = new_label(text, &[]);
    gtk_label_set_xalign(lbl, 0.0);
    gtk_widget_set_hexpand(lbl, 1);
    gtk_box_append(row, lbl);

    gtk_button_set_child(btn, row);
    btn
}

/// Proportional macOS Launchpad tile: 52x52 glossy squircle icon badge + caption below
pub unsafe fn new_icon_tile_button(
    icon: &str,
    icon_css: &str,
    label: &str,
    tile_css: &[&str],
) -> *mut GtkWidget {
    let btn = gtk_button_new();
    for cls in tile_css {
        add_css(btn, cls);
    }
    let col = new_box(GTK_ORIENTATION_VERTICAL, 8, &[]);
    gtk_widget_set_halign(col, GTK_ALIGN_CENTER);
    gtk_widget_set_valign(col, GTK_ALIGN_CENTER);

    let badge = new_label(icon, &["launchpad-icon-badge", icon_css]);
    gtk_widget_set_halign(badge, GTK_ALIGN_CENTER);
    let lbl = new_label(label, &["launchpad-caption"]);
    gtk_widget_set_halign(lbl, GTK_ALIGN_CENTER);

    gtk_box_append(col, badge);
    gtk_box_append(col, lbl);
    gtk_button_set_child(btn, col);
    btn
}

/// Resolve a Freedesktop `.desktop` Icon= entry (absolute path, hicolor/pixmaps, or GtkIconTheme)
pub fn resolve_desktop_icon_file(icon_name: &str) -> Option<String> {
    let trimmed = icon_name.trim();
    if trimmed.is_empty() {
        return None;
    }
    let p = std::path::Path::new(trimmed);
    if p.is_absolute() && p.is_file() {
        return Some(trimmed.to_string());
    }
    let stem = trimmed
        .strip_suffix(".png")
        .or_else(|| trimmed.strip_suffix(".svg"))
        .or_else(|| trimmed.strip_suffix(".xpm"))
        .unwrap_or(trimmed);

    let candidates = [
        format!("/usr/share/icons/hicolor/scalable/apps/{}.svg", stem),
        format!("/usr/share/icons/hicolor/256x256/apps/{}.png", stem),
        format!("/usr/share/icons/hicolor/128x128/apps/{}.png", stem),
        format!("/usr/share/icons/hicolor/96x96/apps/{}.png", stem),
        format!("/usr/share/icons/hicolor/64x64/apps/{}.png", stem),
        format!("/usr/share/icons/hicolor/48x48/apps/{}.png", stem),
        format!("/usr/share/pixmaps/{}.svg", stem),
        format!("/usr/share/pixmaps/{}.png", stem),
        format!("/usr/share/pixmaps/{}.xpm", stem),
        format!("/usr/share/icons/{}.svg", stem),
        format!("/usr/share/icons/{}.png", stem),
    ];
    for c in candidates {
        if std::path::Path::new(&c).is_file() {
            return Some(c);
        }
    }
    None
}

/// Launchpad tile for installed `.deb` apps using their real native Freedesktop icon
pub unsafe fn new_deb_icon_tile_button(
    icon_name: &str,
    fallback_emoji: &str,
    fallback_css: &str,
    label: &str,
    tile_css: &[&str],
) -> *mut GtkWidget {
    let btn = gtk_button_new();
    for cls in tile_css {
        add_css(btn, cls);
    }
    let col = new_box(GTK_ORIENTATION_VERTICAL, 8, &[]);
    gtk_widget_set_halign(col, GTK_ALIGN_CENTER);
    gtk_widget_set_valign(col, GTK_ALIGN_CENTER);

    let icon_widget: *mut GtkWidget = if let Some(file_path) = resolve_desktop_icon_file(icon_name) {
        let c = cstr(&file_path);
        let img = gtk_image_new_from_file(c.as_ptr());
        gtk_image_set_pixel_size(img, 52);
        gtk_widget_set_halign(img, GTK_ALIGN_CENTER);
        img
    } else if !icon_name.trim().is_empty() {
        let display = gdk_display_get_default();
        let theme = if !display.is_null() {
            gtk_icon_theme_get_for_display(display)
        } else {
            std::ptr::null_mut()
        };
        let c_name = cstr(icon_name.trim());
        if !theme.is_null() && gtk_icon_theme_has_icon(theme, c_name.as_ptr()) != 0 {
            let img = gtk_image_new_from_icon_name(c_name.as_ptr());
            gtk_image_set_pixel_size(img, 52);
            gtk_widget_set_halign(img, GTK_ALIGN_CENTER);
            img
        } else {
            let badge = new_label(fallback_emoji, &["launchpad-icon-badge", fallback_css]);
            gtk_widget_set_halign(badge, GTK_ALIGN_CENTER);
            badge
        }
    } else {
        let badge = new_label(fallback_emoji, &["launchpad-icon-badge", fallback_css]);
        gtk_widget_set_halign(badge, GTK_ALIGN_CENTER);
        badge
    };

    let lbl = new_label(label, &["launchpad-caption"]);
    gtk_widget_set_halign(lbl, GTK_ALIGN_CENTER);

    gtk_box_append(col, icon_widget);
    gtk_box_append(col, lbl);
    gtk_button_set_child(btn, col);
    btn
}

pub unsafe fn new_box(orientation: c_int, spacing: i32, css: &[&str]) -> *mut GtkWidget {
    let w = gtk_box_new(orientation, spacing);
    for cls in css {
        add_css(w, cls);
    }
    w
}

pub unsafe fn get_entry_text(entry: *mut GtkWidget) -> String {
    let ptr = gtk_editable_get_text(entry);
    if ptr.is_null() {
        String::new()
    } else {
        CStr::from_ptr(ptr).to_string_lossy().into_owned()
    }
}

pub unsafe fn set_entry_text(entry: *mut GtkWidget, text: &str) {
    let c = cstr(text);
    gtk_editable_set_text(entry, c.as_ptr());
}

pub unsafe fn get_text_view_text(tv: *mut GtkWidget) -> String {
    let buf = gtk_text_view_get_buffer(tv);
    let mut start = GtkTextIter::zeroed();
    let mut end = GtkTextIter::zeroed();
    gtk_text_buffer_get_bounds(buf, &mut start, &mut end);
    let ptr = gtk_text_buffer_get_text(buf, &start, &end, 0);
    if ptr.is_null() {
        return String::new();
    }
    let res = CStr::from_ptr(ptr).to_string_lossy().into_owned();
    g_free(ptr as GPointer);
    res
}

pub unsafe fn set_text_view_text(tv: *mut GtkWidget, text: &str) {
    let buf = gtk_text_view_get_buffer(tv);
    let c = cstr(text);
    gtk_text_buffer_set_text(buf, c.as_ptr(), -1);
}

pub unsafe fn clear_box_children(box_w: *mut GtkWidget) {
    loop {
        let child = gtk_widget_get_first_child(box_w);
        if child.is_null() {
            break;
        }
        gtk_box_remove(box_w, child);
    }
}

pub unsafe fn clear_flow_children(flow_w: *mut GtkWidget) {
    loop {
        let child = gtk_widget_get_first_child(flow_w);
        if child.is_null() {
            break;
        }
        gtk_flow_box_remove(flow_w, child);
    }
}

extern "C" fn trampoline_void(_widget: *mut GtkWidget, data: GPointer) {
    let cb: &mut Box<dyn FnMut()> = unsafe { &mut *(data as *mut Box<dyn FnMut()>) };
    cb();
}

extern "C" fn trampoline_notify(_widget: *mut GtkWidget, _pspec: GPointer, data: GPointer) {
    let cb: &mut Box<dyn FnMut()> = unsafe { &mut *(data as *mut Box<dyn FnMut()>) };
    cb();
}

extern "C" fn destroy_void(data: GPointer, _closure: GPointer) {
    if !data.is_null() {
        unsafe {
            let _ = Box::from_raw(data as *mut Box<dyn FnMut()>);
        }
    }
}

pub unsafe fn connect_signal<F: FnMut() + 'static>(widget: *mut GtkWidget, signal: &str, f: F) {
    let boxed: Box<Box<dyn FnMut()>> = Box::new(Box::new(f));
    let raw = Box::into_raw(boxed) as GPointer;
    let sig = cstr(signal);
    g_signal_connect_data(
        widget,
        sig.as_ptr(),
        trampoline_void as GPointer,
        raw,
        destroy_void as GPointer,
        0,
    );
}

/// Connect GObject `notify::*` property signals (3-arg C callback: obj, pspec, user_data)
pub unsafe fn connect_notify<F: FnMut() + 'static>(widget: *mut GtkWidget, signal: &str, f: F) {
    let boxed: Box<Box<dyn FnMut()>> = Box::new(Box::new(f));
    let raw = Box::into_raw(boxed) as GPointer;
    let sig = cstr(signal);
    g_signal_connect_data(
        widget,
        sig.as_ptr(),
        trampoline_notify as GPointer,
        raw,
        destroy_void as GPointer,
        0,
    );
}

extern "C" fn trampoline_key_pressed(
    _controller: GPointer,
    keyval: c_uint,
    _keycode: c_uint,
    state: c_uint,
    data: GPointer,
) -> c_int {
    let cb: &mut Box<dyn FnMut(u32, u32) -> bool> =
        unsafe { &mut *(data as *mut Box<dyn FnMut(u32, u32) -> bool>) };
    if cb(keyval, state) { 1 } else { 0 }
}

extern "C" fn destroy_key_pressed(data: GPointer, _closure: GPointer) {
    if !data.is_null() {
        unsafe {
            let _ = Box::from_raw(data as *mut Box<dyn FnMut(u32, u32) -> bool>);
        }
    }
}

/// Attach a capture-phase key-pressed handler to any widget (returns true to consume key)
pub unsafe fn connect_key_pressed<F: FnMut(u32, u32) -> bool + 'static>(
    widget: *mut GtkWidget,
    f: F,
) {
    let ctrl = gtk_event_controller_key_new();
    gtk_event_controller_set_propagation_phase(ctrl, 1); // GTK_PHASE_CAPTURE
    let boxed: Box<Box<dyn FnMut(u32, u32) -> bool>> = Box::new(Box::new(f));
    let raw = Box::into_raw(boxed) as GPointer;
    let sig = cstr("key-pressed");
    g_signal_connect_data(
        ctrl,
        sig.as_ptr(),
        trampoline_key_pressed as GPointer,
        raw,
        destroy_key_pressed as GPointer,
        0,
    );
    gtk_widget_add_controller(widget, ctrl);
}

extern "C" fn trampoline_timer(data: GPointer) -> c_int {
    let cb: &mut Box<dyn FnMut() -> bool> = unsafe { &mut *(data as *mut Box<dyn FnMut() -> bool>) };
    if cb() {
        1
    } else {
        unsafe {
            let _ = Box::from_raw(data as *mut Box<dyn FnMut() -> bool>);
        }
        0
    }
}

pub unsafe fn timeout_add_sec<F: FnMut() -> bool + 'static>(sec: u32, f: F) {
    let boxed: Box<Box<dyn FnMut() -> bool>> = Box::new(Box::new(f));
    let raw = Box::into_raw(boxed) as GPointer;
    g_timeout_add_seconds(sec, trampoline_timer, raw);
}

pub unsafe fn timeout_add_ms<F: FnMut() -> bool + 'static>(ms: u32, f: F) {
    let boxed: Box<Box<dyn FnMut() -> bool>> = Box::new(Box::new(f));
    let raw = Box::into_raw(boxed) as GPointer;
    g_timeout_add(ms, trampoline_timer, raw);
}

extern "C" fn trampoline_draw(
    _area: *mut GtkWidget,
    cr: GPointer,
    width: c_int,
    height: c_int,
    data: GPointer,
) {
    let cb: &mut Box<dyn FnMut(GPointer, i32, i32)> =
        unsafe { &mut *(data as *mut Box<dyn FnMut(GPointer, i32, i32)>) };
    cb(cr, width, height);
}

extern "C" fn destroy_draw(data: GPointer) {
    if !data.is_null() {
        unsafe {
            let _ = Box::from_raw(data as *mut Box<dyn FnMut(GPointer, i32, i32)>);
        }
    }
}

pub unsafe fn set_drawing_area_draw_func<F: FnMut(GPointer, i32, i32) + 'static>(
    area: *mut GtkWidget,
    f: F,
) {
    let boxed: Box<Box<dyn FnMut(GPointer, i32, i32)>> = Box::new(Box::new(f));
    let raw = Box::into_raw(boxed) as GPointer;
    gtk_drawing_area_set_draw_func(area, trampoline_draw, raw, destroy_draw);
}

pub unsafe fn scroll_window_to_bottom(sw: *mut GtkWidget) {
    let adj = gtk_scrolled_window_get_vadjustment(sw);
    if !adj.is_null() {
        let upper = gtk_adjustment_get_upper(adj);
        gtk_adjustment_set_value(adj, upper);
    }
}

/// Query LIVE physical/virtual X11 root window resolution (always up-to-date after TigerVNC / xrandr resize)
pub unsafe fn get_x11_screen_size() -> (i32, i32) {
    let display = gdk_display_get_default();
    if display.is_null() {
        return (0, 0);
    }
    let xdisplay = gdk_x11_display_get_xdisplay(display);
    if xdisplay.is_null() {
        return (0, 0);
    }
    let root = XDefaultRootWindow(xdisplay);
    let mut root_ret: c_ulong = 0;
    let mut x: c_int = 0;
    let mut y: c_int = 0;
    let mut w: c_uint = 0;
    let mut h: c_uint = 0;
    let mut border: c_uint = 0;
    let mut depth: c_uint = 0;
    if XGetGeometry(
        xdisplay,
        root,
        &mut root_ret,
        &mut x,
        &mut y,
        &mut w,
        &mut h,
        &mut border,
        &mut depth,
    ) != 0
        && w > 0
        && h > 0
    {
        return (w as i32, h as i32);
    }
    let screen = XDefaultScreen(xdisplay);
    (XDisplayWidth(xdisplay, screen), XDisplayHeight(xdisplay, screen))
}

/// Mark a GTK4 GtkWindow as an official X11 EWMH Desktop window (_NET_WM_WINDOW_TYPE_DESKTOP)
/// covering 100% of the screen (0,0 -> WxH) permanently below all apps, ignoring Openbox <margins>
/// and never interfering with Alt+Tab or raising above application windows when clicked.
pub unsafe fn set_x11_desktop_window(win: *mut GtkWidget) {
    let display = gdk_display_get_default();
    if display.is_null() {
        return;
    }
    let surface = gtk_native_get_surface(win);
    if surface.is_null() {
        return;
    }
    let xdisplay = gdk_x11_display_get_xdisplay(display);
    let xid = gdk_x11_surface_get_xid(surface);
    if xdisplay.is_null() || xid == 0 {
        return;
    }

    let (w, h) = get_x11_screen_size();

    let prop_name = cstr("_NET_WM_WINDOW_TYPE");
    let desk_name = cstr("_NET_WM_WINDOW_TYPE_DESKTOP");
    let xa_atom: c_ulong = 4; // XA_ATOM
    let prop_mode_replace: c_int = 0;

    let prop_atom = XInternAtom(xdisplay, prop_name.as_ptr(), 0);
    let desk_atom = XInternAtom(xdisplay, desk_name.as_ptr(), 0);

    XChangeProperty(
        xdisplay,
        xid,
        prop_atom,
        xa_atom,
        32,
        prop_mode_replace,
        (&desk_atom as *const c_ulong) as *const u8,
        1,
    );

    if w > 0 && h > 0 {
        gtk_window_set_default_size(win, w, h);
        gtk_widget_set_size_request(win, w, h);
        XMoveResizeWindow(xdisplay, xid, 0, 0, w as c_uint, h as c_uint);
    }
    XFlush(xdisplay);
}

/// Enforce full-screen, always-on-top X11 Lock/Login window covering 100% of the screen (0,0 -> WxH)
/// above all desktop bars, docks, and application windows so zero desktop activity is possible while locked.
pub unsafe fn set_x11_lock_window(win: *mut GtkWidget) {
    gtk_window_fullscreen(win);
    let display = gdk_display_get_default();
    if display.is_null() {
        return;
    }
    let surface = gtk_native_get_surface(win);
    if surface.is_null() {
        return;
    }
    let xdisplay = gdk_x11_display_get_xdisplay(display);
    let xid = gdk_x11_surface_get_xid(surface);
    if xdisplay.is_null() || xid == 0 {
        return;
    }

    let (w, h) = get_x11_screen_size();

    let xa_atom: c_ulong = 4; // XA_ATOM
    let prop_mode_replace: c_int = 0;

    let state_prop = XInternAtom(xdisplay, cstr("_NET_WM_STATE").as_ptr(), 0);
    let states: [c_ulong; 4] = [
        XInternAtom(xdisplay, cstr("_NET_WM_STATE_FULLSCREEN").as_ptr(), 0),
        XInternAtom(xdisplay, cstr("_NET_WM_STATE_ABOVE").as_ptr(), 0),
        XInternAtom(xdisplay, cstr("_NET_WM_STATE_SKIP_TASKBAR").as_ptr(), 0),
        XInternAtom(xdisplay, cstr("_NET_WM_STATE_SKIP_PAGER").as_ptr(), 0),
    ];

    XChangeProperty(
        xdisplay,
        xid,
        state_prop,
        xa_atom,
        32,
        prop_mode_replace,
        states.as_ptr() as *const u8,
        states.len() as c_int,
    );

    if w > 0 && h > 0 {
        gtk_window_set_default_size(win, w, h);
        gtk_widget_set_size_request(win, w, h);
        XMoveResizeWindow(xdisplay, xid, 0, 0, w as c_uint, h as c_uint);
    }
    XRaiseWindow(xdisplay, xid);
    XFlush(xdisplay);
}

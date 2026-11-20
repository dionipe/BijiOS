//! BijiOS 1.0.0 "Padi" — Official Brand Identity + macOS Liquid Glass GTK4 Theme
//! Brand Palette:
//!   Hijau Tunas (Primary):   #2F7D4F
//!   Hijau Muda (Support):    #7FBF6A
//!   Krem Tanah (Warm Light): #F4EBD9
//!   Emas Benih (Gold Accent):#E8A33D
//!   Hutan Malam (Deep Dark): #12332A

pub const BIJIOS_CSS: &str = r#"
/* =========================================================================
   BijiOS 1.0.0 "Padi" — macOS Liquid Glass Native GTK4 Theme
   ========================================================================= */

window {
    background-color: #12332A;
    color: #12332A;
    font-family: "DejaVu Sans", "Liberation Sans", sans-serif;
    font-size: 13px;
}

/* -------------------------------------------------------------------------
   Atmospheric Wallpapers (BijiOS Padi Official Default + Golden Gate / Tahoe / Midnight)
   ------------------------------------------------------------------------- */
window.bijios-desktop-bg,
window.bijios-wallpaper-padi {
    background-image:
        radial-gradient(circle at 18% 82%, rgba(232, 163, 61, 0.56) 0%, rgba(232, 163, 61, 0.18) 28%, transparent 56%),
        radial-gradient(circle at 82% 20%, rgba(127, 191, 106, 0.48) 0%, rgba(47, 125, 79, 0.24) 32%, transparent 62%),
        radial-gradient(circle at 48% 50%, rgba(47, 125, 79, 0.44) 0%, transparent 58%),
        linear-gradient(150deg, #0b221c 0%, #12332A 38%, #1e4d38 72%, #2F7D4F 100%);
}

window.bijios-wallpaper-goldengate {
    background-image:
        radial-gradient(circle at 22% 76%, rgba(232, 163, 61, 0.58) 0%, rgba(234, 88, 12, 0.25) 26%, transparent 56%),
        radial-gradient(circle at 78% 24%, rgba(56, 189, 248, 0.42) 0%, rgba(99, 102, 241, 0.22) 32%, transparent 60%),
        radial-gradient(circle at 52% 58%, rgba(225, 29, 72, 0.34) 0%, transparent 52%),
        linear-gradient(155deg, #0b1338 0%, #1e1b52 34%, #5b2149 68%, #9a3412 100%);
}

window.bijios-wallpaper-tahoe {
    background-image:
        radial-gradient(circle at 18% 22%, rgba(56, 189, 248, 0.48) 0%, transparent 48%),
        radial-gradient(circle at 82% 28%, rgba(168, 85, 247, 0.42) 0%, transparent 48%),
        radial-gradient(circle at 50% 82%, rgba(127, 191, 106, 0.34) 0%, transparent 52%),
        linear-gradient(145deg, #070b18 0%, #101b3d 45%, #1e1b4b 100%);
}

window.bijios-wallpaper-midnight {
    background-image:
        radial-gradient(circle at 28% 22%, rgba(47, 125, 79, 0.30) 0%, transparent 52%),
        radial-gradient(circle at 78% 80%, rgba(232, 163, 61, 0.22) 0%, transparent 50%),
        linear-gradient(145deg, #071310 0%, #0d211b 55%, #12332A 100%);
}

/* -------------------------------------------------------------------------
   macOS Tahoe Liquid Glass Login Screen (BijiOS Identity)
   ------------------------------------------------------------------------- */
window.bijios-login-window {
    background-image:
        radial-gradient(circle at 20% 80%, rgba(232, 163, 61, 0.52) 0%, rgba(232, 163, 61, 0.16) 28%, transparent 56%),
        radial-gradient(circle at 80% 22%, rgba(127, 191, 106, 0.46) 0%, rgba(47, 125, 79, 0.24) 34%, transparent 62%),
        linear-gradient(150deg, #0a1f19 0%, #12332A 42%, #1d4e38 78%, #2F7D4F 100%);
}

.tahoe-glass-pill {
    background-image: linear-gradient(180deg, rgba(255, 255, 255, 0.24) 0%, rgba(255, 255, 255, 0.10) 100%);
    border: 1px solid rgba(255, 255, 255, 0.34);
    border-top: 1px solid rgba(255, 255, 255, 0.58);
    border-radius: 999px;
    padding: 5px 15px;
    font-size: 12.5px;
    font-weight: 600;
    color: #ffffff;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.24), inset 0 1px 0 rgba(255, 255, 255, 0.45);
}

.login-clock {
    font-size: 80px;
    font-weight: 800;
    color: #ffffff;
    letter-spacing: -2px;
    text-shadow: 0 6px 24px rgba(0, 0, 0, 0.35);
}

.login-date {
    font-size: 18px;
    font-weight: 500;
    color: rgba(244, 235, 217, 0.94);
    text-shadow: 0 2px 12px rgba(0, 0, 0, 0.3);
}

.login-card {
    background-image: linear-gradient(
        160deg,
        rgba(255, 255, 255, 0.24) 0%,
        rgba(255, 255, 255, 0.11) 48%,
        rgba(18, 51, 42, 0.46) 100%
    );
    background-color: rgba(18, 51, 42, 0.38);
    border: 1px solid rgba(255, 255, 255, 0.36);
    border-top: 1px solid rgba(255, 255, 255, 0.65);
    border-radius: 28px;
    padding: 28px 32px;
    box-shadow:
        0 34px 80px rgba(0, 0, 0, 0.48),
        inset 0 1px 1px rgba(255, 255, 255, 0.56);
}

.login-avatar-ring {
    background-image: linear-gradient(145deg, rgba(244, 235, 217, 0.96), rgba(244, 235, 217, 0.82));
    border: 2px solid rgba(255, 255, 255, 0.8);
    border-radius: 999px;
    min-width: 84px;
    min-height: 84px;
    padding: 14px;
    box-shadow: 0 12px 28px rgba(0, 0, 0, 0.32);
}

.login-title {
    font-size: 20px;
    font-weight: 700;
    color: #ffffff;
}

.login-badge {
    font-size: 11.5px;
    font-weight: 600;
    color: #F4EBD9;
    background-color: rgba(47, 125, 79, 0.55);
    border: 1px solid rgba(127, 191, 106, 0.55);
    border-radius: 999px;
    padding: 4px 14px;
}

.login-subtitle {
    font-size: 12px;
    color: rgba(244, 235, 217, 0.86);
}

entry.login-entry {
    background-image: none;
    background-color: rgba(255, 255, 255, 0.18);
    color: #ffffff;
    caret-color: #ffffff;
    border: 1px solid rgba(255, 255, 255, 0.38);
    border-radius: 999px;
    padding: 8px 18px;
    min-height: 40px;
    font-size: 14px;
    box-shadow: inset 0 2px 6px rgba(0, 0, 0, 0.16);
}

entry.login-entry:focus {
    border-color: #7FBF6A;
    background-color: rgba(255, 255, 255, 0.26);
    box-shadow: 0 0 0 3px rgba(127, 191, 106, 0.42);
}

button.login-submit-btn {
    background-image: linear-gradient(180deg, #E8A33D 0%, #D98A1E 100%);
    color: #12332A;
    border: 1px solid rgba(255, 255, 255, 0.65);
    border-radius: 999px;
    min-width: 42px;
    min-height: 42px;
    padding: 0;
    font-size: 16px;
    font-weight: 800;
    box-shadow: 0 6px 18px rgba(0, 0, 0, 0.28);
}

button.login-submit-btn:hover {
    background-image: linear-gradient(180deg, #F3B95F 0%, #E8A33D 100%);
}

.login-error {
    color: #fee2e2;
    background-color: rgba(220, 38, 38, 0.42);
    border: 1px solid rgba(248, 113, 113, 0.7);
    border-radius: 12px;
    padding: 8px 14px;
    font-size: 12px;
    font-weight: 600;
}

button.login-power-circle {
    background-image: linear-gradient(180deg, rgba(255, 255, 255, 0.24) 0%, rgba(255, 255, 255, 0.10) 100%);
    color: #ffffff;
    border: 1px solid rgba(255, 255, 255, 0.36);
    border-top: 1px solid rgba(255, 255, 255, 0.6);
    border-radius: 999px;
    min-width: 46px;
    min-height: 46px;
    padding: 0;
    font-size: 18px;
    box-shadow: 0 8px 22px rgba(0, 0, 0, 0.28), inset 0 1px 0 rgba(255, 255, 255, 0.45);
}

button.login-power-circle:hover {
    background-image: linear-gradient(180deg, rgba(127, 191, 106, 0.42) 0%, rgba(47, 125, 79, 0.42) 100%);
    border-color: rgba(255, 255, 255, 0.8);
}

.login-power-caption {
    font-size: 11px;
    font-weight: 600;
    color: rgba(244, 235, 217, 0.9);
}

/* -------------------------------------------------------------------------
   Top Menu Bar (Sleek macOS Proportions — 26px Translucent Liquid Glass)
   ------------------------------------------------------------------------- */
.bijios-menubar {
    background-image: linear-gradient(180deg, rgba(18, 51, 42, 0.74) 0%, rgba(18, 51, 42, 0.58) 100%);
    border-bottom: 1px solid rgba(255, 255, 255, 0.2);
    padding: 2px 12px;
    min-height: 26px;
    box-shadow: 0 2px 12px rgba(0, 0, 0, 0.18);
}

.bijios-menubar button,
.bijios-menubar menubutton > button {
    background-image: none;
    background-color: transparent;
    border: none;
    color: #ffffff;
    padding: 1px 8px;
    border-radius: 6px;
    font-size: 12px;
    font-weight: 600;
    box-shadow: none;
    min-height: 22px;
}

.bijios-menubar button:hover,
.bijios-menubar menubutton > button:hover {
    background-color: rgba(255, 255, 255, 0.18);
}

.dynamic-island {
    background-image: linear-gradient(180deg, rgba(10, 28, 23, 0.90) 0%, rgba(18, 51, 42, 0.84) 100%);
    border: 1px solid rgba(127, 191, 106, 0.36);
    border-top: 1px solid rgba(232, 163, 61, 0.5);
    border-radius: 999px;
    padding: 1px 12px;
    font-size: 11px;
    font-weight: 700;
    color: #F4EBD9;
    box-shadow: 0 2px 10px rgba(0, 0, 0, 0.3);
}

.menubar-pill {
    background-color: rgba(255, 255, 255, 0.13);
    border: 1px solid rgba(255, 255, 255, 0.22);
    border-radius: 999px;
    padding: 1px 8px;
    font-size: 11px;
    font-weight: 600;
    color: #ffffff;
}

.menubar-clock {
    font-size: 11.5px;
    font-weight: 700;
    color: #ffffff;
    padding: 0 4px;
}

/* -------------------------------------------------------------------------
   Liquid Glass Desktop Widgets (Compact & Airy)
   ------------------------------------------------------------------------- */
.glass-card {
    background-image: linear-gradient(
        155deg,
        rgba(255, 255, 255, 0.22) 0%,
        rgba(255, 255, 255, 0.09) 48%,
        rgba(18, 51, 42, 0.34) 100%
    );
    background-color: rgba(18, 51, 42, 0.24);
    border: 1px solid rgba(255, 255, 255, 0.30);
    border-top: 1px solid rgba(255, 255, 255, 0.58);
    border-radius: 18px;
    padding: 14px 18px;
    box-shadow:
        0 14px 34px rgba(0, 0, 0, 0.28),
        inset 0 1px 1px rgba(255, 255, 255, 0.48);
}

.sticky-note-card {
    background-image: linear-gradient(
        155deg,
        rgba(244, 235, 217, 0.32) 0%,
        rgba(232, 163, 61, 0.22) 55%,
        rgba(120, 68, 12, 0.30) 100%
    );
    border: 1px solid rgba(244, 235, 217, 0.42);
    border-top: 1px solid rgba(255, 250, 235, 0.7);
    border-radius: 18px;
    padding: 14px 16px;
    box-shadow:
        0 14px 34px rgba(0, 0, 0, 0.26),
        inset 0 1px 1px rgba(255, 250, 235, 0.48);
}

.sticky-note-view {
    background-color: transparent;
    color: #F4EBD9;
    font-size: 12px;
}

.widget-clock-big {
    font-size: 40px;
    font-weight: 800;
    color: #ffffff;
    letter-spacing: -1px;
    text-shadow: 0 3px 12px rgba(0, 0, 0, 0.26);
}

.widget-temp-big {
    font-size: 24px;
    font-weight: 800;
    color: #ffffff;
}

.widget-title {
    font-size: 10.5px;
    font-weight: 800;
    color: #7FBF6A;
    letter-spacing: 0.7px;
}

.widget-title-amber {
    font-size: 10.5px;
    font-weight: 800;
    color: #F4EBD9;
    letter-spacing: 0.7px;
}

.widget-sub {
    font-size: 11.5px;
    color: rgba(248, 250, 252, 0.88);
}

.bijios-desktop-bg progressbar trough {
    background-color: rgba(255, 255, 255, 0.2);
    border-radius: 999px;
    min-height: 6px;
}

.bijios-desktop-bg progressbar progress {
    background-image: linear-gradient(90deg, #7FBF6A, #E8A33D);
    border-radius: 999px;
    min-height: 6px;
}

/* -------------------------------------------------------------------------
   Bottom Floating Liquid Glass Dock (Sleek, Compact & Proportional)
   ------------------------------------------------------------------------- */
.bijios-dock {
    background-image: linear-gradient(
        180deg,
        rgba(255, 255, 255, 0.28) 0%,
        rgba(255, 255, 255, 0.14) 50%,
        rgba(18, 51, 42, 0.34) 100%
    );
    background-color: rgba(18, 51, 42, 0.30);
    border: 1px solid rgba(255, 255, 255, 0.38);
    border-top: 1px solid rgba(255, 255, 255, 0.65);
    border-radius: 18px;
    padding: 5px 10px;
    box-shadow:
        0 16px 40px rgba(0, 0, 0, 0.42),
        inset 0 1px 1px rgba(255, 255, 255, 0.6);
}

button.dock-btn {
    border: 1px solid rgba(255, 255, 255, 0.38);
    border-top: 1px solid rgba(255, 255, 255, 0.68);
    border-radius: 11px;
    min-width: 40px;
    min-height: 40px;
    padding: 2px;
    font-size: 19px;
    color: #ffffff;
    box-shadow: 0 5px 12px rgba(0, 0, 0, 0.26), inset 0 1px 1px rgba(255, 255, 255, 0.5);
}

button.dock-btn:hover {
    border-color: #ffffff;
    box-shadow: 0 8px 18px rgba(127, 191, 106, 0.45), inset 0 1px 2px rgba(255, 255, 255, 0.85);
}

.icon-finder     { background-image: linear-gradient(145deg, #38bdf8 0%, #2563eb 100%); }
.icon-launchpad  { background-image: linear-gradient(145deg, #7FBF6A 0%, #2F7D4F 55%, #12332A 100%); }
.icon-bibit      { background-image: linear-gradient(145deg, #E8A33D 0%, #2F7D4F 100%); }
.icon-pancer     { background-image: linear-gradient(145deg, #1E4D38 0%, #0D241E 100%); }
.icon-wordcraft  { background-image: linear-gradient(145deg, #60a5fa 0%, #1d4ed8 100%); }
.icon-gridcraft  { background-image: linear-gradient(145deg, #7FBF6A 0%, #2F7D4F 100%); }
.icon-deckcraft  { background-image: linear-gradient(145deg, #fb923c 0%, #c2410c 100%); }
.icon-pdfcraft   { background-image: linear-gradient(145deg, #f87171 0%, #b91c1c 100%); }
.icon-notes      { background-image: linear-gradient(145deg, #F3B95F 0%, #E8A33D 100%); }
.icon-calculator { background-image: linear-gradient(145deg, #fb923c 0%, #ea580c 100%); }
.icon-activity   { background-image: linear-gradient(145deg, #2dd4bf 0%, #0f766e 100%); }
.icon-settings   { background-image: linear-gradient(145deg, #94a3b8 0%, #475569 100%); }
.icon-about      { background-image: linear-gradient(145deg, #1b4b3d 0%, #12332A 100%); }
.icon-lock       { background-image: linear-gradient(145deg, #f87171 0%, #be123c 100%); }

/* -------------------------------------------------------------------------
   macOS Liquid Glass Light Application Windows (True ARGB Rounded Corners)
   ------------------------------------------------------------------------- */
window.app-window {
    background-image: none;
    background-color: #FFFFFF;
    color: #12332A;
    border: none;
    border-radius: 0px;
    padding: 0px;
    margin: 0px;
    box-shadow: none;
}

/* Custom macOS Liquid Glass Titlebar (Ultra-Compact, Minimal Top Padding, Fully Rounded) */
.mac-titlebar {
    background-image: linear-gradient(180deg, #163A30 0%, #102B23 100%);
    border-bottom: 1px solid #0B1F19;
    border-top-left-radius: 13px;
    border-top-right-radius: 13px;
    padding: 2px 10px 2px 10px;
    min-height: 28px;
}

.mac-title-tab {
    background-image: linear-gradient(180deg, rgba(244, 235, 217, 0.16) 0%, rgba(244, 235, 217, 0.09) 100%);
    border: 1px solid rgba(244, 235, 217, 0.18);
    border-radius: 999px;
    padding: 2px 12px;
    margin: 0px;
}

.mac-titlebar-title {
    font-size: 11.5px;
    font-weight: 600;
    color: #F4EBD9;
}

.mac-titlebar-brand {
    font-size: 10.5px;
    font-weight: 600;
    color: rgba(244, 235, 217, 0.65);
}

button.mac-title-pill-btn {
    background-image: none;
    background-color: rgba(244, 235, 217, 0.12);
    border: 1px solid rgba(244, 235, 217, 0.16);
    color: #E8A33D;
    border-radius: 999px;
    min-width: 20px;
    min-height: 20px;
    padding: 0 5px;
    font-size: 10px;
    font-weight: 700;
    margin: 0px;
    box-shadow: none;
}

button.mac-title-pill-btn:hover {
    background-color: rgba(244, 235, 217, 0.22);
}

button.traffic-btn {
    min-width: 11px;
    min-height: 11px;
    padding: 0;
    margin: 0px;
    border-radius: 999px;
    border: 1px solid rgba(255, 255, 255, 0.24);
    box-shadow: inset 0 1px 2px rgba(255, 255, 255, 0.55), 0 1px 2px rgba(0, 0, 0, 0.42);
}

button.traffic-close {
    background-image: radial-gradient(circle at 35% 30%, #FF8A80 0%, #FF5F56 55%, #D9382E 100%);
}
button.traffic-min {
    background-image: radial-gradient(circle at 35% 30%, #FFE082 0%, #FFBD2E 55%, #D49415 100%);
}
button.traffic-max {
    background-image: radial-gradient(circle at 35% 30%, #69F0AE 0%, #27C93F 55%, #179A2C 100%);
}

button.traffic-close:hover {
    background-image: radial-gradient(circle at 35% 30%, #FFABA3 0%, #FF453A 100%);
}
button.traffic-min:hover {
    background-image: radial-gradient(circle at 35% 30%, #FFECB3 0%, #FFB300 100%);
}
button.traffic-max:hover {
    background-image: radial-gradient(circle at 35% 30%, #B9F6CA 0%, #30D158 100%);
}

/* Sidebar (Clean macOS Light Frosted Sidebar) */
.app-sidebar {
    background-image: linear-gradient(180deg, #F6F7F9 0%, #EEF2F5 100%);
    border-right: 1px solid #E2E8F0;
    padding: 12px 10px;
    min-width: 196px;
}

.sidebar-section-hdr {
    font-size: 10.5px;
    font-weight: 700;
    color: #64748B;
    letter-spacing: 0.6px;
    padding: 8px 10px 4px 10px;
}

button.sidebar-btn {
    background-image: none;
    background-color: transparent;
    border: none;
    color: #1E293B;
    border-radius: 8px;
    padding: 7px 10px;
    font-size: 13px;
    font-weight: 500;
    box-shadow: none;
}

button.sidebar-btn:hover {
    background-color: rgba(47, 125, 79, 0.10);
    color: #12332A;
}

button.sidebar-btn-active {
    background-image: linear-gradient(135deg, #2F7D4F 0%, #24643E 100%);
    border: none;
    color: #FFFFFF;
    border-radius: 8px;
    padding: 7px 10px;
    font-size: 13px;
    font-weight: 600;
    box-shadow: 0 2px 8px rgba(47, 125, 79, 0.28);
}

/* App Toolbar (Ultra-Compact, Minimal Padding, Pill Rounded) */
.app-toolbar {
    background-color: #FFFFFF;
    border-bottom: 1px solid #E2E8F0;
    padding: 4px 10px;
    min-height: 30px;
}

button.toolbar-btn {
    background-image: none;
    background-color: #F8FAFC;
    border: 1px solid #E2E8F0;
    color: #1E293B;
    border-radius: 999px;
    padding: 2px 10px;
    min-height: 24px;
    font-size: 11.5px;
    font-weight: 600;
    box-shadow: none;
}

button.toolbar-btn:hover {
    background-color: #EDF2F7;
    border-color: #CBD5E1;
}

button.primary-btn {
    background-image: linear-gradient(135deg, #2F7D4F 0%, #23613C 100%);
    color: #FFFFFF;
    border: 1px solid #23613C;
    border-radius: 999px;
    padding: 2px 12px;
    min-height: 24px;
    font-size: 11.5px;
    font-weight: 700;
    box-shadow: 0 2px 6px rgba(47, 125, 79, 0.22);
}

button.primary-btn:hover {
    background-image: linear-gradient(135deg, #3A945E 0%, #2F7D4F 100%);
}

button.accent-gold-btn {
    background-image: linear-gradient(135deg, #E8A33D 0%, #D48C26 100%);
    color: #12332A;
    border: 1px solid #D48C26;
    border-radius: 8px;
    padding: 6px 14px;
    font-size: 12.5px;
    font-weight: 700;
    box-shadow: none;
}

button.danger-btn {
    background-image: none;
    background-color: #FEF2F2;
    color: #DC2626;
    border: 1px solid #FECACA;
    border-radius: 8px;
    padding: 5px 10px;
    font-size: 12px;
    font-weight: 600;
    box-shadow: none;
}

button.danger-btn:hover {
    background-color: #FEE2E2;
    border-color: #F87171;
}

button.row-trash-btn {
    background-image: none;
    background-color: transparent;
    border: none;
    color: #94A3B8;
    border-radius: 6px;
    padding: 2px 7px;
    min-height: 24px;
    box-shadow: none;
}

button.row-trash-btn:hover {
    background-color: #FEE2E2;
    color: #DC2626;
}

/* Clean Light macOS Entries inside App Windows (Pill Rounded) */
.app-window entry {
    background-image: none;
    background-color: #F8FAFC;
    color: #12332A;
    border: 1px solid #CBD5E1;
    border-radius: 999px;
    padding: 1px 12px;
    min-height: 24px;
    font-size: 12px;
}

.app-window entry:focus {
    background-color: #FFFFFF;
    border-color: #2F7D4F;
    box-shadow: 0 0 0 2px rgba(47, 125, 79, 0.22);
}

/* Finder Table Header, Zebra Rows & Statusbar (Matching ekspektasi-1.png) */
.finder-col-header {
    background-color: #F8FAFC;
    border-bottom: 1px solid #E2E8F0;
    padding: 6px 16px;
}

.finder-col-lbl {
    font-size: 11.5px;
    font-weight: 600;
    color: #64748B;
}

.finder-row-even {
    background-color: #FFFFFF;
    padding: 3px 12px;
    min-height: 34px;
}

.finder-row-odd {
    background-color: #F8FAFC;
    padding: 3px 12px;
    min-height: 34px;
}

.finder-row-even:hover,
.finder-row-odd:hover {
    background-color: rgba(47, 125, 79, 0.08);
}

button.finder-name-btn {
    background-image: none;
    background-color: transparent;
    border: none;
    color: #12332A;
    padding: 2px 4px;
    font-size: 13px;
    font-weight: 500;
    box-shadow: none;
}

.finder-meta-lbl {
    font-size: 12px;
    color: #64748B;
}

.file-badge {
    min-width: 22px;
    min-height: 22px;
    border-radius: 5px;
    font-size: 10.5px;
    font-weight: 800;
    color: #FFFFFF;
    padding: 0 4px;
}

.badge-folder { background-color: #38BDF8; color: #FFFFFF; }
.badge-docx   { background-color: #2563EB; color: #FFFFFF; }
.badge-xlsx   { background-color: #2F7D4F; color: #FFFFFF; }
.badge-pdf    { background-color: #DC2626; color: #FFFFFF; }
.badge-img    { background-color: #8B5CF6; color: #FFFFFF; }
.badge-file   { background-color: #64748B; color: #FFFFFF; }

.finder-statusbar {
    background-color: #F8FAFC;
    border-top: 1px solid #E2E8F0;
    padding: 6px 16px;
    font-size: 11.5px;
    color: #64748B;
}

/* App Cards (Settings / About / Activity Monitor) */
.app-card {
    background-color: #F8FAFC;
    border: 1px solid #E2E8F0;
    border-radius: 14px;
    padding: 14px 18px;
}

.app-heading {
    font-size: 18px;
    font-weight: 700;
    color: #12332A;
}

.app-section-title {
    font-size: 12px;
    font-weight: 800;
    color: #2F7D4F;
    letter-spacing: 0.4px;
}

.app-muted {
    font-size: 12.5px;
    color: #475569;
}

/* Launchpad Proportional Squircle Tiles (Light Frosted Glass) */
button.launchpad-tile {
    background-image: linear-gradient(180deg, #FFFFFF 0%, #F8FAFC 100%);
    border: 1px solid #E2E8F0;
    border-radius: 18px;
    padding: 14px 10px;
    min-width: 132px;
    min-height: 112px;
    color: #12332A;
    box-shadow: 0 4px 12px rgba(18, 51, 42, 0.06);
}

button.launchpad-tile:hover {
    background-image: linear-gradient(180deg, #F4EBD9 0%, #FFFFFF 100%);
    border-color: #2F7D4F;
    box-shadow: 0 8px 20px rgba(47, 125, 79, 0.16);
}

.launchpad-icon-badge {
    min-width: 52px;
    min-height: 52px;
    border-radius: 14px;
    font-size: 26px;
    color: #FFFFFF;
    box-shadow: 0 6px 14px rgba(0, 0, 0, 0.16);
}

.launchpad-caption {
    font-size: 12.5px;
    font-weight: 600;
    color: #12332A;
}

/* Document / Notes / Process Editor */
.editor-view {
    background-color: #FFFFFF;
    color: #12332A;
    font-size: 14px;
    padding: 16px;
}

/* Calculator */
.calc-display {
    background-color: #12332A;
    color: #F4EBD9;
    font-size: 36px;
    font-weight: 700;
    padding: 18px 16px;
    border-radius: 14px;
}

button.calc-btn {
    background-image: none;
    background-color: #F1F5F9;
    color: #12332A;
    border: 1px solid #E2E8F0;
    border-radius: 12px;
    min-height: 46px;
    font-size: 17px;
    font-weight: 600;
    box-shadow: none;
}

button.calc-btn:hover {
    background-color: #E2E8F0;
}

button.calc-op {
    background-image: linear-gradient(145deg, #E8A33D 0%, #D48C26 100%);
    color: #12332A;
    border-color: #D48C26;
}

button.calc-op:hover {
    background-image: linear-gradient(145deg, #F3B95F 0%, #E8A33D 100%);
}

/* Pancer - BijiOS Native Terminal (Menu Bar, Dynamic Host Tab Bar & Split Panes) */
.pancer-menubar {
    background-color: #0D241E;
    border-bottom: 1px solid rgba(127, 191, 106, 0.18);
    padding: 2px 8px;
    min-height: 26px;
}

button.pancer-menu-btn {
    background-image: none;
    background-color: transparent;
    border: none;
    color: #F4EBD9;
    border-radius: 6px;
    padding: 2px 10px;
    min-height: 22px;
    font-size: 12px;
    font-weight: 600;
    box-shadow: none;
}

button.pancer-menu-btn:hover {
    background-color: rgba(127, 191, 106, 0.22);
    color: #FFFFFF;
}

.pancer-dropdown {
    background-image: linear-gradient(180deg, #133128 0%, #0C211B 100%);
    border: 1px solid rgba(127, 191, 106, 0.36);
    border-radius: 10px;
    padding: 5px;
    box-shadow: 0 12px 28px rgba(0, 0, 0, 0.48);
}

button.pancer-menu-item {
    background-image: none;
    background-color: transparent;
    border: none;
    color: #F4EBD9;
    border-radius: 7px;
    padding: 5px 10px;
    min-height: 24px;
    font-size: 12px;
    font-weight: 500;
    box-shadow: none;
}

button.pancer-menu-item:hover {
    background-color: #2F7D4F;
    color: #FFFFFF;
}

.pancer-accel {
    font-family: "DejaVu Sans Mono", "Liberation Mono", monospace;
    font-size: 10.5px;
    font-weight: 600;
    color: rgba(244, 235, 217, 0.58);
}

button.pancer-menu-backdrop {
    background-image: none;
    background-color: transparent;
    border: none;
    border-radius: 0px;
    box-shadow: none;
    padding: 0px;
}

button.pancer-menu-backdrop:hover {
    background-color: transparent;
}

.pancer-tabbar {
    background-color: #091814;
    border-bottom: 1px solid rgba(127, 191, 106, 0.22);
    padding: 3px 8px 0px 8px;
    min-height: 28px;
}

.pancer-tab {
    background-color: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.10);
    border-bottom: none;
    border-top-left-radius: 8px;
    border-top-right-radius: 8px;
    padding: 2px 6px 2px 10px;
}

.pancer-tab-active {
    background-image: linear-gradient(180deg, #173D31 0%, #0E2720 100%);
    border: 1px solid rgba(127, 191, 106, 0.42);
    border-bottom: none;
    border-top-left-radius: 8px;
    border-top-right-radius: 8px;
    padding: 2px 6px 2px 10px;
}

button.pancer-tab-btn {
    background-image: none;
    background-color: transparent;
    border: none;
    color: #F4EBD9;
    padding: 1px 4px;
    min-height: 22px;
    font-size: 11.5px;
    font-weight: 600;
    box-shadow: none;
}

button.pancer-tab-close {
    background-image: none;
    background-color: transparent;
    border: none;
    color: rgba(244, 235, 217, 0.55);
    border-radius: 999px;
    padding: 0 5px;
    min-height: 18px;
    min-width: 18px;
    font-size: 11px;
    box-shadow: none;
}

button.pancer-tab-close:hover {
    background-color: rgba(239, 68, 68, 0.35);
    color: #FFFFFF;
}

button.pancer-add-tab {
    background-image: none;
    background-color: rgba(127, 191, 106, 0.16);
    border: 1px solid rgba(127, 191, 106, 0.32);
    color: #E8A33D;
    border-radius: 999px;
    min-width: 24px;
    min-height: 22px;
    padding: 0 8px;
    margin-bottom: 3px;
    font-size: 14px;
    font-weight: 800;
    box-shadow: none;
}

button.pancer-add-tab:hover {
    background-color: #2F7D4F;
    color: #FFFFFF;
}

paned > separator {
    background-color: rgba(127, 191, 106, 0.35);
    min-width: 2px;
    min-height: 2px;
}

.pancer-cwd-pill {
    background-color: rgba(127, 191, 106, 0.14);
    border: 1px solid rgba(127, 191, 106, 0.28);
    border-radius: 999px;
    padding: 2px 10px;
    font-size: 11.5px;
    font-weight: 600;
    color: #F4EBD9;
}

button.pancer-btn {
    background-image: none;
    background-color: rgba(255, 255, 255, 0.08);
    border: 1px solid rgba(255, 255, 255, 0.16);
    color: #F4EBD9;
    border-radius: 999px;
    padding: 2px 10px;
    min-height: 22px;
    font-size: 11px;
    font-weight: 600;
    box-shadow: none;
}

button.pancer-btn:hover {
    background-color: rgba(127, 191, 106, 0.24);
    border-color: #7FBF6A;
}

.pancer-view {
    background-color: #091814;
    color: #F4EBD9;
    font-family: "DejaVu Sans Mono", "Liberation Mono", monospace;
    font-size: 13px;
    padding: 12px 14px;
}

.pancer-prompt-bar {
    background-color: #0D221C;
    border-top: 1px solid rgba(127, 191, 106, 0.24);
    padding: 6px 12px;
}

.pancer-prompt-lbl {
    font-family: "DejaVu Sans Mono", "Liberation Mono", monospace;
    font-size: 12.5px;
    font-weight: 700;
    color: #7FBF6A;
}

entry.pancer-entry {
    background-image: none;
    background-color: #091814;
    color: #F4EBD9;
    caret-color: #E8A33D;
    border: 1px solid rgba(127, 191, 106, 0.34);
    border-radius: 999px;
    padding: 2px 12px;
    min-height: 26px;
    font-family: "DejaVu Sans Mono", "Liberation Mono", monospace;
    font-size: 12.5px;
}

entry.pancer-entry:focus {
    border-color: #E8A33D;
    box-shadow: 0 0 0 2px rgba(232, 163, 61, 0.28);
}

/* Settings Sliders, Switches & Segmented Pills (BijiOS Hijau Tunas) */
.app-window scale trough {
    background-color: #E2E8F0;
    border-radius: 999px;
    min-height: 6px;
}

.app-window scale highlight {
    background-image: linear-gradient(90deg, #2F7D4F, #7FBF6A);
    border-radius: 999px;
    min-height: 6px;
}

.app-window scale slider {
    background-color: #FFFFFF;
    border: 2px solid #2F7D4F;
    border-radius: 999px;
    min-width: 16px;
    min-height: 16px;
    box-shadow: 0 2px 5px rgba(0, 0, 0, 0.18);
}

.app-window switch {
    background-color: #CBD5E1;
    border-radius: 999px;
    min-width: 40px;
    min-height: 22px;
}

.app-window switch:checked {
    background-color: #2F7D4F;
}

.app-window switch slider {
    background-color: #FFFFFF;
    border-radius: 999px;
    min-width: 18px;
    min-height: 18px;
    margin: 2px;
}

button.seg-btn {
    background-image: none;
    background-color: #F1F5F9;
    color: #334155;
    border: 1px solid #CBD5E1;
    border-radius: 999px;
    padding: 3px 12px;
    min-height: 24px;
    font-size: 11.5px;
    font-weight: 600;
    box-shadow: none;
}

button.seg-btn:hover {
    background-color: #E2E8F0;
}

button.seg-btn-active {
    background-image: linear-gradient(135deg, #2F7D4F 0%, #23613C 100%);
    color: #FFFFFF;
    border: 1px solid #23613C;
    border-radius: 999px;
    padding: 3px 12px;
    min-height: 24px;
    font-size: 11.5px;
    font-weight: 700;
    box-shadow: 0 2px 6px rgba(47, 125, 79, 0.25);
}

button.wp-tile {
    background-image: none;
    background-color: #FFFFFF;
    border: 2px solid #E2E8F0;
    border-radius: 14px;
    padding: 6px;
    color: #12332A;
    box-shadow: 0 2px 8px rgba(18, 51, 42, 0.06);
}

button.wp-tile:hover {
    border-color: #7FBF6A;
}

button.wp-tile-active {
    background-image: none;
    background-color: #F4EBD9;
    border: 2px solid #2F7D4F;
    border-radius: 14px;
    padding: 6px;
    color: #12332A;
    box-shadow: 0 4px 14px rgba(47, 125, 79, 0.22);
}

.wp-preview-img {
    border-radius: 9px;
}

.wp-badge {
    background-color: #2F7D4F;
    color: #FFFFFF;
    border-radius: 999px;
    padding: 1px 8px;
    font-size: 10px;
    font-weight: 700;
}

.user-avatar-circle {
    background-image: linear-gradient(135deg, #2F7D4F 0%, #7FBF6A 100%);
    color: #FFFFFF;
    border-radius: 999px;
    min-width: 44px;
    min-height: 44px;
    font-size: 18px;
    font-weight: 800;
}

/* Custom User Profile Avatars (Circular Photo + Curated Gradient Presets) */
.avatar-photo-frame {
    border-radius: 999px;
    border: 2px solid rgba(47, 125, 79, 0.35);
    background-color: #12332A;
}

.avatar-circle-sm {
    border-radius: 999px;
    min-width: 18px;
    min-height: 18px;
    font-size: 10px;
    font-weight: 800;
    color: #FFFFFF;
}

.avatar-circle-md {
    border-radius: 999px;
    min-width: 56px;
    min-height: 56px;
    font-size: 24px;
    font-weight: 800;
    color: #FFFFFF;
    box-shadow: 0 4px 12px rgba(18, 51, 42, 0.16);
}

.avatar-circle-lg {
    border-radius: 999px;
    min-width: 76px;
    min-height: 76px;
    font-size: 32px;
    font-weight: 800;
    color: #FFFFFF;
    box-shadow: 0 6px 18px rgba(18, 51, 42, 0.22);
}

.avatar-grad-green { background-image: linear-gradient(135deg, #2F7D4F 0%, #7FBF6A 100%); }
.avatar-grad-gold { background-image: linear-gradient(135deg, #D97706 0%, #F59E0B 100%); }
.avatar-grad-ocean { background-image: linear-gradient(135deg, #0284C7 0%, #38BDF8 100%); }
.avatar-grad-indigo { background-image: linear-gradient(135deg, #4F46E5 0%, #818CF8 100%); }
.avatar-grad-rose { background-image: linear-gradient(135deg, #E11D48 0%, #FB7185 100%); }
.avatar-grad-teal { background-image: linear-gradient(135deg, #0D9488 0%, #2DD4BF 100%); }
.avatar-grad-forest { background-image: linear-gradient(135deg, #12332A 0%, #2F7D4F 100%); }
.avatar-grad-sunset { background-image: linear-gradient(135deg, #EA580C 0%, #E8A33D 100%); }
.avatar-grad-plum { background-image: linear-gradient(135deg, #7C3AED 0%, #C084FC 100%); }
.avatar-grad-slate { background-image: linear-gradient(135deg, #334155 0%, #64748B 100%); }

button.avatar-pick-btn {
    background-image: none;
    background-color: #FFFFFF;
    border: 2px solid #E2E8F0;
    border-radius: 14px;
    padding: 8px;
    color: #12332A;
    box-shadow: none;
}

button.avatar-pick-btn:hover {
    border-color: #7FBF6A;
    background-color: #F8FAF9;
}

button.avatar-pick-btn-active {
    background-image: none;
    background-color: #F4EBD9;
    border: 2px solid #2F7D4F;
    border-radius: 14px;
    padding: 8px;
    color: #12332A;
    box-shadow: 0 3px 10px rgba(47, 125, 79, 0.20);
}

.kv-key {
    font-size: 12px;
    font-weight: 600;
    color: #64748B;
}

.kv-val {
    font-size: 12.5px;
    font-weight: 500;
    color: #12332A;
}

button.link-btn {
    background-image: none;
    background-color: transparent;
    border: none;
    color: #2F7D4F;
    padding: 0px 2px;
    min-height: 20px;
    font-size: 12px;
    font-weight: 700;
    box-shadow: none;
}

button.link-btn:hover {
    color: #1B5E3A;
    text-decoration: underline;
}

button.link-pill {
    background-image: none;
    background-color: #FFFFFF;
    border: 1px solid #CBD5E1;
    color: #12332A;
    border-radius: 999px;
    padding: 3px 10px;
    min-height: 24px;
    font-size: 11.5px;
    font-weight: 600;
    box-shadow: none;
}

button.link-pill:hover {
    background-color: #F4EBD9;
    border-color: #2F7D4F;
    color: #2F7D4F;
}

/* In-Window Liquid Glass Dropdown Menu (GtkOverlay — Zero Black X11 Artifacts) */
.bijios-dropdown-menu {
    background-image: linear-gradient(180deg, rgba(255, 255, 255, 0.96) 0%, rgba(244, 235, 217, 0.94) 100%);
    border: 1px solid rgba(255, 255, 255, 0.72);
    border-radius: 14px;
    padding: 6px;
    box-shadow: 0 18px 42px rgba(0, 0, 0, 0.36);
}

/* ==========================================================================
   BijiOS Activity Monitor — Aesthetic Real-Time Telemetry & Process Manager
   ========================================================================== */
.act-topbar {
    background-image: linear-gradient(180deg, rgba(255, 255, 255, 0.95) 0%, rgba(244, 235, 217, 0.65) 100%);
    border-bottom: 1px solid rgba(18, 51, 42, 0.10);
    padding: 10px 18px;
}

.act-uptime-pill {
    background-color: rgba(47, 125, 79, 0.10);
    border: 1px solid rgba(47, 125, 79, 0.22);
    color: #12332A;
    border-radius: 999px;
    padding: 3px 11px;
    font-size: 11.5px;
    font-weight: 700;
}

.act-kpi-card {
    background-image: linear-gradient(180deg, #FFFFFF 0%, #FBFDFB 100%);
    border: 1px solid rgba(18, 51, 42, 0.10);
    border-radius: 14px;
    padding: 12px 14px;
    box-shadow: 0 4px 14px rgba(18, 51, 42, 0.05);
}

.act-kpi-val {
    font-size: 24px;
    font-weight: 800;
    color: #12332A;
    letter-spacing: -0.4px;
}

.act-kpi-sub {
    font-size: 11.5px;
    font-weight: 500;
    color: #64748B;
}

.act-badge-ok {
    background-color: rgba(47, 125, 79, 0.14);
    color: #1B5E3A;
    border-radius: 999px;
    padding: 2px 8px;
    font-size: 10.5px;
    font-weight: 700;
}

.act-badge-warn {
    background-color: rgba(232, 163, 61, 0.22);
    color: #9A5B06;
    border-radius: 999px;
    padding: 2px 8px;
    font-size: 10.5px;
    font-weight: 700;
}

.act-badge-high {
    background-color: rgba(225, 29, 72, 0.16);
    color: #BE123C;
    border-radius: 999px;
    padding: 2px 8px;
    font-size: 10.5px;
    font-weight: 700;
}

.act-chart-frame {
    background-color: #0D221C;
    border: 1px solid rgba(127, 191, 106, 0.24);
    border-radius: 11px;
    padding: 2px;
}

.act-legend-cpu {
    color: #2F7D4F;
    font-size: 11.5px;
    font-weight: 700;
}

.act-legend-mem {
    color: #2563EB;
    font-size: 11.5px;
    font-weight: 700;
}

.act-legend-read {
    color: #D97706;
    font-size: 11.5px;
    font-weight: 700;
}

.act-legend-write {
    color: #E11D48;
    font-size: 11.5px;
    font-weight: 700;
}

.act-legend-rx {
    color: #059669;
    font-size: 11.5px;
    font-weight: 700;
}

.act-legend-tx {
    color: #4F46E5;
    font-size: 11.5px;
    font-weight: 700;
}

.act-proc-header {
    background-color: #EEF3F0;
    border: 1px solid #D8E2DC;
    border-radius: 10px;
    padding: 7px 12px;
}

.act-proc-hdr-lbl {
    font-size: 11px;
    font-weight: 800;
    color: #334155;
    letter-spacing: 0.4px;
}

.act-proc-row {
    background-color: #FFFFFF;
    border: 1px solid rgba(18, 51, 42, 0.06);
    border-radius: 9px;
    padding: 6px 12px;
}

.act-proc-row-alt {
    background-color: #F8FAF9;
    border: 1px solid rgba(18, 51, 42, 0.06);
    border-radius: 9px;
    padding: 6px 12px;
}

.act-proc-name {
    font-size: 12.5px;
    font-weight: 700;
    color: #12332A;
}

.act-proc-mono {
    font-family: "DejaVu Sans Mono", "Liberation Mono", monospace;
    font-size: 11.5px;
    font-weight: 600;
    color: #334155;
}

.act-state-run {
    background-color: rgba(47, 125, 79, 0.16);
    color: #1B5E3A;
    border-radius: 999px;
    padding: 1px 8px;
    font-size: 10.5px;
    font-weight: 700;
}

.act-state-sleep {
    background-color: rgba(100, 116, 139, 0.14);
    color: #475569;
    border-radius: 999px;
    padding: 1px 8px;
    font-size: 10.5px;
    font-weight: 700;
}

.act-state-other {
    background-color: rgba(232, 163, 61, 0.22);
    color: #9A5B06;
    border-radius: 999px;
    padding: 1px 8px;
    font-size: 10.5px;
    font-weight: 700;
}

button.act-end-btn {
    background-image: none;
    background-color: #FEF3C7;
    border: 1px solid #F59E0B;
    color: #92400E;
    border-radius: 8px;
    padding: 2px 8px;
    min-height: 22px;
    font-size: 11px;
    font-weight: 700;
    box-shadow: none;
}

button.act-end-btn:hover {
    background-color: #F59E0B;
    color: #FFFFFF;
}

button.act-kill-btn {
    background-image: none;
    background-color: #FFE4E6;
    border: 1px solid #FB7185;
    color: #BE123C;
    border-radius: 8px;
    padding: 2px 8px;
    min-height: 22px;
    font-size: 11px;
    font-weight: 700;
    box-shadow: none;
}

button.act-kill-btn:hover {
    background-color: #E11D48;
    color: #FFFFFF;
}

progressbar.act-bar-cpu > trough > progress {
    background-image: linear-gradient(90deg, #2F7D4F, #7FBF6A);
}

progressbar.act-bar-mem > trough > progress {
    background-image: linear-gradient(90deg, #2563EB, #60A5FA);
}

progressbar.act-bar-disk > trough > progress {
    background-image: linear-gradient(90deg, #D97706, #E8A33D);
}
"#;

//! BijiOS System Settings — Sound (Hardware & Virtual PipeWire/PulseAudio) & Network (Ethernet + Wi-Fi)
//! 100% Pure Rust + Native GTK4 (Zero External Crates)

use crate::native_de::apps::create_app_window;
use crate::native_de::gtk4_ffi::*;
use crate::native_de::settings::{run_pkexec_sh, shell_quote};
use std::cell::{Cell, RefCell};
use std::fs;
use std::process::{Command, Stdio};
use std::rc::Rc;

// ============================================================================
// 1. SOUND HELPERS (PipeWire / WirePlumber / PulseAudio + Virtual Audio Sinks)
// ============================================================================

#[derive(Clone)]
struct AudioDevice {
    index: String,
    name: String,
    desc: String,
    is_virtual: bool,
    is_default: bool,
}

fn read_audio_backend_status() -> (String, Vec<String>) {
    let cards: Vec<String> = fs::read_to_string("/proc/asound/cards")
        .unwrap_or_default()
        .lines()
        .filter(|l| l.trim().chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false))
        .map(|l| l.trim().to_string())
        .collect();

    let server = Command::new("pactl")
        .arg("info")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|txt| {
            txt.lines()
                .find(|l| l.starts_with("Server Name:"))
                .map(|l| l.trim_start_matches("Server Name:").trim().to_string())
        })
        .unwrap_or_else(|| "PipeWire / PulseAudio (standby)".to_string());

    (server, cards)
}

fn read_default_sink_name() -> String {
    Command::new("pactl")
        .arg("get-default-sink")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}

fn read_default_source_name() -> String {
    Command::new("pactl")
        .arg("get-default-source")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}

fn parse_pactl_devices(kind: &str, default_name: &str) -> Vec<AudioDevice> {
    let mut out = Vec::new();
    let Ok(res) = Command::new("pactl").args(["list", kind]).output() else {
        return out;
    };
    if !res.status.success() {
        return out;
    }
    let txt = String::from_utf8_lossy(&res.stdout);
    let mut cur_idx = String::new();
    let mut cur_name = String::new();
    let mut cur_desc = String::new();

    let flush = |out: &mut Vec<AudioDevice>, idx: &mut String, name: &mut String, desc: &mut String| {
        if !name.is_empty() && !name.ends_with(".monitor") {
            let n = name.clone();
            let d = if desc.is_empty() { n.clone() } else { desc.clone() };
            let is_virtual = n.contains("null")
                || n.contains("virtual")
                || n.contains("auto_null")
                || d.to_lowercase().contains("dummy")
                || d.to_lowercase().contains("virtual")
                || d.to_lowercase().contains("null");
            out.push(AudioDevice {
                index: idx.clone(),
                is_default: n == default_name,
                name: n,
                desc: d,
                is_virtual,
            });
        }
        idx.clear();
        name.clear();
        desc.clear();
    };

    for line in txt.lines() {
        let t = line.trim();
        if t.starts_with("Sink #") || t.starts_with("Source #") {
            flush(&mut out, &mut cur_idx, &mut cur_name, &mut cur_desc);
            cur_idx = t.split('#').nth(1).unwrap_or("0").to_string();
        } else if let Some(v) = t.strip_prefix("Name:") {
            cur_name = v.trim().to_string();
        } else if let Some(v) = t.strip_prefix("Description:") {
            cur_desc = v.trim().to_string();
        }
    }
    flush(&mut out, &mut cur_idx, &mut cur_name, &mut cur_desc);
    out
}

fn read_volume_pct(is_sink: bool) -> (f64, bool) {
    let target = if is_sink { "@DEFAULT_SINK@" } else { "@DEFAULT_SOURCE@" };
    let vol_cmd = if is_sink { "get-sink-volume" } else { "get-source-volume" };
    let mute_cmd = if is_sink { "get-sink-mute" } else { "get-source-mute" };

    let pct = Command::new("pactl")
        .args([vol_cmd, target])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|txt| {
            txt.split('%')
                .next()
                .and_then(|left| left.split_whitespace().last())
                .and_then(|num| num.parse::<f64>().ok())
        })
        .unwrap_or(75.0);

    let muted = Command::new("pactl")
        .args([mute_cmd, target])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.to_lowercase().contains("yes"))
        .unwrap_or(false);

    (pct.clamp(0.0, 150.0), muted)
}

fn ensure_audio_daemon() {
    let _ = Command::new("systemctl")
        .args(["--user", "start", "pipewire", "pipewire-pulse", "wireplumber"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

// ============================================================================
// PAGE: SOUND (Hardware + Virtual Sinks, Master Output, Input Mic, Test Sound)
// ============================================================================
pub unsafe fn build_sound_page() -> *mut GtkWidget {
    let scroller = gtk_scrolled_window_new();
    gtk_scrolled_window_set_policy(scroller, GTK_POLICY_NEVER, GTK_POLICY_AUTOMATIC);
    gtk_widget_set_hexpand(scroller, 1);
    let page = new_box(GTK_ORIENTATION_VERTICAL, 14, &[]);
    set_margin_all(page, 22);

    let title = new_label("Sound", &["app-heading"]);
    gtk_label_set_xalign(title, 0.0);
    gtk_box_append(page, title);

    let status_lbl = new_label("Configure hardware & virtual audio outputs, volume, and microphone input.", &["app-muted"]);
    gtk_label_set_xalign(status_lbl, 0.0);
    gtk_label_set_wrap(status_lbl, 1);
    gtk_box_append(page, status_lbl);

    // 1. Audio Engine & Hardware Summary Card
    let eng_card = new_box(GTK_ORIENTATION_VERTICAL, 8, &["app-card"]);
    let eng_top = new_box(GTK_ORIENTATION_HORIZONTAL, 8, &[]);
    let eng_hdr = new_label("🎚️ AUDIO ENGINE & HARDWARE STATUS", &["app-section-title"]);
    gtk_label_set_xalign(eng_hdr, 0.0);
    gtk_widget_set_hexpand(eng_hdr, 1);
    let btn_refresh = new_button("🔄 Refresh Audio", &["toolbar-btn"]);
    let btn_virtual = new_button("➕ Add Virtual Speaker", &["primary-btn"]);
    gtk_box_append(eng_top, eng_hdr);
    gtk_box_append(eng_top, btn_refresh);
    gtk_box_append(eng_top, btn_virtual);
    gtk_box_append(eng_card, eng_top);

    let eng_info = new_label("", &["app-muted"]);
    gtk_label_set_xalign(eng_info, 0.0);
    gtk_label_set_wrap(eng_info, 1);
    gtk_box_append(eng_card, eng_info);
    gtk_box_append(page, eng_card);

    // 2. Output Volume & Mute Card
    let out_card = new_box(GTK_ORIENTATION_VERTICAL, 10, &["app-card"]);
    let out_hdr = new_label("🔊 OUTPUT VOLUME (SPEAKERS / VIRTUAL SINK)", &["app-section-title"]);
    gtk_label_set_xalign(out_hdr, 0.0);
    gtk_box_append(out_card, out_hdr);

    let (init_out_vol, init_out_mute) = read_volume_pct(true);
    let vol_row = new_box(GTK_ORIENTATION_HORIZONTAL, 10, &[]);
    let vol_lbl = new_label("Output Volume", &["finder-name-btn"]);
    gtk_label_set_xalign(vol_lbl, 0.0);
    gtk_widget_set_size_request(vol_lbl, 130, -1);
    let vol_scale = gtk_scale_new_with_range(GTK_ORIENTATION_HORIZONTAL, 0.0, 150.0, 2.0);
    gtk_widget_set_hexpand(vol_scale, 1);
    gtk_range_set_value(vol_scale, init_out_vol);
    let vol_val = new_label(&format!("{:.0}%", init_out_vol), &["act-proc-mono"]);
    gtk_widget_set_size_request(vol_val, 46, -1);
    gtk_box_append(vol_row, vol_lbl);
    gtk_box_append(vol_row, vol_scale);
    gtk_box_append(vol_row, vol_val);
    gtk_box_append(out_card, vol_row);

    let mute_row = new_box(GTK_ORIENTATION_HORIZONTAL, 10, &[]);
    let mute_lbl = new_label("Mute Output", &["finder-name-btn"]);
    gtk_label_set_xalign(mute_lbl, 0.0);
    gtk_widget_set_hexpand(mute_lbl, 1);
    let mute_sw = gtk_switch_new();
    gtk_widget_set_valign(mute_sw, GTK_ALIGN_CENTER);
    gtk_switch_set_active(mute_sw, if init_out_mute { 1 } else { 0 });
    let btn_test = new_button("🔔 Test Sound", &["toolbar-btn"]);
    gtk_box_append(mute_row, mute_lbl);
    gtk_box_append(mute_row, mute_sw);
    gtk_box_append(mute_row, btn_test);
    gtk_box_append(out_card, mute_row);

    let sinks_hdr = new_label("OUTPUT DEVICES (HARDWARE & VIRTUAL)", &["kv-key"]);
    gtk_label_set_xalign(sinks_hdr, 0.0);
    let sinks_box = new_box(GTK_ORIENTATION_VERTICAL, 5, &[]);
    gtk_box_append(out_card, sinks_hdr);
    gtk_box_append(out_card, sinks_box);
    gtk_box_append(page, out_card);

    // 3. Input / Microphone Card
    let in_card = new_box(GTK_ORIENTATION_VERTICAL, 10, &["app-card"]);
    let in_hdr = new_label("🎙️ INPUT / MICROPHONE", &["app-section-title"]);
    gtk_label_set_xalign(in_hdr, 0.0);
    gtk_box_append(in_card, in_hdr);

    let (init_in_vol, init_in_mute) = read_volume_pct(false);
    let mic_row = new_box(GTK_ORIENTATION_HORIZONTAL, 10, &[]);
    let mic_lbl = new_label("Input Level", &["finder-name-btn"]);
    gtk_label_set_xalign(mic_lbl, 0.0);
    gtk_widget_set_size_request(mic_lbl, 130, -1);
    let mic_scale = gtk_scale_new_with_range(GTK_ORIENTATION_HORIZONTAL, 0.0, 100.0, 2.0);
    gtk_widget_set_hexpand(mic_scale, 1);
    gtk_range_set_value(mic_scale, init_in_vol);
    let mic_val = new_label(&format!("{:.0}%", init_in_vol), &["act-proc-mono"]);
    gtk_widget_set_size_request(mic_val, 46, -1);
    gtk_box_append(mic_row, mic_lbl);
    gtk_box_append(mic_row, mic_scale);
    gtk_box_append(mic_row, mic_val);
    gtk_box_append(in_card, mic_row);

    let mic_mute_row = new_box(GTK_ORIENTATION_HORIZONTAL, 10, &[]);
    let mic_mute_lbl = new_label("Mute Microphone", &["finder-name-btn"]);
    gtk_label_set_xalign(mic_mute_lbl, 0.0);
    gtk_widget_set_hexpand(mic_mute_lbl, 1);
    let mic_mute_sw = gtk_switch_new();
    gtk_widget_set_valign(mic_mute_sw, GTK_ALIGN_CENTER);
    gtk_switch_set_active(mic_mute_sw, if init_in_mute { 1 } else { 0 });
    gtk_box_append(mic_mute_row, mic_mute_lbl);
    gtk_box_append(mic_mute_row, mic_mute_sw);
    gtk_box_append(in_card, mic_mute_row);

    let sources_box = new_box(GTK_ORIENTATION_VERTICAL, 5, &[]);
    gtk_box_append(in_card, sources_box);
    gtk_box_append(page, in_card);

    // Wire volume & mute controls
    connect_signal(vol_scale, "value-changed", move || {
        let v = gtk_range_get_value(vol_scale).round() as u32;
        set_label_text(vol_val, &format!("{}%", v));
        let _ = Command::new("pactl")
            .args(["set-sink-volume", "@DEFAULT_SINK@", &format!("{}%", v)])
            .status();
    });
    connect_notify(mute_sw, "notify::active", move || {
        let m = if gtk_switch_get_active(mute_sw) != 0 { "1" } else { "0" };
        let _ = Command::new("pactl")
            .args(["set-sink-mute", "@DEFAULT_SINK@", m])
            .status();
        set_label_text(
            status_lbl,
            if m == "1" { "🔇 Audio output muted." } else { "🔊 Audio output unmuted." },
        );
    });
    connect_signal(mic_scale, "value-changed", move || {
        let v = gtk_range_get_value(mic_scale).round() as u32;
        set_label_text(mic_val, &format!("{}%", v));
        let _ = Command::new("pactl")
            .args(["set-source-volume", "@DEFAULT_SOURCE@", &format!("{}%", v)])
            .status();
    });
    connect_notify(mic_mute_sw, "notify::active", move || {
        let m = if gtk_switch_get_active(mic_mute_sw) != 0 { "1" } else { "0" };
        let _ = Command::new("pactl")
            .args(["set-source-mute", "@DEFAULT_SOURCE@", m])
            .status();
    });

    connect_signal(btn_test, "clicked", move || {
        let ok = Command::new("sh")
            .args([
                "-c",
                "if command -v paplay >/dev/null 2>&1 && [ -f /usr/share/sounds/freedesktop/stereo/complete.oga ]; then \
                   paplay /usr/share/sounds/freedesktop/stereo/complete.oga; \
                 else \
                   pactl get-default-sink >/dev/null 2>&1; \
                 fi",
            ])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        set_label_text(
            status_lbl,
            if ok {
                "🔔 Test tone sent to active audio output."
            } else {
                "⚠️ Audio daemon not running or no active sink. Click 'Add Virtual Speaker'."
            },
        );
    });

    // Populate audio devices closure
    let populate_audio: Rc<RefCell<Box<dyn Fn()>>> = Rc::new(RefCell::new(Box::new(|| {})));
    let pop_a = Rc::clone(&populate_audio);
    *populate_audio.borrow_mut() = Box::new(move || {
        let (server, cards) = read_audio_backend_status();
        let hw_str = if cards.is_empty() {
            "No physical ALSA soundcard detected (Virtual / VM mode)".to_string()
        } else {
            format!("{} hardware card(s): {}", cards.len(), cards.join(" • "))
        };
        set_label_text(eng_info, &format!("Server: {}  •  {}", server, hw_str));

        let def_sink = read_default_sink_name();
        let sinks = parse_pactl_devices("sinks", &def_sink);
        clear_box_children(sinks_box);
        if sinks.is_empty() {
            let em = new_label(
                "No output devices detected yet. Click '➕ Add Virtual Speaker' to create a virtual PipeWire/PulseAudio sink.",
                &["finder-meta-lbl"],
            );
            gtk_label_set_xalign(em, 0.0);
            gtk_label_set_wrap(em, 1);
            gtk_box_append(sinks_box, em);
        } else {
            for d in sinks {
                let row = new_box(GTK_ORIENTATION_HORIZONTAL, 8, &["act-proc-row"]);
                let col = new_box(GTK_ORIENTATION_VERTICAL, 1, &[]);
                gtk_widget_set_hexpand(col, 1);
                let top = new_box(GTK_ORIENTATION_HORIZONTAL, 6, &[]);
                let ic = if d.is_virtual { "☁️" } else { "🔊" };
                let nl = new_label(&format!("{} {}", ic, d.desc), &["act-proc-name"]);
                gtk_label_set_xalign(nl, 0.0);
                let kind_b = new_label(
                    if d.is_virtual { "Virtual" } else { "Hardware" },
                    if d.is_virtual { &["act-state-other"] } else { &["act-state-run"] },
                );
                gtk_box_append(top, nl);
                gtk_box_append(top, kind_b);
                let sub = new_label(&format!("Sink #{}  •  {}", d.index, d.name), &["finder-meta-lbl"]);
                gtk_label_set_xalign(sub, 0.0);
                gtk_box_append(col, top);
                gtk_box_append(col, sub);
                gtk_box_append(row, col);

                if d.is_default {
                    let act = new_label("✓ Active", &["act-state-run"]);
                    gtk_widget_set_valign(act, GTK_ALIGN_CENTER);
                    gtk_box_append(row, act);
                } else {
                    let btn = new_button("Set Default", &["toolbar-btn"]);
                    gtk_widget_set_valign(btn, GTK_ALIGN_CENTER);
                    let dn = d.name.clone();
                    let dd = d.desc.clone();
                    let pa = Rc::clone(&pop_a);
                    connect_signal(btn, "clicked", move || {
                        let _ = Command::new("pactl").args(["set-default-sink", &dn]).status();
                        set_label_text(status_lbl, &format!("✅ Default output set to '{}'.", dd));
                        (pa.borrow())();
                    });
                    gtk_box_append(row, btn);
                }
                gtk_box_append(sinks_box, row);
            }
        }

        let def_src = read_default_source_name();
        let sources = parse_pactl_devices("sources", &def_src);
        clear_box_children(sources_box);
        if sources.is_empty() {
            let em = new_label("No hardware microphone detected (virtual inputs appear automatically when created).", &["finder-meta-lbl"]);
            gtk_label_set_xalign(em, 0.0);
            gtk_box_append(sources_box, em);
        } else {
            for d in sources {
                let row = new_box(GTK_ORIENTATION_HORIZONTAL, 8, &["act-proc-row"]);
                let nl = new_label(&format!("🎙️ {}", d.desc), &["act-proc-name"]);
                gtk_label_set_xalign(nl, 0.0);
                gtk_widget_set_hexpand(nl, 1);
                gtk_box_append(row, nl);
                if d.is_default {
                    let act = new_label("✓ Active", &["act-state-run"]);
                    gtk_box_append(row, act);
                } else {
                    let btn = new_button("Set Default", &["toolbar-btn"]);
                    let dn = d.name.clone();
                    let pa = Rc::clone(&pop_a);
                    connect_signal(btn, "clicked", move || {
                        let _ = Command::new("pactl").args(["set-default-source", &dn]).status();
                        (pa.borrow())();
                    });
                    gtk_box_append(row, btn);
                }
                gtk_box_append(sources_box, row);
            }
        }
    });

    {
        let pa = Rc::clone(&populate_audio);
        connect_signal(btn_refresh, "clicked", move || {
            ensure_audio_daemon();
            (pa.borrow())();
            set_label_text(status_lbl, "🔄 Audio devices refreshed.");
        });
    }
    {
        let pa = Rc::clone(&populate_audio);
        connect_signal(btn_virtual, "clicked", move || {
            ensure_audio_daemon();
            let ok = Command::new("pactl")
                .args([
                    "load-module",
                    "module-null-sink",
                    "sink_name=bijios_virtual",
                    "sink_properties=device.description=BijiOS-Virtual-Speaker",
                ])
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
            if ok {
                let _ = Command::new("pactl").args(["set-default-sink", "bijios_virtual"]).status();
                set_label_text(status_lbl, "✅ Created & activated 'BijiOS-Virtual-Speaker' virtual sink.");
            } else {
                set_label_text(status_lbl, "⚠️ Could not load virtual sink (ensure PipeWire/PulseAudio user session is active).");
            }
            (pa.borrow())();
        });
    }

    (populate_audio.borrow())();
    gtk_scrolled_window_set_child(scroller, page);
    scroller
}

// ============================================================================
// 2. NETWORK HELPERS (Ethernet & Wi-Fi via nmcli + ip)
// ============================================================================

#[derive(Clone)]
struct NetDeviceInfo {
    device: String,
    kind: String, // "ethernet", "wifi", "other"
    state: String,
    connection: String,
    ipv4: String,
    mac: String,
}

#[derive(Clone)]
struct WifiNetwork {
    in_use: bool,
    ssid: String,
    signal: String,
    security: String,
}

fn read_network_devices() -> Vec<NetDeviceInfo> {
    let mut devs = Vec::new();
    let mut ipv4_map: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    if let Ok(out) = Command::new("ip").args(["-4", "-o", "addr", "show"]).output() {
        for line in String::from_utf8_lossy(&out.stdout).lines() {
            let p: Vec<&str> = line.split_whitespace().collect();
            if p.len() >= 4 {
                ipv4_map.insert(p[1].to_string(), p[3].to_string());
            }
        }
    }

    if let Ok(out) = Command::new("nmcli")
        .args(["-t", "-f", "DEVICE,TYPE,STATE,CONNECTION", "device", "status"])
        .output()
    {
        if out.status.success() {
            for line in String::from_utf8_lossy(&out.stdout).lines() {
                let p: Vec<&str> = line.split(':').collect();
                if p.len() >= 4 && p[0] != "lo" && p[1] != "loopback" {
                    let dev = p[0].to_string();
                    let mac = fs::read_to_string(format!("/sys/class/net/{}/address", dev))
                        .unwrap_or_default()
                        .trim()
                        .to_uppercase();
                    devs.push(NetDeviceInfo {
                        ipv4: ipv4_map.get(&dev).cloned().unwrap_or_else(|| "—".to_string()),
                        device: dev,
                        kind: p[1].to_string(),
                        state: p[2].to_string(),
                        connection: p[3].to_string(),
                        mac,
                    });
                }
            }
        }
    }

    // Fallback to /sys/class/net if nmcli returned nothing
    if devs.is_empty() {
        for (dev, ip) in ipv4_map {
            if dev == "lo" {
                continue;
            }
            let is_wifi = std::path::Path::new(&format!("/sys/class/net/{}/wireless", dev)).exists();
            let oper = fs::read_to_string(format!("/sys/class/net/{}/operstate", dev))
                .unwrap_or_default()
                .trim()
                .to_string();
            let mac = fs::read_to_string(format!("/sys/class/net/{}/address", dev))
                .unwrap_or_default()
                .trim()
                .to_uppercase();
            devs.push(NetDeviceInfo {
                device: dev,
                kind: if is_wifi { "wifi".into() } else { "ethernet".into() },
                state: if oper == "up" { "connected".into() } else { "disconnected".into() },
                connection: "System Interface".into(),
                ipv4: ip,
                mac,
            });
        }
    }
    devs
}

fn is_wifi_radio_enabled() -> bool {
    Command::new("nmcli")
        .args(["radio", "wifi"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().eq_ignore_ascii_case("enabled"))
        .unwrap_or(false)
}

fn scan_wifi_networks(rescan: bool) -> Vec<WifiNetwork> {
    let mut nets = Vec::new();
    let mut args = vec!["-t", "-f", "IN-USE,SSID,SIGNAL,SECURITY", "dev", "wifi", "list"];
    if rescan {
        args.push("--rescan");
        args.push("yes");
    }
    if let Ok(out) = Command::new("nmcli").args(&args).output() {
        if out.status.success() {
            for line in String::from_utf8_lossy(&out.stdout).lines() {
                let p: Vec<&str> = line.split(':').collect();
                if p.len() >= 4 {
                    let ssid = p[1].trim().to_string();
                    if ssid.is_empty() || nets.iter().any(|n: &WifiNetwork| n.ssid == ssid) {
                        continue;
                    }
                    nets.push(WifiNetwork {
                        in_use: p[0].trim() == "*",
                        ssid,
                        signal: p[2].trim().to_string(),
                        security: if p[3].trim().is_empty() {
                            "Open".to_string()
                        } else {
                            p[3].trim().to_string()
                        },
                    });
                }
            }
        }
    }
    nets
}

fn read_default_gateway_and_dns() -> (String, String) {
    let gw = Command::new("sh")
        .args(["-c", "ip route | awk '/^default/{print $3; exit}'"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "—".to_string());

    let dns: Vec<String> = fs::read_to_string("/etc/resolv.conf")
        .unwrap_or_default()
        .lines()
        .filter_map(|l| l.trim().strip_prefix("nameserver "))
        .map(|s| s.trim().to_string())
        .collect();
    let dns_str = if dns.is_empty() { "—".to_string() } else { dns.join(", ") };
    (gw, dns_str)
}

unsafe fn open_wifi_connect_dialog(
    app: *mut GtkApplication,
    ssid: String,
    status_lbl: *mut GtkWidget,
    on_done: Rc<RefCell<Box<dyn Fn()>>>,
) {
    let dlg = create_app_window(app, &format!("📶 Connect to {}", ssid), 400, 220);
    gtk_window_set_resizable(dlg, 0);
    gtk_window_set_modal(dlg, 1);
    let root = new_box(GTK_ORIENTATION_VERTICAL, 12, &[]);
    set_margin_all(root, 20);

    let hdr = new_label(&format!("Connect to Wi-Fi '{}'", ssid), &["app-heading"]);
    gtk_label_set_xalign(hdr, 0.0);
    let pw = gtk_entry_new();
    let ph = cstr("Enter Wi-Fi password");
    gtk_entry_set_placeholder_text(pw, ph.as_ptr());
    gtk_entry_set_visibility(pw, 0);

    let row = new_box(GTK_ORIENTATION_HORIZONTAL, 8, &[]);
    gtk_widget_set_halign(row, GTK_ALIGN_END);
    let btn_cancel = new_button("Cancel", &["toolbar-btn"]);
    let btn_ok = new_button("Connect", &["primary-btn"]);
    gtk_box_append(row, btn_cancel);
    gtk_box_append(row, btn_ok);

    gtk_box_append(root, hdr);
    gtk_box_append(root, pw);
    gtk_box_append(root, row);

    connect_signal(btn_cancel, "clicked", move || gtk_window_close(dlg));
    connect_signal(btn_ok, "clicked", move || {
        let pass = get_entry_text(pw);
        let cmd = format!(
            "nmcli dev wifi connect {} password {}",
            shell_quote(&ssid),
            shell_quote(&pass)
        );
        if run_pkexec_sh(&cmd) {
            set_label_text(status_lbl, &format!("✅ Connected to Wi-Fi '{}'.", ssid));
            (on_done.borrow())();
            gtk_window_close(dlg);
        } else {
            set_label_text(status_lbl, &format!("❌ Failed to connect to '{}'. Check password.", ssid));
        }
    });

    gtk_window_set_child(dlg, root);
    gtk_window_present(dlg);
}

fn is_valid_ipv4(s: &str) -> bool {
    let parts: Vec<&str> = s.trim().split('.').collect();
    parts.len() == 4 && parts.iter().all(|p| !p.is_empty() && p.parse::<u8>().is_ok())
}

unsafe fn open_ethernet_ip_dialog(
    app: *mut GtkApplication,
    dev: NetDeviceInfo,
    status_lbl: *mut GtkWidget,
    on_done: Rc<RefCell<Box<dyn Fn()>>>,
) {
    let dlg = create_app_window(app, &format!("⚙️ Configure IPv4 — {}", dev.device), 460, 410);
    gtk_window_set_resizable(dlg, 0);
    gtk_window_set_modal(dlg, 1);

    let root = new_box(GTK_ORIENTATION_VERTICAL, 12, &[]);
    set_margin_all(root, 20);

    let hdr = new_label(&format!("🔌 IPv4 Configuration ({})", dev.device), &["app-heading"]);
    gtk_label_set_xalign(hdr, 0.0);
    gtk_box_append(root, hdr);

    let sub = new_label(
        "Switch between Automatic (DHCP) or Manual (Static IPv4). Changes apply immediately.",
        &["app-muted"],
    );
    gtk_label_set_xalign(sub, 0.0);
    gtk_label_set_wrap(sub, 1);
    gtk_box_append(root, sub);

    // Mode Toggle: Manual (Static) vs Automatic (DHCP)
    let is_manual = Rc::new(Cell::new(true));
    let mode_row = new_box(GTK_ORIENTATION_HORIZONTAL, 8, &[]);
    let btn_manual = new_button("📌 Manual (Static IP)", &["seg-btn-active"]);
    let btn_dhcp = new_button("⚡ Automatic (DHCP)", &["seg-btn"]);
    gtk_box_append(mode_row, btn_manual);
    gtk_box_append(mode_row, btn_dhcp);
    gtk_box_append(root, mode_row);

    // Pre-fill from current interface IP/CIDR, Gateway & DNS
    let (cur_ip, cur_prefix) = if let Some((ip, pfx)) = dev.ipv4.split_once('/') {
        (ip.to_string(), pfx.to_string())
    } else if dev.ipv4 != "—" {
        (dev.ipv4.clone(), "24".to_string())
    } else {
        (String::new(), "24".to_string())
    };
    let (cur_gw, cur_dns) = read_default_gateway_and_dns();

    let mk_field = |label: &str, init_val: &str, placeholder: &str| -> (*mut GtkWidget, *mut GtkWidget) {
        let row = new_box(GTK_ORIENTATION_HORIZONTAL, 10, &[]);
        let l = new_label(label, &["kv-key"]);
        gtk_label_set_xalign(l, 0.0);
        gtk_widget_set_size_request(l, 130, -1);
        let e = gtk_entry_new();
        gtk_widget_set_hexpand(e, 1);
        let ph = cstr(placeholder);
        gtk_entry_set_placeholder_text(e, ph.as_ptr());
        if !init_val.is_empty() && init_val != "—" {
            set_entry_text(e, init_val);
        }
        gtk_box_append(row, l);
        gtk_box_append(row, e);
        (row, e)
    };

    let (r_ip, e_ip) = mk_field("IPv4 Address", &cur_ip, "e.g. 192.168.202.10");
    let (r_pfx, e_pfx) = mk_field("Subnet Prefix", &cur_prefix, "e.g. 24 (255.255.255.0)");
    let (r_gw, e_gw) = mk_field("Default Gateway", &cur_gw, "e.g. 192.168.202.1");
    let (r_dns, e_dns) = mk_field("DNS Servers", &cur_dns, "e.g. 1.1.1.1, 8.8.8.8");

    let form_box = new_box(GTK_ORIENTATION_VERTICAL, 8, &["app-card"]);
    gtk_box_append(form_box, r_ip);
    gtk_box_append(form_box, r_pfx);
    gtk_box_append(form_box, r_gw);
    gtk_box_append(form_box, r_dns);
    gtk_box_append(root, form_box);

    let dlg_err = new_label("", &["finder-meta-lbl"]);
    gtk_label_set_xalign(dlg_err, 0.0);
    gtk_widget_set_visible(dlg_err, 0);
    gtk_box_append(root, dlg_err);

    {
        let im = Rc::clone(&is_manual);
        connect_signal(btn_manual, "clicked", move || {
            im.set(true);
            remove_css(btn_dhcp, "seg-btn-active");
            add_css(btn_dhcp, "seg-btn");
            remove_css(btn_manual, "seg-btn");
            add_css(btn_manual, "seg-btn-active");
            gtk_widget_set_sensitive(form_box, 1);
        });
    }
    {
        let im = Rc::clone(&is_manual);
        connect_signal(btn_dhcp, "clicked", move || {
            im.set(false);
            remove_css(btn_manual, "seg-btn-active");
            add_css(btn_manual, "seg-btn");
            remove_css(btn_dhcp, "seg-btn");
            add_css(btn_dhcp, "seg-btn-active");
            gtk_widget_set_sensitive(form_box, 0);
        });
    }

    let btn_row = new_box(GTK_ORIENTATION_HORIZONTAL, 8, &[]);
    gtk_widget_set_halign(btn_row, GTK_ALIGN_END);
    let btn_cancel = new_button("Cancel", &["toolbar-btn"]);
    let btn_apply = new_button("💾 Apply IPv4", &["primary-btn"]);
    gtk_box_append(btn_row, btn_cancel);
    gtk_box_append(btn_row, btn_apply);
    gtk_box_append(root, btn_row);

    connect_signal(btn_cancel, "clicked", move || gtk_window_close(dlg));

    let dev_name = dev.device.clone();
    connect_signal(btn_apply, "clicked", move || {
        let qdev = shell_quote(&dev_name);
        if !is_manual.get() {
            // Switch to Automatic (DHCP)
            let script = format!(
                "CONN=$(nmcli -t -f NAME,DEVICE con show --active 2>/dev/null | awk -F: -v d={d} '$2==d{{print $1; exit}}'); \
                 if [ -n \"$CONN\" ]; then \
                   nmcli con mod \"$CONN\" ipv4.method auto ipv4.addresses \"\" ipv4.gateway \"\" ipv4.dns \"\" && nmcli con up \"$CONN\"; \
                 else \
                   rm -f /etc/network/interfaces.d/bijios-{d_raw}.cfg; \
                   dhclient -r {d} 2>/dev/null || true; dhclient {d} 2>/dev/null || nmcli device connect {d}; \
                 fi",
                d = qdev,
                d_raw = dev_name.replace(|c: char| !c.is_alphanumeric() && c != '-' && c != '_', "")
            );
            if run_pkexec_sh(&script) {
                set_label_text(status_lbl, &format!("✅ Switched '{}' to Automatic (DHCP).", dev_name));
                (on_done.borrow())();
                gtk_window_close(dlg);
            } else {
                set_label_text(dlg_err, "❌ Failed to switch to DHCP.");
                gtk_widget_set_visible(dlg_err, 1);
            }
            return;
        }

        // Validate Manual Static IPv4
        let ip = get_entry_text(e_ip).trim().to_string();
        let pfx = get_entry_text(e_pfx).trim().parse::<u8>().unwrap_or(0);
        let gw = get_entry_text(e_gw).trim().to_string();
        let dns_raw = get_entry_text(e_dns);
        let dns_list: Vec<String> = dns_raw
            .split([',', ' '])
            .map(|s| s.trim().to_string())
            .filter(|s| is_valid_ipv4(s))
            .collect();

        if !is_valid_ipv4(&ip) {
            set_label_text(dlg_err, "❌ Invalid IPv4 address (expected e.g. 192.168.202.10).");
            gtk_widget_set_visible(dlg_err, 1);
            return;
        }
        if !(1..=32).contains(&pfx) {
            set_label_text(dlg_err, "❌ Subnet prefix must be between 1 and 32 (e.g. 24).");
            gtk_widget_set_visible(dlg_err, 1);
            return;
        }
        if !gw.is_empty() && !is_valid_ipv4(&gw) {
            set_label_text(dlg_err, "❌ Invalid Default Gateway IPv4 address.");
            gtk_widget_set_visible(dlg_err, 1);
            return;
        }

        let cidr = format!("{}/{}", ip, pfx);
        let qcidr = shell_quote(&cidr);
        let qgw = shell_quote(&gw);
        let qdns_nm = shell_quote(&dns_list.join(" "));
        let safe_dev = dev_name.replace(|c: char| !c.is_alphanumeric() && c != '-' && c != '_', "");

        let mut resolv_cmds = String::new();
        if !dns_list.is_empty() {
            let mut rc_txt = String::new();
            for d in &dns_list {
                rc_txt.push_str(&format!("nameserver {}\n", d));
            }
            resolv_cmds = format!("printf %s {} > /etc/resolv.conf;", shell_quote(&rc_txt));
        }

        let gw_nm = if gw.is_empty() {
            "ipv4.gateway \"\"".to_string()
        } else {
            format!("ipv4.gateway {}", qgw)
        };
        let gw_ip_cmd = if gw.is_empty() {
            String::new()
        } else {
            format!("ip route replace default via {} dev {};", qgw, qdev)
        };
        let dns_nm = if dns_list.is_empty() {
            String::new()
        } else {
            format!("ipv4.dns {}", qdns_nm)
        };

        let script = format!(
            "CONN=$(nmcli -t -f NAME,DEVICE con show --active 2>/dev/null | awk -F: -v d={d} '$2==d{{print $1; exit}}'); \
             if [ -n \"$CONN\" ]; then \
               nmcli con mod \"$CONN\" ipv4.method manual ipv4.addresses {cidr} {gw_nm} {dns_nm} && nmcli con up \"$CONN\"; \
             else \
               ip addr flush dev {d} && ip addr add {cidr} dev {d} && ip link set {d} up; \
               {gw_ip_cmd} \
               {resolv_cmds} \
               if [ -d /etc/network/interfaces.d ]; then \
                 printf 'auto {sdev}\\niface {sdev} inet static\\n  address {cidr_raw}\\n' > /etc/network/interfaces.d/bijios-{sdev}.cfg; \
                 if [ -n {qgw} ]; then printf '  gateway %s\\n' {qgw} >> /etc/network/interfaces.d/bijios-{sdev}.cfg; fi; \
               fi; \
             fi",
            d = qdev,
            cidr = qcidr,
            cidr_raw = cidr,
            gw_nm = gw_nm,
            dns_nm = dns_nm,
            gw_ip_cmd = gw_ip_cmd,
            resolv_cmds = resolv_cmds,
            sdev = safe_dev,
            qgw = qgw
        );

        if run_pkexec_sh(&script) {
            set_label_text(status_lbl, &format!("✅ Applied Static IP {} on '{}'.", cidr, dev_name));
            (on_done.borrow())();
            gtk_window_close(dlg);
        } else {
            set_label_text(dlg_err, "❌ Failed to apply IPv4 configuration.");
            gtk_widget_set_visible(dlg_err, 1);
        }
    });

    gtk_window_set_child(dlg, root);
    gtk_window_present(dlg);
}

// ============================================================================
// PAGE: NETWORK (Ethernet Interfaces, Wi-Fi Scanner/Toggle, DNS & Diagnostics)
// ============================================================================
pub unsafe fn build_network_page(app: *mut GtkApplication) -> *mut GtkWidget {
    let scroller = gtk_scrolled_window_new();
    gtk_scrolled_window_set_policy(scroller, GTK_POLICY_NEVER, GTK_POLICY_AUTOMATIC);
    gtk_widget_set_hexpand(scroller, 1);
    let page = new_box(GTK_ORIENTATION_VERTICAL, 14, &[]);
    set_margin_all(page, 22);

    let title = new_label("Network", &["app-heading"]);
    gtk_label_set_xalign(title, 0.0);
    gtk_box_append(page, title);

    let status_lbl = new_label("Manage Ethernet adapters, Wi-Fi networks, DNS, and connectivity.", &["app-muted"]);
    gtk_label_set_xalign(status_lbl, 0.0);
    gtk_label_set_wrap(status_lbl, 1);
    gtk_box_append(page, status_lbl);

    // 1. Ethernet & Active Interfaces Card
    let eth_card = new_box(GTK_ORIENTATION_VERTICAL, 10, &["app-card"]);
    let eth_top = new_box(GTK_ORIENTATION_HORIZONTAL, 8, &[]);
    let eth_hdr = new_label("🔌 ETHERNET & NETWORK INTERFACES", &["app-section-title"]);
    gtk_label_set_xalign(eth_hdr, 0.0);
    gtk_widget_set_hexpand(eth_hdr, 1);
    let btn_refresh_net = new_button("🔄 Refresh", &["toolbar-btn"]);
    let btn_ping = new_button("🌐 Test Internet", &["primary-btn"]);
    gtk_box_append(eth_top, eth_hdr);
    gtk_box_append(eth_top, btn_refresh_net);
    gtk_box_append(eth_top, btn_ping);
    gtk_box_append(eth_card, eth_top);

    let route_info = new_label("", &["app-muted"]);
    gtk_label_set_xalign(route_info, 0.0);
    gtk_box_append(eth_card, route_info);

    let ifaces_box = new_box(GTK_ORIENTATION_VERTICAL, 6, &[]);
    gtk_box_append(eth_card, ifaces_box);
    gtk_box_append(page, eth_card);

    // 2. Wi-Fi Card
    let wifi_card = new_box(GTK_ORIENTATION_VERTICAL, 10, &["app-card"]);
    let wifi_top = new_box(GTK_ORIENTATION_HORIZONTAL, 10, &[]);
    let wifi_hdr = new_label("📶 WI-FI NETWORKS", &["app-section-title"]);
    gtk_label_set_xalign(wifi_hdr, 0.0);
    gtk_widget_set_hexpand(wifi_hdr, 1);
    let btn_scan = new_button("🔍 Scan Wi-Fi", &["toolbar-btn"]);
    let wifi_sw = gtk_switch_new();
    gtk_widget_set_valign(wifi_sw, GTK_ALIGN_CENTER);
    gtk_switch_set_active(wifi_sw, if is_wifi_radio_enabled() { 1 } else { 0 });
    gtk_box_append(wifi_top, wifi_hdr);
    gtk_box_append(wifi_top, btn_scan);
    gtk_box_append(wifi_top, wifi_sw);
    gtk_box_append(wifi_card, wifi_top);

    let wifi_box = new_box(GTK_ORIENTATION_VERTICAL, 5, &[]);
    gtk_box_append(wifi_card, wifi_box);
    gtk_box_append(page, wifi_card);

    // 3. DNS Configuration Card
    let dns_card = new_box(GTK_ORIENTATION_VERTICAL, 8, &["app-card"]);
    let dns_hdr = new_label("🧭 DNS SERVERS", &["app-section-title"]);
    gtk_label_set_xalign(dns_hdr, 0.0);
    gtk_box_append(dns_card, dns_hdr);

    let dns_row = new_box(GTK_ORIENTATION_HORIZONTAL, 8, &[]);
    let dns_entry = gtk_entry_new();
    gtk_widget_set_hexpand(dns_entry, 1);
    let ph_dns = cstr("e.g. 1.1.1.1, 8.8.8.8");
    gtk_entry_set_placeholder_text(dns_entry, ph_dns.as_ptr());
    let btn_set_dns = new_button("Save DNS", &["primary-btn"]);
    gtk_box_append(dns_row, dns_entry);
    gtk_box_append(dns_row, btn_set_dns);
    gtk_box_append(dns_card, dns_row);
    gtk_box_append(page, dns_card);

    // Populate network state closure
    let populate_net: Rc<RefCell<Box<dyn Fn()>>> = Rc::new(RefCell::new(Box::new(|| {})));
    let pop_n = Rc::clone(&populate_net);
    let wifi_guard = Rc::new(Cell::new(false));
    let wg_p = Rc::clone(&wifi_guard);

    *populate_net.borrow_mut() = Box::new(move || {
        let (gw, dns) = read_default_gateway_and_dns();
        set_label_text(route_info, &format!("Default Gateway: {}  •  DNS: {}", gw, dns));

        let devs = read_network_devices();
        let has_wifi_hw = devs.iter().any(|d| d.kind == "wifi");
        clear_box_children(ifaces_box);
        if devs.is_empty() {
            let em = new_label("No network interfaces found.", &["finder-meta-lbl"]);
            gtk_box_append(ifaces_box, em);
        } else {
            for d in devs {
                let row = new_box(GTK_ORIENTATION_HORIZONTAL, 10, &["act-proc-row"]);
                let col = new_box(GTK_ORIENTATION_VERTICAL, 2, &[]);
                gtk_widget_set_hexpand(col, 1);

                let top = new_box(GTK_ORIENTATION_HORIZONTAL, 8, &[]);
                let ic = if d.kind == "wifi" { "📶" } else { "🔌" };
                let nl = new_label(&format!("{} {} ({})", ic, d.device, d.kind.to_uppercase()), &["act-proc-name"]);
                gtk_label_set_xalign(nl, 0.0);
                let is_conn = d.state == "connected" || d.state == "connected (externally)";
                let st_b = new_label(
                    if is_conn { "Connected" } else { &d.state },
                    if is_conn { &["act-state-run"] } else { &["act-state-sleep"] },
                );
                gtk_box_append(top, nl);
                gtk_box_append(top, st_b);

                let meta = new_label(
                    &format!("IPv4: {}  •  MAC: {}  •  Profile: {}", d.ipv4, d.mac, d.connection),
                    &["finder-meta-lbl"],
                );
                gtk_label_set_xalign(meta, 0.0);
                gtk_box_append(col, top);
                gtk_box_append(col, meta);
                gtk_box_append(row, col);

                if d.kind != "wifi" {
                    let btn_ip = new_button("⚙️ Configure IP", &["primary-btn"]);
                    gtk_widget_set_valign(btn_ip, GTK_ALIGN_CENTER);
                    let d_clone = d.clone();
                    let pn_ip = Rc::clone(&pop_n);
                    connect_signal(btn_ip, "clicked", move || {
                        open_ethernet_ip_dialog(app, d_clone.clone(), status_lbl, Rc::clone(&pn_ip));
                    });
                    gtk_box_append(row, btn_ip);
                }

                let btn_tog = new_button(if is_conn { "Disconnect" } else { "Connect" }, &["toolbar-btn"]);
                gtk_widget_set_valign(btn_tog, GTK_ALIGN_CENTER);
                let dev_name = d.device.clone();
                let pn = Rc::clone(&pop_n);
                connect_signal(btn_tog, "clicked", move || {
                    let cmd = if is_conn {
                        format!("nmcli device disconnect {} || ip link set {} down", shell_quote(&dev_name), shell_quote(&dev_name))
                    } else {
                        format!("nmcli device connect {} || ip link set {} up", shell_quote(&dev_name), shell_quote(&dev_name))
                    };
                    if run_pkexec_sh(&cmd) {
                        set_label_text(status_lbl, &format!("✅ Updated interface '{}'.", dev_name));
                        (pn.borrow())();
                    } else {
                        set_label_text(status_lbl, &format!("❌ Failed to change state of '{}'.", dev_name));
                    }
                });
                gtk_box_append(row, btn_tog);
                gtk_box_append(ifaces_box, row);
            }
        }

        // Update Wi-Fi section
        wg_p.set(true);
        gtk_switch_set_active(wifi_sw, if is_wifi_radio_enabled() { 1 } else { 0 });
        wg_p.set(false);

        clear_box_children(wifi_box);
        if !has_wifi_hw {
            let msg = new_label(
                "No Wi-Fi hardware adapter detected on this machine (Ethernet / Virtual NIC active). Connect a USB/PCIe Wi-Fi adapter to scan wireless networks.",
                &["finder-meta-lbl"],
            );
            gtk_label_set_xalign(msg, 0.0);
            gtk_label_set_wrap(msg, 1);
            gtk_box_append(wifi_box, msg);
        } else {
            let nets = scan_wifi_networks(false);
            if nets.is_empty() {
                let msg = new_label("No Wi-Fi networks in range. Click '🔍 Scan Wi-Fi' to refresh.", &["finder-meta-lbl"]);
                gtk_label_set_xalign(msg, 0.0);
                gtk_box_append(wifi_box, msg);
            } else {
                for n in nets.into_iter().take(12) {
                    let row = new_box(GTK_ORIENTATION_HORIZONTAL, 8, &["act-proc-row"]);
                    let nl = new_label(&format!("📶 {}", n.ssid), &["act-proc-name"]);
                    gtk_label_set_xalign(nl, 0.0);
                    gtk_widget_set_hexpand(nl, 1);
                    let sig = new_label(&format!("{}% • {}", n.signal, n.security), &["finder-meta-lbl"]);
                    gtk_box_append(row, nl);
                    gtk_box_append(row, sig);

                    if n.in_use {
                        let c = new_label("✓ Connected", &["act-state-run"]);
                        gtk_box_append(row, c);
                    } else {
                        let btn = new_button("Connect", &["primary-btn"]);
                        let ssid_c = n.ssid.clone();
                        let is_open = n.security == "Open";
                        let pn = Rc::clone(&pop_n);
                        connect_signal(btn, "clicked", move || {
                            if is_open {
                                let cmd = format!("nmcli dev wifi connect {}", shell_quote(&ssid_c));
                                if run_pkexec_sh(&cmd) {
                                    set_label_text(status_lbl, &format!("✅ Connected to '{}'.", ssid_c));
                                    (pn.borrow())();
                                }
                            } else {
                                open_wifi_connect_dialog(app, ssid_c.clone(), status_lbl, Rc::clone(&pn));
                            }
                        });
                        gtk_box_append(row, btn);
                    }
                    gtk_box_append(wifi_box, row);
                }
            }
        }
    });

    // Wire buttons & switches
    {
        let pn = Rc::clone(&populate_net);
        connect_signal(btn_refresh_net, "clicked", move || {
            (pn.borrow())();
            set_label_text(status_lbl, "🔄 Network interfaces refreshed.");
        });
    }
    connect_signal(btn_ping, "clicked", move || {
        let ok = Command::new("ping")
            .args(["-c", "1", "-W", "2", "1.1.1.1"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        set_label_text(
            status_lbl,
            if ok {
                "✅ Internet connectivity OK (ping 1.1.1.1 succeeded)."
            } else {
                "⚠️ Ping to 1.1.1.1 timed out. Check gateway or DNS settings."
            },
        );
    });
    {
        let pn = Rc::clone(&populate_net);
        connect_signal(btn_scan, "clicked", move || {
            let _ = scan_wifi_networks(true);
            (pn.borrow())();
            set_label_text(status_lbl, "🔍 Wi-Fi scan complete.");
        });
    }
    {
        let wg = Rc::clone(&wifi_guard);
        let pn = Rc::clone(&populate_net);
        connect_notify(wifi_sw, "notify::active", move || {
            if wg.get() {
                return;
            }
            let on = gtk_switch_get_active(wifi_sw) != 0;
            let cmd = if on { "nmcli radio wifi on" } else { "nmcli radio wifi off" };
            let _ = run_pkexec_sh(cmd);
            set_label_text(status_lbl, if on { "📶 Wi-Fi radio enabled." } else { "📶 Wi-Fi radio disabled." });
            (pn.borrow())();
        });
    }
    {
        let pn = Rc::clone(&populate_net);
        connect_signal(btn_set_dns, "clicked", move || {
            let raw = get_entry_text(dns_entry);
            let servers: Vec<&str> = raw
                .split([',', ' '])
                .map(|s| s.trim())
                .filter(|s| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit() || c == '.' || c == ':'))
                .collect();
            if servers.is_empty() {
                set_label_text(status_lbl, "❌ Enter valid DNS IP(s), e.g. 1.1.1.1, 8.8.8.8");
                return;
            }
            let mut content = String::new();
            for s in &servers {
                content.push_str(&format!("nameserver {}\n", s));
            }
            let script = format!("printf %s {} > /etc/resolv.conf", shell_quote(&content));
            if run_pkexec_sh(&script) {
                set_label_text(status_lbl, &format!("✅ DNS updated to: {}", servers.join(", ")));
                (pn.borrow())();
            } else {
                set_label_text(status_lbl, "❌ Failed to update DNS.");
            }
        });
    }

    (populate_net.borrow())();
    gtk_scrolled_window_set_child(scroller, page);
    scroller
}

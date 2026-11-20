<p align="center">
  <img src="static/img/bijios-logo-badge.svg" alt="BijiOS Desktop — Pure Rust Linux Desktop Environment Logo" width="96" />
</p>

<h1 align="center">BijiOS Desktop — Pure Rust Linux Desktop Environment (v1.0.0 "Padi")</h1>

<p align="center">
  <strong>A fast, sovereign Linux Desktop Environment written 100% in Pure Rust with Native GTK4, Cairo, VTE4 &amp; X11</strong><br/>
  <em>Zero Chromium • Zero Electron • Zero External Crates • Liquid Glass UI • Debian 13 (Trixie)</em>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Language-100%25%20Pure%20Rust-DEA584?style=flat-square&logo=rust&logoColor=white" alt="Written in 100% Pure Rust" />
  <img src="https://img.shields.io/badge/UI-Native%20GTK4%20%2B%20Cairo-2F7D4F?style=flat-square" alt="Native GTK4 and Cairo UI" />
  <img src="https://img.shields.io/badge/Dependencies-Zero%20crates.io-12332A?style=flat-square" alt="Zero External Crates" />
  <img src="https://img.shields.io/badge/Platform-Debian%2013%20(Trixie)-A81D33?style=flat-square&logo=debian&logoColor=white" alt="Debian 13 Trixie" />
  <img src="https://img.shields.io/badge/Release-v1.0.0%20Padi-E8A33D?style=flat-square" alt="BijiOS v1.0.0 Padi" />
</p>

<p align="center">
  <img src="Screenshot/About-BijiOS.png" alt="BijiOS Desktop v1.0.0 Padi — Pure Rust Linux Desktop Environment with About BijiOS and Pancer Terminal" width="900" />
</p>

---

## What is BijiOS?

**BijiOS Desktop** is an open-source, lightweight, sovereign **Linux Desktop Environment written entirely in the Rust programming language**. Built from the ground up with **zero external `crates.io` dependencies** and **zero Chromium / Electron overhead**, BijiOS binds directly via Rust C FFI to **GTK4**, **Cairo**, **VTE4** (`libvte-2.91-gtk4`), and **X11/Xlib**, running cleanly over **X11 + Openbox + `xcompmgr`** on **Debian 13 (`trixie`)**.

Designed around a cohesive Indonesian botanical identity (**BijiOS v1.0.0 "Padi"**), it delivers macOS-inspired **Liquid Glass** window chrome, true rounded corners, real-time Cairo telemetry graphs, a full interactive multi-tab/split-pane terminal, customizable user profile avatars, dynamic nature wallpapers, hardware/VNC display management, and native `.deb` package integration — all packaged into a single compact `~1.4 MB` Debian package.

---

## Why Written in 100% Pure Rust?

Most modern custom desktop shells rely on heavy web runtimes (Electron, Chromium, WebView, Node.js) or complex multi-language stacks. **BijiOS takes a different approach**:

- **100% Pure Rust (`rustc` + `Cargo`)**: The desktop shell, window chrome, login/lock screen, file manager, app launcher, app store, terminal wrapper, real-time system monitor, settings panels, PAM authentication daemon, and storage engine are all compiled into one native Rust binary.
- **Zero External `crates.io` Dependencies**: `[dependencies]` in `Cargo.toml` is intentionally empty — enabling offline, auditable, reproducible builds with zero supply-chain bloat.
- **Direct Native System FFI**: Links directly against Debian system libraries (`libgtk-4`, `libcairo`, `libvte-2.91-gtk4`, `libpango-1.0`, `libgio-2.0`, `libX11`) for native speed and ultra-low RAM usage.
- **Pure Rust Bundled Office Suite**: Integrates Storytold's clean-room pure Rust office apps (**WordCraft**, **GridCraft**, **DeckCraft**, **PDFCraft**) under Apache License 2.0.
- **Built-in TOON v4 Storage Engine**: Custom Token-Oriented Object Notation database engine written in Rust.

---

## Core Native Applications

| Application | Description |
| :--- | :--- |
| **Akar — File Manager** | Native GTK4 file manager with sidebar navigation, zebra-striped file table (`Name \| Size \| Kind`), file-type badges, quick folder/document creation, and automatic file associations. |
| **Semai — App Launcher** | Proportional squircle launcher combining built-in BijiOS apps and all installed Linux `.deb` applications with instant search. |
| **Bibit — Debian App Store** | Curated Debian desktop catalog + live `apt-cache search`, `dpkg-query` status detection, native GTK4 **sudo/PAM Authentication Modal**, background `apt-get` install/remove, and automatic launcher sync. |
| **Pancer — Native Terminal** | Full interactive PTY/TTY `/bin/bash` terminal powered by `libvte-2.91-gtk4` featuring `File / Edit / Shell` menus, **`Ctrl+Shift` hotkeys**, **Split Vertical & Split Horizontal** panes, and a **Dynamic Host Tab Bar** (`\| 💻 BijiOS \| 🔐 ssh-host \| + \|`) that auto-detects local vs SSH sessions. |
| **Activity Monitor** | Aesthetic real-time system monitor with **60-second live Cairo area charts**, 6 telemetry tabs (**Overview**, **CPU** per-core bars, **Memory**, **Disk I/O & Mounts**, **Network**, and an interactive **Process Manager** with search, sort, `End` SIGTERM, and `Kill` SIGKILL). |
| **System Settings** | Sidebar settings app covering **Desktop & Dock** (live size, magnification, position, minimize effects), **Displays & VNC** (hardware `xrandr` resolution picker, custom `W×H`, live **TigerVNC** & **X11VNC** enable/disable toggles & password), **Wallpaper** (5 nature SVGs + Dynamic time-of-day wallpaper), **General** (OS/KVM info, APT updates, storage, power & Log Out), and **Users & Groups** (customizable circular **User Profile Avatars** from local photos or built-in presets, user/password/group management). |
| **WordCraft, GridCraft, DeckCraft & PDFCraft** | Bundled pure-Rust native office suite by **Storytold / ArtCraft** (Apache-2.0) for `.docx`, `.xlsx`, `.pptx`, and `.pdf` documents. |
| **Liquid Glass Login & Lock Screen** | Fullscreen above-all X11 lock/login screen backed by local Linux PAM authentication with custom user avatar sync, Sleep, Restart, Shut Down, and Log Out controls. |

---

## Brand Identity & Color Palette

| Name | Hex | Role |
| :--- | :--- | :--- |
| **Hijau Tunas** | `#2F7D4F` | Primary accent & active states |
| **Hijau Muda** | `#7FBF6A` | Secondary highlight & growth accent |
| **Krem Tanah** | `#F4EBD9` | Warm light foreground & canvas |
| **Emas Benih** | `#E8A33D` | Seed gold accent & terminal cursor |
| **Hutan Malam** | `#12332A` | Deep forest dark chrome & typography |

---

## Project Architecture

```text
bijios-desktop/
├── build.rs                 # Direct system library linking (gtk-4, cairo, vte-2.91-gtk4, pango, glib, X11)
├── Cargo.toml               # Zero crates.io external dependencies
├── src/
│   ├── main.rs              # Dual-mode binary: HTTP/PAM/TOON daemon & --shell native DE
│   ├── native_de/
│   │   ├── gtk4_ffi.rs      # Pure Rust FFI bindings for GTK4, Cairo, VTE4, Pango, GLib, GObject, Xlib
│   │   ├── theme.rs         # BijiOS Liquid Glass GTK4 CSS theme
│   │   ├── mod.rs           # Login/Lock screen, Desktop shell, Dock, Widgets, Dynamic VNC resize sync
│   │   ├── apps.rs          # Akar, Semai, Bibit (sudo/PAM modal), Notes, Calculator, About BijiOS
│   │   ├── pancer.rs        # Multi-tab, split-pane VTE4 terminal with dynamic SSH host bar & hotkeys
│   │   ├── activity.rs      # Real-time Cairo area-chart Activity Monitor & interactive process manager
│   │   └── settings.rs      # System Settings (Dock, Displays & VNC, Wallpaper, General, Users & Avatars)
│   ├── api/                 # Local PAM auth, OS integration & TOON APIs
│   ├── toon/                # TOON v4 (Token-Oriented Object Notation) database engine
│   └── docx/ & xlsx/        # Pure Rust OpenXML document & spreadsheet generators
├── static/
│   ├── img/                 # Official BijiOS SVG logos & IDVE host logo
│   └── wallpapers/          # Nature & Dynamic SVG wallpapers
└── packaging/
    ├── build-deb.sh         # Debian (.deb) package builder
    ├── bundle-storytold.sh  # Storytold Pure Rust Office Suite installer (Apache-2.0)
    └── deb/                 # LightDM, Openbox BijiOS-Liquid theme, systemd & session scripts
```

---

## Build & Install on Debian 13 (Trixie)

### 1. Install Build Dependencies

```bash
sudo apt-get update
sudo apt-get install -y rustc cargo libgtk-4-dev libcairo2-dev libvte-2.91-gtk4-dev libx11-dev dpkg-dev
```

### 2. Compile & Build `.deb` Package

```bash
cargo build --release
bash packaging/build-deb.sh
```

### 3. Install BijiOS Desktop & Bundled Office Suite

```bash
sudo apt-get install -y ./dist/bijios-desktop_1.0.0_amd64.deb
bash packaging/bundle-storytold.sh
sudo systemctl restart lightdm
```

---

## Tentang BijiOS (Bahasa Indonesia)

**BijiOS** adalah lingkungan desktop Linux (*Linux Desktop Environment*) open-source yang dikembangkan dari nol menggunakan **100% bahasa pemrograman Rust** tanpa dependensi eksternal (`crates.io`) dan tanpa Chromium/Electron. BijiOS berjalan ringan di atas **Debian 13 (Trixie)** dengan **GTK4, Cairo, VTE4, dan X11/Openbox**, dilengkapi manajer berkas **Akar**, peluncur aplikasi **Semai**, toko aplikasi Debian **Bibit**, terminal multi-tab/split-pane **Pancer**, **Activity Monitor** dengan grafik Cairo real-time, dukungan **Remote Desktop VNC (TigerVNC & X11VNC)**, serta paket perkantoran murni Rust (**WordCraft, GridCraft, DeckCraft, PDFCraft**).

---

## Frequently Asked Questions (FAQ)

- **What programming language is BijiOS written in?**
  BijiOS Desktop is written in **100% Pure Rust**, compiling directly against native Linux C libraries (`GTK4`, `Cairo`, `VTE4`, `X11`) with zero external Rust crates.
- **Does BijiOS use Electron, Chromium, or WebView?**
  No. Every native window, widget, chart, and terminal in BijiOS is rendered natively using GTK4, Cairo, and VTE4 for minimal RAM consumption and instant responsiveness.
- **Does BijiOS support Remote Desktop (VNC) and dynamic screen resolutions?**
  Yes. BijiOS includes built-in **Displays & VNC** controls in System Settings supporting hardware `xrandr` resolutions, custom `W×H` resolutions, **TigerVNC** (dynamic auto-resize via RFB ExtendedDesktopSize), and **X11VNC**.

---

## Bundled Open-Source Suite & Credits (Apache License 2.0)

BijiOS Desktop bundles and integrates the following pure Rust office applications under their original names, in compliance with the **Apache License 2.0** (see [THIRD_PARTY_LICENSES.md](THIRD_PARTY_LICENSES.md)):

| Application | Version | Description | Upstream Repository | License |
| :--- | :--- | :--- | :--- | :--- |
| **WordCraft** | `0.18.1` | Clean-room native word processor in pure Rust | https://github.com/storytold/wordcraft | Apache-2.0 |
| **GridCraft** | `0.19.1` | Clean-room native spreadsheet in pure Rust | https://github.com/storytold/gridcraft | Apache-2.0 |
| **DeckCraft** | `0.18.1` | Clean-room native presentation app in pure Rust | https://github.com/storytold/deckcraft | Apache-2.0 |
| **PDFCraft** | `0.21.1` | Native PDF reader & editor written from scratch in Rust | https://github.com/storytold/pdfcraft | Apache-2.0 |

---

## Contributors

| Contributor | GitHub | Role |
| :--- | :--- | :--- |
| **Dionipe** | [@dionipe](https://github.com/dionipe) | Creator & Core Maintainer |

- **Repository**: [https://github.com/dionipe/BijiOS](https://github.com/dionipe/BijiOS)
- **Contributors Graph**: [https://github.com/dionipe/BijiOS/graphs/contributors](https://github.com/dionipe/BijiOS/graphs/contributors)

---

## Infrastructure & Credits

- **OS & Desktop**: BijiOS v1.0.0 ("Padi") — [https://github.com/dionipe/BijiOS](https://github.com/dionipe/BijiOS)
- **Bundled Office Suite**: Storytold / ArtCraft ([https://github.com/storytold](https://github.com/storytold)) — Apache License 2.0
- **Cloud & Virtualization Host**: Powered by **IDVE Cloud / KVM** ([https://idve.cloud/](https://idve.cloud/))

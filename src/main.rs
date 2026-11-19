mod api;
mod auth;
mod db;
pub mod docx;
pub mod native_de;
pub mod pptx;
pub mod xlsx;
mod server;
mod toon;

use db::Database;
use server::{HttpServer, ServerConfig};
use std::env;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.iter().any(|a| a == "--shell" || a == "--native") {
        let code = native_de::run_native_shell();
        std::process::exit(code);
    }

    println!("============================================================");
    println!("  🍏 BijiOS - Sovereign Desktop Environment (PAM Enabled)    ");
    println!("  Backend: Pure Rust (Zero External Dependencies)           ");
    println!("  Database: TOON v4 Engine + Linux PAM Authentication       ");
    println!("============================================================");

    let mut base_dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    if base_dir.join("bijios-desktop").join("static").exists() {
        base_dir = base_dir.join("bijios-desktop");
    }

    // Support Debian system-wide paths (/usr/share/bijios-desktop/static & /var/lib/bijios-desktop/db)
    let static_dir = if let Ok(s) = env::var("BIJIOS_STATIC_DIR").or_else(|_| env::var("DOKUHUB_STATIC_DIR")) {
        PathBuf::from(s)
    } else if base_dir.join("static").exists() {
        base_dir.join("static")
    } else if PathBuf::from("/usr/share/bijios-desktop/static").exists() {
        PathBuf::from("/usr/share/bijios-desktop/static")
    } else if PathBuf::from("/usr/share/dokuhub/static").exists() {
        PathBuf::from("/usr/share/dokuhub/static")
    } else {
        base_dir.join("static")
    };

    let db_dir = if let Ok(d) = env::var("BIJIOS_DATA_DIR").or_else(|_| env::var("DOKUHUB_DATA_DIR")) {
        PathBuf::from(d)
    } else if base_dir.join("data").join("db").exists() || base_dir.join("static").exists() {
        base_dir.join("data").join("db")
    } else {
        PathBuf::from("/var/lib/bijios-desktop/db")
    };

    // Initialize TOON Database
    let database = Database::new(&db_dir);
    println!("✅ Database TOON v4 dimuat dari: {:?}", db_dir);
    println!("   - Tenants: {}", database.tenants.len());
    println!("   - Users: {}", database.users.len());
    println!("   - Files: {}", database.files.len());
    println!("   - Shares: {}", database.shares.len());
    println!("   - Notes: {}", database.notes.len());
    println!("   - Meet Rooms: {}", database.rooms.len());

    let db_handle = Arc::new(RwLock::new(database));

    let port: u16 = env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);

    let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());

    let config = ServerConfig {
        host: host.clone(),
        port,
        static_dir,
        db_dir,
    };

    let server = HttpServer::new(config, db_handle);

    println!("\n🌐 BijiOS Desktop UI siap diakses:");
    println!("   👉 http://localhost:{}", port);
    println!("   👉 http://127.0.0.1:{}\n", port);
    println!("Apps yang tersedia di macOS Dock:");
    println!("   1. Finder (File Manager dengan Public/Internal Sharing)");
    println!("   2. WordCraft (Word Editor .docx)");
    println!("   3. GriidCraft (Excel Spreadsheet .xlsx dengan Formula Real-time)");
    println!("   4. DeckCraft (PowerPoint Presentation .pptx dengan Slideshow)");
    println!("   5. PDF Viewer");
    println!("   6. Notes (Markdown & Catatan Pribadi)");
    println!("   7. BijiMeet (Self-Hosted WebRTC Video Conference)");
    println!("   8. Konektor Google Drive (Sync & Import)");
    println!("   9. System Settings & API Docs (OpenAPI / Swagger)");
    println!("  10. Wallpaper Chooser & TOON Inspector\n");

    server.run()
}

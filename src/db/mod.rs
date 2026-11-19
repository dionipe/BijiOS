//! TOON Database Engine for DokuHub
//! Stores all records formatted in TOON v4 specification.

use crate::auth::Role;
use crate::toon::{ToonParser, ToonSerializer};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct Tenant {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub plan: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct User {
    pub id: String,
    pub tenant_id: String,
    pub email: String,
    pub name: String,
    pub password: String,
    pub role: Role,
    pub status: String,
    pub created_at: String,
    pub avatar: String,
    pub color: String,
    pub quota_bytes: u64,
    pub department: String,
    pub supervisor_id: String,
}

#[derive(Debug, Clone)]
pub struct FileItem {
    pub id: String,
    pub tenant_id: String,
    pub parent_id: String,
    pub name: String,
    pub extension: String,
    pub size_bytes: usize,
    pub mime_type: String,
    pub owner_id: String,
    pub is_folder: bool,
    pub content: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct ShareItem {
    pub id: String,
    pub tenant_id: String,
    pub file_id: String,
    pub share_type: String, // "public" | "internal" | "external"
    pub target_user_id: String,
    pub token: String,
    pub permission: String, // "view" | "comment" | "edit"
    pub password: Option<String>,
    pub expires_at: Option<String>,
    pub download_count: u64,
    pub requested_by: String,
    pub approver_id: String,
    pub approval_status: String, // "approved" | "pending" | "rejected"
    pub request_note: String,
    pub approval_note: String,
    pub target_email: String,
}

#[derive(Debug, Clone)]
pub struct NoteItem {
    pub id: String,
    pub tenant_id: String,
    pub user_id: String,
    pub title: String,
    pub content: String,
    pub tags: String,
    pub pinned: bool,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct MeetRoom {
    pub id: String,
    pub tenant_id: String,
    pub code: String,
    pub title: String,
    pub host_id: String,
    pub is_active: bool,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct ApiKeyItem {
    pub id: String,
    pub tenant_id: String,
    pub name: String,
    pub key: String,
    pub created_at: String,
}

pub struct Database {
    db_dir: PathBuf,
    pub tenants: Vec<Tenant>,
    pub users: Vec<User>,
    pub files: Vec<FileItem>,
    pub shares: Vec<ShareItem>,
    pub notes: Vec<NoteItem>,
    pub rooms: Vec<MeetRoom>,
    pub api_keys: Vec<ApiKeyItem>,
}

pub type DbHandle = Arc<RwLock<Database>>;

impl Database {
    pub fn new<P: AsRef<Path>>(db_dir: P) -> Self {
        let db_dir = db_dir.as_ref().to_path_buf();
        fs::create_dir_all(&db_dir).unwrap_or_default();

        let mut db = Self {
            db_dir,
            tenants: Vec::new(),
            users: Vec::new(),
            files: Vec::new(),
            shares: Vec::new(),
            notes: Vec::new(),
            rooms: Vec::new(),
            api_keys: Vec::new(),
        };

        if !db.load_all() {
            db.seed_defaults();
            db.save_all();
        }

        db
    }

    fn now_str() -> String {
        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        format!("{}", secs)
    }

    pub fn save_all(&self) {
        self.save_tenants();
        self.save_users();
        self.save_files();
        self.save_shares();
        self.save_notes();
        self.save_rooms();
        self.save_api_keys();
    }

    pub fn load_all(&mut self) -> bool {
        let tenants_path = self.db_dir.join("tenants.toon");
        if !tenants_path.exists() {
            return false;
        }

        self.load_tenants();
        self.load_users();
        self.load_files();
        self.load_shares();
        self.load_notes();
        self.load_rooms();
        self.load_api_keys();
        true
    }

    // --- Tenants ---
    pub fn save_tenants(&self) {
        let headers = vec![
            "id".into(),
            "name".into(),
            "slug".into(),
            "plan".into(),
            "created_at".into(),
        ];
        let rows: Vec<Vec<String>> = self
            .tenants
            .iter()
            .map(|t| {
                vec![
                    t.id.clone(),
                    t.name.clone(),
                    t.slug.clone(),
                    t.plan.clone(),
                    t.created_at.clone(),
                ]
            })
            .collect();
        let content = format!(
            "{}\n{}",
            ToonSerializer::serialize_meta(&[("schema", "tenants_v4"), ("app", "dokuhub")]),
            ToonSerializer::serialize_table("tenants", &headers, &rows)
        );
        fs::write(self.db_dir.join("tenants.toon"), content).unwrap_or_default();
    }

    pub fn load_tenants(&mut self) {
        if let Ok(content) = fs::read_to_string(self.db_dir.join("tenants.toon")) {
            if let Ok(parsed) = ToonParser::parse(&content) {
                if let Some((_, rows)) = parsed.get("tenants").and_then(|v| v.as_table()) {
                    self.tenants = rows
                        .iter()
                        .filter_map(|r| {
                            if r.len() >= 5 {
                                Some(Tenant {
                                    id: r[0].to_string_lossy(),
                                    name: r[1].to_string_lossy(),
                                    slug: r[2].to_string_lossy(),
                                    plan: r[3].to_string_lossy(),
                                    created_at: r[4].to_string_lossy(),
                                })
                            } else {
                                None
                            }
                        })
                        .collect();
                }
            }
        }
    }

    // --- Users ---
    pub fn save_users(&self) {
        let headers = vec![
            "id".into(),
            "tenant_id".into(),
            "email".into(),
            "name".into(),
            "password".into(),
            "role".into(),
            "status".into(),
            "created_at".into(),
            "avatar".into(),
            "color".into(),
            "quota_bytes".into(),
            "department".into(),
            "supervisor_id".into(),
        ];
        let rows: Vec<Vec<String>> = self
            .users
            .iter()
            .map(|u| {
                vec![
                    u.id.clone(),
                    u.tenant_id.clone(),
                    u.email.clone(),
                    u.name.clone(),
                    u.password.clone(),
                    u.role.as_str().into(),
                    u.status.clone(),
                    u.created_at.clone(),
                    u.avatar.clone(),
                    u.color.clone(),
                    u.quota_bytes.to_string(),
                    u.department.clone(),
                    u.supervisor_id.clone(),
                ]
            })
            .collect();
        let content = format!(
            "{}\n{}",
            ToonSerializer::serialize_meta(&[("schema", "users_v4"), ("app", "dokuhub")]),
            ToonSerializer::serialize_table("users", &headers, &rows)
        );
        fs::write(self.db_dir.join("users.toon"), content).unwrap_or_default();
    }

    pub fn load_users(&mut self) {
        if let Ok(content) = fs::read_to_string(self.db_dir.join("users.toon")) {
            if let Ok(parsed) = ToonParser::parse(&content) {
                if let Some((_, rows)) = parsed.get("users").and_then(|v| v.as_table()) {
                    self.users = rows
                        .iter()
                        .filter_map(|r| {
                            if r.len() >= 8 {
                                let avatar = if r.len() >= 9 { r[8].to_string_lossy() } else { String::new() };
                                let color = if r.len() >= 10 {
                                    let c = r[9].to_string_lossy();
                                    if c.is_empty() { "#007aff".to_string() } else { c }
                                } else {
                                    "#007aff".to_string()
                                };
                                let quota_bytes = if r.len() >= 11 {
                                    r[10].to_string_lossy().parse::<u64>().unwrap_or(21474836480)
                                } else {
                                    21474836480 // 20 GB default for existing seed/admin
                                };
                                let department = if r.len() >= 12 { r[11].to_string_lossy() } else { String::new() };
                                let supervisor_id = if r.len() >= 13 { r[12].to_string_lossy() } else { String::new() };
                                Some(User {
                                    id: r[0].to_string_lossy(),
                                    tenant_id: r[1].to_string_lossy(),
                                    email: r[2].to_string_lossy(),
                                    name: r[3].to_string_lossy(),
                                    password: r[4].to_string_lossy(),
                                    role: Role::from_str(&r[5].to_string_lossy()),
                                    status: r[6].to_string_lossy(),
                                    created_at: r[7].to_string_lossy(),
                                    avatar,
                                    color,
                                    quota_bytes,
                                    department,
                                    supervisor_id,
                                })
                            } else {
                                None
                            }
                        })
                        .collect();
                }
            }
        }
    }

    // --- Files ---
    pub fn save_files(&self) {
        let headers = vec![
            "id".into(),
            "tenant_id".into(),
            "parent_id".into(),
            "name".into(),
            "extension".into(),
            "size_bytes".into(),
            "mime_type".into(),
            "owner_id".into(),
            "is_folder".into(),
            "created_at".into(),
            "updated_at".into(),
        ];
        let rows: Vec<Vec<String>> = self
            .files
            .iter()
            .map(|f| {
                vec![
                    f.id.clone(),
                    f.tenant_id.clone(),
                    f.parent_id.clone(),
                    f.name.clone(),
                    f.extension.clone(),
                    f.size_bytes.to_string(),
                    f.mime_type.clone(),
                    f.owner_id.clone(),
                    f.is_folder.to_string(),
                    f.created_at.clone(),
                    f.updated_at.clone(),
                ]
            })
            .collect();
        let content = format!(
            "{}\n{}",
            ToonSerializer::serialize_meta(&[("schema", "files_v4"), ("app", "dokuhub")]),
            ToonSerializer::serialize_table("files", &headers, &rows)
        );
        fs::write(self.db_dir.join("files.toon"), content).unwrap_or_default();

        // Save file contents
        let contents_dir = self.db_dir.join("file_contents");
        fs::create_dir_all(&contents_dir).unwrap_or_default();
        for file in &self.files {
            if !file.is_folder {
                fs::write(contents_dir.join(format!("{}.dat", file.id)), &file.content)
                    .unwrap_or_default();
            }
        }
    }

    pub fn load_files(&mut self) {
        let contents_dir = self.db_dir.join("file_contents");
        if let Ok(content) = fs::read_to_string(self.db_dir.join("files.toon")) {
            if let Ok(parsed) = ToonParser::parse(&content) {
                if let Some((_, rows)) = parsed.get("files").and_then(|v| v.as_table()) {
                    self.files = rows
                        .iter()
                        .filter_map(|r| {
                            if r.len() >= 11 {
                                let id = r[0].to_string_lossy();
                                let is_folder = r[8].to_string_lossy() == "true";
                                let content = if !is_folder {
                                    fs::read_to_string(contents_dir.join(format!("{}.dat", id)))
                                        .unwrap_or_default()
                                } else {
                                    String::new()
                                };

                                Some(FileItem {
                                    id,
                                    tenant_id: r[1].to_string_lossy(),
                                    parent_id: r[2].to_string_lossy(),
                                    name: r[3].to_string_lossy(),
                                    extension: r[4].to_string_lossy(),
                                    size_bytes: r[5].to_string_lossy().parse().unwrap_or(0),
                                    mime_type: r[6].to_string_lossy(),
                                    owner_id: r[7].to_string_lossy(),
                                    is_folder,
                                    content,
                                    created_at: r[9].to_string_lossy(),
                                    updated_at: r[10].to_string_lossy(),
                                })
                            } else {
                                None
                            }
                        })
                        .collect();
                }
            }
        }
    }

    // --- Shares ---
    pub fn save_shares(&self) {
        let headers = vec![
            "id".into(),
            "tenant_id".into(),
            "file_id".into(),
            "share_type".into(),
            "target_user_id".into(),
            "token".into(),
            "permission".into(),
            "password".into(),
            "expires_at".into(),
            "download_count".into(),
            "requested_by".into(),
            "approver_id".into(),
            "approval_status".into(),
            "request_note".into(),
            "approval_note".into(),
            "target_email".into(),
        ];
        let rows: Vec<Vec<String>> = self
            .shares
            .iter()
            .map(|s| {
                vec![
                    s.id.clone(),
                    s.tenant_id.clone(),
                    s.file_id.clone(),
                    s.share_type.clone(),
                    s.target_user_id.clone(),
                    s.token.clone(),
                    s.permission.clone(),
                    s.password.clone().unwrap_or_default(),
                    s.expires_at.clone().unwrap_or_default(),
                    s.download_count.to_string(),
                    s.requested_by.clone(),
                    s.approver_id.clone(),
                    s.approval_status.clone(),
                    s.request_note.clone(),
                    s.approval_note.clone(),
                    s.target_email.clone(),
                ]
            })
            .collect();
        let content = format!(
            "{}\n{}",
            ToonSerializer::serialize_meta(&[("schema", "shares_v4"), ("app", "dokuhub")]),
            ToonSerializer::serialize_table("shares", &headers, &rows)
        );
        fs::write(self.db_dir.join("shares.toon"), content).unwrap_or_default();
    }

    pub fn load_shares(&mut self) {
        if let Ok(content) = fs::read_to_string(self.db_dir.join("shares.toon")) {
            if let Ok(parsed) = ToonParser::parse(&content) {
                if let Some((_, rows)) = parsed.get("shares").and_then(|v| v.as_table()) {
                    self.shares = rows
                        .iter()
                        .filter_map(|r| {
                            if r.len() >= 10 {
                                let pw = r[7].to_string_lossy();
                                let exp = r[8].to_string_lossy();
                                let approval_status = if r.len() >= 13 && !r[12].to_string_lossy().is_empty() {
                                    r[12].to_string_lossy()
                                } else {
                                    "approved".to_string()
                                };
                                Some(ShareItem {
                                    id: r[0].to_string_lossy(),
                                    tenant_id: r[1].to_string_lossy(),
                                    file_id: r[2].to_string_lossy(),
                                    share_type: r[3].to_string_lossy(),
                                    target_user_id: r[4].to_string_lossy(),
                                    token: r[5].to_string_lossy(),
                                    permission: r[6].to_string_lossy(),
                                    password: if pw.is_empty() { None } else { Some(pw) },
                                    expires_at: if exp.is_empty() { None } else { Some(exp) },
                                    download_count: r[9].to_string_lossy().parse().unwrap_or(0),
                                    requested_by: if r.len() >= 11 { r[10].to_string_lossy() } else { String::new() },
                                    approver_id: if r.len() >= 12 { r[11].to_string_lossy() } else { String::new() },
                                    approval_status,
                                    request_note: if r.len() >= 14 { r[13].to_string_lossy() } else { String::new() },
                                    approval_note: if r.len() >= 15 { r[14].to_string_lossy() } else { String::new() },
                                    target_email: if r.len() >= 16 { r[15].to_string_lossy() } else { String::new() },
                                })
                            } else {
                                None
                            }
                        })
                        .collect();
                }
            }
        }
    }

    // --- Notes ---
    pub fn save_notes(&self) {
        let headers = vec![
            "id".into(),
            "tenant_id".into(),
            "user_id".into(),
            "title".into(),
            "tags".into(),
            "pinned".into(),
            "updated_at".into(),
        ];
        let rows: Vec<Vec<String>> = self
            .notes
            .iter()
            .map(|n| {
                vec![
                    n.id.clone(),
                    n.tenant_id.clone(),
                    n.user_id.clone(),
                    n.title.clone(),
                    n.tags.clone(),
                    n.pinned.to_string(),
                    n.updated_at.clone(),
                ]
            })
            .collect();
        let content = format!(
            "{}\n{}",
            ToonSerializer::serialize_meta(&[("schema", "notes_v4"), ("app", "dokuhub")]),
            ToonSerializer::serialize_table("notes", &headers, &rows)
        );
        fs::write(self.db_dir.join("notes.toon"), content).unwrap_or_default();

        let notes_dir = self.db_dir.join("note_bodies");
        fs::create_dir_all(&notes_dir).unwrap_or_default();
        for n in &self.notes {
            fs::write(notes_dir.join(format!("{}.md", n.id)), &n.content).unwrap_or_default();
        }
    }

    pub fn load_notes(&mut self) {
        let notes_dir = self.db_dir.join("note_bodies");
        if let Ok(content) = fs::read_to_string(self.db_dir.join("notes.toon")) {
            if let Ok(parsed) = ToonParser::parse(&content) {
                if let Some((_, rows)) = parsed.get("notes").and_then(|v| v.as_table()) {
                    self.notes = rows
                        .iter()
                        .filter_map(|r| {
                            if r.len() >= 7 {
                                let id = r[0].to_string_lossy();
                                let body = fs::read_to_string(notes_dir.join(format!("{}.md", id)))
                                    .unwrap_or_default();
                                Some(NoteItem {
                                    id,
                                    tenant_id: r[1].to_string_lossy(),
                                    user_id: r[2].to_string_lossy(),
                                    title: r[3].to_string_lossy(),
                                    tags: r[4].to_string_lossy(),
                                    pinned: r[5].to_string_lossy() == "true",
                                    updated_at: r[6].to_string_lossy(),
                                    content: body,
                                })
                            } else {
                                None
                            }
                        })
                        .collect();
                }
            }
        }
    }

    // --- Rooms ---
    pub fn save_rooms(&self) {
        let headers = vec![
            "id".into(),
            "tenant_id".into(),
            "code".into(),
            "title".into(),
            "host_id".into(),
            "is_active".into(),
            "created_at".into(),
        ];
        let rows: Vec<Vec<String>> = self
            .rooms
            .iter()
            .map(|r| {
                vec![
                    r.id.clone(),
                    r.tenant_id.clone(),
                    r.code.clone(),
                    r.title.clone(),
                    r.host_id.clone(),
                    r.is_active.to_string(),
                    r.created_at.clone(),
                ]
            })
            .collect();
        let content = format!(
            "{}\n{}",
            ToonSerializer::serialize_meta(&[("schema", "rooms_v4"), ("app", "dokuhub")]),
            ToonSerializer::serialize_table("rooms", &headers, &rows)
        );
        fs::write(self.db_dir.join("rooms.toon"), content).unwrap_or_default();
    }

    pub fn load_rooms(&mut self) {
        if let Ok(content) = fs::read_to_string(self.db_dir.join("rooms.toon")) {
            if let Ok(parsed) = ToonParser::parse(&content) {
                if let Some((_, rows)) = parsed.get("rooms").and_then(|v| v.as_table()) {
                    self.rooms = rows
                        .iter()
                        .filter_map(|r| {
                            if r.len() >= 7 {
                                Some(MeetRoom {
                                    id: r[0].to_string_lossy(),
                                    tenant_id: r[1].to_string_lossy(),
                                    code: r[2].to_string_lossy(),
                                    title: r[3].to_string_lossy(),
                                    host_id: r[4].to_string_lossy(),
                                    is_active: r[5].to_string_lossy() == "true",
                                    created_at: r[6].to_string_lossy(),
                                })
                            } else {
                                None
                            }
                        })
                        .collect();
                }
            }
        }
    }

    // --- API Keys ---
    pub fn save_api_keys(&self) {
        let headers = vec![
            "id".into(),
            "tenant_id".into(),
            "name".into(),
            "key".into(),
            "created_at".into(),
        ];
        let rows: Vec<Vec<String>> = self
            .api_keys
            .iter()
            .map(|k| {
                vec![
                    k.id.clone(),
                    k.tenant_id.clone(),
                    k.name.clone(),
                    k.key.clone(),
                    k.created_at.clone(),
                ]
            })
            .collect();
        let content = format!(
            "{}\n{}",
            ToonSerializer::serialize_meta(&[("schema", "api_keys_v4"), ("app", "dokuhub")]),
            ToonSerializer::serialize_table("api_keys", &headers, &rows)
        );
        fs::write(self.db_dir.join("api_keys.toon"), content).unwrap_or_default();
    }

    pub fn load_api_keys(&mut self) {
        if let Ok(content) = fs::read_to_string(self.db_dir.join("api_keys.toon")) {
            if let Ok(parsed) = ToonParser::parse(&content) {
                if let Some((_, rows)) = parsed.get("api_keys").and_then(|v| v.as_table()) {
                    self.api_keys = rows
                        .iter()
                        .filter_map(|r| {
                            if r.len() >= 5 {
                                Some(ApiKeyItem {
                                    id: r[0].to_string_lossy(),
                                    tenant_id: r[1].to_string_lossy(),
                                    name: r[2].to_string_lossy(),
                                    key: r[3].to_string_lossy(),
                                    created_at: r[4].to_string_lossy(),
                                })
                            } else {
                                None
                            }
                        })
                        .collect();
                }
            }
        }
    }

    /// Seed default data for first launch
    pub fn seed_defaults(&mut self) {
        let now = Self::now_str();

        // 1. Tenants
        self.tenants = vec![
            Tenant {
                id: "tenant_idbsd".into(),
                name: "Indonesia Bisnis Digital".into(),
                slug: "idbsd".into(),
                plan: "Enterprise Plus".into(),
                created_at: now.clone(),
            },
        ];

        // 2. Users (RBAC)
        self.users = vec![
            User {
                id: "usr_superadmin".into(),
                tenant_id: "tenant_acme".into(),
                email: "admin@bijios.local".into(),
                name: "Alex Vance (SuperAdmin)".into(),
                password: "password123".into(),
                role: Role::SuperAdmin,
                status: "active".into(),
                created_at: now.clone(),
                avatar: String::new(),
                color: "#007aff".into(),
                quota_bytes: 32212254720, // 30 GB
                department: "Pusat".into(),
                supervisor_id: String::new(),
            },
            User {
                id: "usr_alice".into(),
                tenant_id: "tenant_acme".into(),
                email: "alice@acme.com".into(),
                name: "Alice Wright (Admin)".into(),
                password: "password123".into(),
                role: Role::Admin,
                status: "active".into(),
                created_at: now.clone(),
                avatar: String::new(),
                color: "#10b981".into(),
                quota_bytes: 21474836480, // 20 GB
                department: "Manajemen".into(),
                supervisor_id: "usr_superadmin".into(),
            },
            User {
                id: "usr_bob".into(),
                tenant_id: "tenant_acme".into(),
                email: "bob@acme.com".into(),
                name: "Bob Martin (Staff)".into(),
                password: "password123".into(),
                role: Role::Member,
                status: "active".into(),
                created_at: now.clone(),
                avatar: String::new(),
                color: "#f59e0b".into(),
                quota_bytes: 10737418240, // 10 GB
                department: "Bagian A".into(),
                supervisor_id: "usr_alice".into(),
            },
            User {
                id: "usr_guest".into(),
                tenant_id: "tenant_acme".into(),
                email: "external.client@partner.org".into(),
                name: "External Auditor (Guest)".into(),
                password: "guest".into(),
                role: Role::Guest,
                status: "active".into(),
                created_at: now.clone(),
                avatar: String::new(),
                color: "#64748b".into(),
                quota_bytes: 1073741824, // 1 GB
                department: "Eksternal".into(),
                supervisor_id: String::new(),
            },
        ];

        // 3. Sample Documents (WordCraft, GriidCraft, DeckCraft, PDF, Notes)
        let sample_word_doc = r##"{
            "title": "Dokumentasi Strategi Transformasi Digital 2027",
            "content": "<h1>Strategi Transformasi Digital BijiOS</h1><p>BijiOS adalah ekosistem kerja kolaboratif masa depan yang menggabungkan manajemen berkas multitenant, editor dokumen native (WordCraft, GriidCraft, DeckCraft), konferensi video WebRTC mandiri, dan database berbasis format TOON v4.</p><h2>Pilar Utama</h2><ul><li><strong>Sovereign Cloud:</strong> Seluruh data disimpan dalam server sendiri tanpa ketergantungan pihak ketiga.</li><li><strong>TOON Storage Engine:</strong> Penghematan token LLM hingga 45% dengan representasi data tabular v4.</li><li><strong>macOS Window Experience:</strong> Alur kerja multitasking dengan jendela melayang (floating apps).</li></ul><blockquote>BijiOS dirancang untuk produktivitas tim yang berstandar enterprise dengan privasi tertinggi.</blockquote>"
        }"##;

        let sample_excel_sheet = r##"{
            "title": "Laporan Keuangan & Proyeksi Kuartal IV",
            "sheets": [
                {
                    "name": "Ringkasan Finansial",
                    "cells": {
                        "A1": {"value": "Kategori", "bold": true, "bg": "#e2e8f0"},
                        "B1": {"value": "Target (IDR)", "bold": true, "bg": "#e2e8f0"},
                        "C1": {"value": "Realisasi (IDR)", "bold": true, "bg": "#e2e8f0"},
                        "D1": {"value": "Pencapaian %", "bold": true, "bg": "#e2e8f0"},
                        "A2": {"value": "Pendapatan SaaS Cloud"},
                        "B2": {"value": 850000000, "format": "currency"},
                        "C2": {"value": 920000000, "format": "currency"},
                        "D2": {"formula": "=C2/B2*100", "format": "percent"},
                        "A3": {"value": "Layanan Integrasi API"},
                        "B3": {"value": 320000000, "format": "currency"},
                        "C3": {"value": 345000000, "format": "currency"},
                        "D3": {"formula": "=C3/B3*100", "format": "percent"},
                        "A4": {"value": "Lisensi On-Premise Meet"},
                        "B4": {"value": 450000000, "format": "currency"},
                        "C4": {"value": 510000000, "format": "currency"},
                        "D4": {"formula": "=C4/B4*100", "format": "percent"},
                        "A5": {"value": "TOTAL", "bold": true, "bg": "#fef08a"},
                        "B5": {"formula": "=SUM(B2:B4)", "bold": true, "format": "currency", "bg": "#fef08a"},
                        "C5": {"formula": "=SUM(C2:C4)", "bold": true, "format": "currency", "bg": "#fef08a"},
                        "D5": {"formula": "=C5/B5*100", "bold": true, "format": "percent", "bg": "#fef08a"}
                    }
                }
            ]
        }"##;

        let sample_deck_slides = r##"{
            "title": "BijiOS Pitch Deck - Executive Overview",
            "slides": [
                {
                    "id": "slide_1",
                    "background": "linear-gradient(135deg, #1e293b 0%, #0f172a 100%)",
                    "elements": [
                        {"type": "title", "text": "BijiOS", "x": 100, "y": 140, "size": 56, "color": "#38bdf8"},
                        {"type": "subtitle", "text": "Modern Workspace Platform with macOS Desktop & TOON v4 Engine", "x": 100, "y": 220, "size": 24, "color": "#94a3b8"},
                        {"type": "badge", "text": "RUST POWERED • SELF-HOSTED WEBRTC", "x": 100, "y": 280, "color": "#34d399"}
                    ]
                },
                {
                    "id": "slide_2",
                    "background": "#ffffff",
                    "elements": [
                        {"type": "heading", "text": "Solusi Lengkap Produktivitas", "x": 80, "y": 60, "size": 36, "color": "#0f172a"},
                        {"type": "card", "title": "WordCraft", "desc": "Clean-room Word Editor untuk dokumen resmi .docx", "x": 80, "y": 140, "w": 260, "h": 160, "color": "#2563eb"},
                        {"type": "card", "title": "GriidCraft", "desc": "Spreadsheet canggih dengan formula real-time .xlsx", "x": 370, "y": 140, "w": 260, "h": 160, "color": "#059669"},
                        {"type": "card", "title": "DeckCraft", "desc": "Presentasi interaktif dengan presenter mode .pptx", "x": 660, "y": 140, "w": 260, "h": 160, "color": "#d97706"}
                    ]
                }
            ]
        }"##;

        let sample_pdf_content = "SAMPLE_PDF_BINARY_REPRESENTATION:BijiOS User Manual & Architecture Specification";

        // Folders
        self.files = vec![
            FileItem {
                id: "folder_docs".into(),
                tenant_id: "tenant_acme".into(),
                parent_id: "root".into(),
                name: "Dokumen Resmi".into(),
                extension: "folder".into(),
                size_bytes: 0,
                mime_type: "inode/directory".into(),
                owner_id: "usr_superadmin".into(),
                is_folder: true,
                content: String::new(),
                created_at: now.clone(),
                updated_at: now.clone(),
            },
            FileItem {
                id: "folder_finance".into(),
                tenant_id: "tenant_acme".into(),
                parent_id: "root".into(),
                name: "Keuangan & Akuntansi".into(),
                extension: "folder".into(),
                size_bytes: 0,
                mime_type: "inode/directory".into(),
                owner_id: "usr_alice".into(),
                is_folder: true,
                content: String::new(),
                created_at: now.clone(),
                updated_at: now.clone(),
            },
            FileItem {
                id: "file_word_1".into(),
                tenant_id: "tenant_acme".into(),
                parent_id: "folder_docs".into(),
                name: "Strategi Transformasi Digital 2027.docx".into(),
                extension: "docx".into(),
                size_bytes: sample_word_doc.len(),
                mime_type: "application/vnd.openxmlformats-officedocument.wordprocessingml.document".into(),
                owner_id: "usr_superadmin".into(),
                is_folder: false,
                content: sample_word_doc.into(),
                created_at: now.clone(),
                updated_at: now.clone(),
            },
            FileItem {
                id: "file_excel_1".into(),
                tenant_id: "tenant_acme".into(),
                parent_id: "folder_finance".into(),
                name: "Laporan Finansial Q4 2026.xlsx".into(),
                extension: "xlsx".into(),
                size_bytes: sample_excel_sheet.len(),
                mime_type: "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet".into(),
                owner_id: "usr_alice".into(),
                is_folder: false,
                content: sample_excel_sheet.into(),
                created_at: now.clone(),
                updated_at: now.clone(),
            },
            FileItem {
                id: "file_deck_1".into(),
                tenant_id: "tenant_acme".into(),
                parent_id: "root".into(),
                name: "BijiOS Pitch Deck.pptx".into(),
                extension: "pptx".into(),
                size_bytes: sample_deck_slides.len(),
                mime_type: "application/vnd.openxmlformats-officedocument.presentationml.presentation".into(),
                owner_id: "usr_superadmin".into(),
                is_folder: false,
                content: sample_deck_slides.into(),
                created_at: now.clone(),
                updated_at: now.clone(),
            },
            FileItem {
                id: "file_pdf_1".into(),
                tenant_id: "tenant_acme".into(),
                parent_id: "root".into(),
                name: "Panduan_Arsitektur_BijiOS.pdf".into(),
                extension: "pdf".into(),
                size_bytes: 48920,
                mime_type: "application/pdf".into(),
                owner_id: "usr_superadmin".into(),
                is_folder: false,
                content: sample_pdf_content.into(),
                created_at: now.clone(),
                updated_at: now.clone(),
            },
        ];

        // 4. Sample Shares (Internal & Public Link)
        self.shares = vec![
            ShareItem {
                id: "share_pub_1".into(),
                tenant_id: "tenant_acme".into(),
                file_id: "file_word_1".into(),
                share_type: "public".into(),
                target_user_id: "".into(),
                token: "doku-pub-share-word-77x".into(),
                permission: "view".into(),
                password: None,
                expires_at: None,
                download_count: 14,
                requested_by: "usr_superadmin".into(),
                approver_id: "usr_superadmin".into(),
                approval_status: "approved".into(),
                request_note: String::new(),
                approval_note: String::new(),
                target_email: String::new(),
            },
            ShareItem {
                id: "share_pub_2".into(),
                tenant_id: "tenant_acme".into(),
                file_id: "file_deck_1".into(),
                share_type: "public".into(),
                target_user_id: "".into(),
                token: "doku-pub-share-deck-99z".into(),
                permission: "view".into(),
                password: None,
                expires_at: None,
                download_count: 32,
                requested_by: "usr_superadmin".into(),
                approver_id: "usr_superadmin".into(),
                approval_status: "approved".into(),
                request_note: String::new(),
                approval_note: String::new(),
                target_email: String::new(),
            },
            ShareItem {
                id: "share_int_1".into(),
                tenant_id: "tenant_acme".into(),
                file_id: "file_excel_1".into(),
                share_type: "internal".into(),
                target_user_id: "usr_bob".into(),
                token: "int-bob-excel".into(),
                permission: "edit".into(),
                password: None,
                expires_at: None,
                download_count: 5,
                requested_by: "usr_alice".into(),
                approver_id: "usr_alice".into(),
                approval_status: "approved".into(),
                request_note: String::new(),
                approval_note: String::new(),
                target_email: String::new(),
            },
        ];

        // 5. Notes
        self.notes = vec![
            NoteItem {
                id: "note_1".into(),
                tenant_id: "tenant_acme".into(),
                user_id: "usr_superadmin".into(),
                title: "Rangkuman Rapat Arsitektur TOON & Rust".into(),
                content: "# Rapat Arsitektur\n- Migrasi database ke format TOON v4 berhasil menurunkan overhead token hingga 50%.\n- WebRTC signaling server berjalan langsung di dalam runtime Rust BijiOS.\n- Tampilan desktop macOS dengan window manager floating mendapat feedback positif dari tim UI/UX.".into(),
                tags: "arsitektur,rust,toon".into(),
                pinned: true,
                updated_at: now.clone(),
            },
            NoteItem {
                id: "note_2".into(),
                tenant_id: "tenant_acme".into(),
                user_id: "usr_superadmin".into(),
                title: "Daftar Tugas Kuartal Depan".into(),
                content: "- [x] Implementasi Konektor Google Drive\n- [x] Sinkronisasi RBAC Tenant\n- [ ] Penambahan Template DeckCraft\n- [x] WebRTC Mesh Video Calling".into(),
                tags: "tasks,roadmap".into(),
                pinned: false,
                updated_at: now.clone(),
            },
        ];

        // 6. Meet Room
        self.rooms = vec![
            MeetRoom {
                id: "room_daily".into(),
                tenant_id: "tenant_acme".into(),
                code: "daily-sync-hq".into(),
                title: "Daily Standup & Strategy Room".into(),
                host_id: "usr_superadmin".into(),
                is_active: true,
                created_at: now.clone(),
            },
        ];

        // 7. API Keys
        self.api_keys = vec![
            ApiKeyItem {
                id: "key_1".into(),
                tenant_id: "tenant_acme".into(),
                name: "Production Zapier Integration".into(),
                key: "dkh_live_9f83ac8d120489a74e50882f019b76".into(),
                created_at: now.clone(),
            },
        ];
    }
}

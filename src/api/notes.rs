use crate::api::json_helper::{escape_json, parse_json};
use crate::auth::AuthContext;
use crate::db::{DbHandle, NoteItem};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct NotesApi;

impl NotesApi {
    fn now_str() -> String {
        format!(
            "{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        )
    }

    pub fn list(ctx: &AuthContext, db: &DbHandle) -> (u16, String) {
        let db_lock = db.read().unwrap();
        let items: Vec<String> = db_lock
            .notes
            .iter()
            .filter(|n| n.tenant_id == ctx.tenant_id)
            .map(|n| {
                format!(
                    r#"{{
                        "id": "{}",
                        "title": "{}",
                        "content": "{}",
                        "tags": "{}",
                        "pinned": {},
                        "updated_at": "{}"
                    }}"#,
                    escape_json(&n.id),
                    escape_json(&n.title),
                    escape_json(&n.content),
                    escape_json(&n.tags),
                    n.pinned,
                    escape_json(&n.updated_at)
                )
            })
            .collect();
        (200, format!("[{}]", items.join(",")))
    }

    pub fn save(body: &str, ctx: &AuthContext, db: &DbHandle) -> (u16, String) {
        let json = match parse_json(body) {
            Some(j) => j,
            None => return (400, r#"{"error":"JSON tidak valid"}"#.to_string()),
        };

        let note_id = json.get_str("id");
        let title = json.get_str("title").unwrap_or("Catatan Baru").trim();
        let content = json.get_str("content").unwrap_or("");
        let tags = json.get_str("tags").unwrap_or("");
        let pinned = json.get_bool("pinned").unwrap_or(false);

        let now = Self::now_str();
        let mut db_lock = db.write().unwrap();

        if let Some(id) = note_id {
            if let Some(n) = db_lock.notes.iter_mut().find(|n| n.id == id && n.tenant_id == ctx.tenant_id) {
                n.title = title.to_string();
                n.content = content.to_string();
                n.tags = tags.to_string();
                n.pinned = pinned;
                n.updated_at = now;
                db_lock.save_notes();
                return (200, format!(r#"{{"success":true,"id":"{}"}}"#, escape_json(id)));
            }
        }

        let new_id = format!("note_{}", db_lock.notes.len() + 1);
        let new_note = NoteItem {
            id: new_id.clone(),
            tenant_id: ctx.tenant_id.clone(),
            user_id: ctx.user_id.clone(),
            title: title.to_string(),
            content: content.to_string(),
            tags: tags.to_string(),
            pinned,
            updated_at: now,
        };

        db_lock.notes.push(new_note);
        db_lock.save_notes();

        (201, format!(r#"{{"success":true,"id":"{}"}}"#, escape_json(&new_id)))
    }

    pub fn delete(note_id: &str, ctx: &AuthContext, db: &DbHandle) -> (u16, String) {
        let mut db_lock = db.write().unwrap();
        let pos = db_lock.notes.iter().position(|n| n.id == note_id && n.tenant_id == ctx.tenant_id);

        if let Some(idx) = pos {
            db_lock.notes.remove(idx);
            db_lock.save_notes();
            (200, r#"{"success":true}"#.to_string())
        } else {
            (404, r#"{"error":"Catatan tidak ditemukan"}"#.to_string())
        }
    }
}

use crate::api::json_helper::{escape_json, parse_json};
use crate::auth::AuthContext;
use crate::db::{DbHandle, MeetRoom};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct SignalMessage {
    pub id: u64,
    pub from_peer: String,
    pub to_peer: String,
    pub user_name: String,
    pub msg_type: String, // "offer" | "answer" | "candidate" | "join" | "leave" | "chat"
    pub data: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone)]
pub struct RoomPeer {
    pub peer_id: String,
    pub user_name: String,
    pub joined_at: u64,
    pub last_seen: u64,
}

pub struct MeetSignalingState {
    pub rooms: HashMap<String, Vec<RoomPeer>>,
    pub signals: HashMap<String, Vec<SignalMessage>>, // room_code -> messages
    pub next_msg_id: u64,
}

impl MeetSignalingState {
    pub fn new() -> Self {
        Self {
            rooms: HashMap::new(),
            signals: HashMap::new(),
            next_msg_id: 0,
        }
    }
}

pub type MeetSignalingHandle = Arc<Mutex<MeetSignalingState>>;

pub struct MeetApi;

impl MeetApi {
    fn now_secs() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    pub fn list_rooms(ctx: &AuthContext, db: &DbHandle, signal_state: &MeetSignalingHandle) -> (u16, String) {
        let now = Self::now_secs();
        let mut state = signal_state.lock().unwrap();

        // 1. Prune all inactive / stale peers older than 12 seconds
        for (_, peers) in state.rooms.iter_mut() {
            peers.retain(|p| now.saturating_sub(p.last_seen) < 12);
        }
        state.rooms.retain(|_, peers| !peers.is_empty());

        let db_lock = db.read().unwrap();
        let mut items = Vec::new();

        for r in db_lock.rooms.iter().filter(|r| r.tenant_id == ctx.tenant_id) {
            let active_peers_count = state.rooms.get(&r.code).map(|p| p.len()).unwrap_or(0);
            // Only list rooms that ACTUALLY have at least 1 currently connected member
            if active_peers_count > 0 {
                items.push(format!(
                    r#"{{
                        "id": "{}",
                        "code": "{}",
                        "title": "{}",
                        "host_id": "{}",
                        "is_active": true,
                        "active_peers": {},
                        "created_at": "{}"
                    }}"#,
                    escape_json(&r.id),
                    escape_json(&r.code),
                    escape_json(&r.title),
                    escape_json(&r.host_id),
                    active_peers_count,
                    escape_json(&r.created_at)
                ));
            }
        }

        (200, format!("[{}]", items.join(",")))
    }

    pub fn create_room(body: &str, ctx: &AuthContext, db: &DbHandle) -> (u16, String) {
        let json = match parse_json(body) {
            Some(j) => j,
            None => return (400, r#"{"error":"JSON tidak valid"}"#.to_string()),
        };

        let title = json.get_str("title").unwrap_or("Meeting BijiOS").trim();
        let random_suffix: u32 = (Self::now_secs() % 10000) as u32;
        let code = format!("biji-{}", random_suffix);

        let mut db_lock = db.write().unwrap();
        let new_room = MeetRoom {
            id: format!("room_{}", db_lock.rooms.len() + 1),
            tenant_id: ctx.tenant_id.clone(),
            code: code.clone(),
            title: title.to_string(),
            host_id: ctx.user_id.clone(),
            is_active: true,
            created_at: format!("{}", Self::now_secs()),
        };

        db_lock.rooms.push(new_room);
        db_lock.save_rooms();

        (201, format!(r#"{{"success":true,"code":"{}","title":"{}"}}"#, escape_json(&code), escape_json(title)))
    }

    pub fn handle_signal(
        body: &str,
        signal_state: &MeetSignalingHandle,
    ) -> (u16, String) {
        let json = match parse_json(body) {
            Some(j) => j,
            None => return (400, r#"{"error":"JSON sinyal tidak valid"}"#.to_string()),
        };

        let room_code = json.get_str("room_code").unwrap_or("").trim();
        let from_peer = json.get_str("from_peer").unwrap_or("").trim();
        let to_peer = json.get_str("to_peer").unwrap_or("").trim();
        let msg_type = json.get_str("type").unwrap_or("").trim();
        let data = json.get_str("data").unwrap_or("");
        let user_name = json.get_str("user_name").unwrap_or("Peserta");

        if room_code.is_empty() || from_peer.is_empty() || msg_type.is_empty() {
            return (400, r#"{"error":"Field sinyal kurang lengkap"}"#.to_string());
        }

        let now = Self::now_secs();
        let mut state = signal_state.lock().unwrap();

        // Manage peers in room
        {
            let peers = state.rooms.entry(room_code.to_string()).or_default();
            // Prune stale peers
            peers.retain(|p| now.saturating_sub(p.last_seen) < 12);

            if msg_type == "join" {
                if let Some(existing) = peers.iter_mut().find(|p| p.peer_id == from_peer) {
                    existing.last_seen = now;
                } else {
                    peers.push(RoomPeer {
                        peer_id: from_peer.to_string(),
                        user_name: user_name.to_string(),
                        joined_at: now,
                        last_seen: now,
                    });
                }
            } else if msg_type == "leave" {
                peers.retain(|p| p.peer_id != from_peer);
            } else if let Some(existing) = peers.iter_mut().find(|p| p.peer_id == from_peer) {
                existing.last_seen = now;
            }
        }

        // Store signal message with sequential id
        state.next_msg_id += 1;
        let msg_id = state.next_msg_id;
        let msg = SignalMessage {
            id: msg_id,
            from_peer: from_peer.to_string(),
            to_peer: to_peer.to_string(),
            user_name: user_name.to_string(),
            msg_type: msg_type.to_string(),
            data: data.to_string(),
            timestamp: now,
        };

        let queue = state.signals.entry(room_code.to_string()).or_default();
        queue.push(msg);

        // Keep queue bounded (last 100 messages)
        if queue.len() > 100 {
            queue.drain(0..queue.len() - 100);
        }

        // Return current peer list
        let current_peers: Vec<String> = state
            .rooms
            .get(room_code)
            .map(|peers| {
                peers
                    .iter()
                    .map(|p| {
                        format!(
                            r#"{{"peer_id":"{}","user_name":"{}"}}"#,
                            escape_json(&p.peer_id),
                            escape_json(&p.user_name)
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();

        (200, format!(r#"{{"success":true,"peers":[{}]}}"#, current_peers.join(",")))
    }

    pub fn poll_signals(
        room_code: &str,
        peer_id: &str,
        since_id: u64,
        signal_state: &MeetSignalingHandle,
    ) -> (u16, String) {
        let now = Self::now_secs();
        let mut state = signal_state.lock().unwrap();

        // Update heartbeat for polling peer and prune stale peers
        if let Some(peers) = state.rooms.get_mut(room_code) {
            if let Some(p) = peers.iter_mut().find(|p| p.peer_id == peer_id) {
                p.last_seen = now;
            }
            peers.retain(|p| now.saturating_sub(p.last_seen) < 12);
        }

        let mut out_messages = Vec::new();

        if let Some(queue) = state.signals.get(room_code) {
            for msg in queue {
                // If message is broadcast or specifically for this peer, and from someone else, and strictly newer than since_id
                if (msg.to_peer.is_empty() || msg.to_peer == peer_id) && msg.from_peer != peer_id && msg.id > since_id {
                    out_messages.push(format!(
                        r#"{{
                            "id": {},
                            "from_peer": "{}",
                            "user_name": "{}",
                            "type": "{}",
                            "data": "{}",
                            "timestamp": {}
                        }}"#,
                        msg.id,
                        escape_json(&msg.from_peer),
                        escape_json(&msg.user_name),
                        escape_json(&msg.msg_type),
                        escape_json(&msg.data),
                        msg.timestamp
                    ));
                }
            }
        }

        let peers_list = if let Some(peers) = state.rooms.get(room_code) {
            let p_items: Vec<String> = peers
                .iter()
                .map(|p| {
                    format!(
                        r#"{{"peer_id":"{}","user_name":"{}"}}"#,
                        escape_json(&p.peer_id),
                        escape_json(&p.user_name)
                    )
                })
                .collect();
            p_items.join(",")
        } else {
            String::new()
        };

        let res = format!(
            r#"{{"signals":[{}],"peers":[{}],"server_time":{}}}"#,
            out_messages.join(","),
            peers_list,
            now
        );

        (200, res)
    }
}

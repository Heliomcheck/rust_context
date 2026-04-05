use serde::{Serialize, Deserialize};
use tokio::sync::broadcast;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub username: String,
    pub text: String,
    pub timestamp: u64,
    pub message_id: u64,
    pub event_id: u64,
    pub subgroups: Option<String>
}

// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct ChatMessage {
//     pub event_id: u64,
//     pub subgroups: Option<Vec<String>>
// }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub event_name: String,
    pub data: Option<String>,
    pub event_id: u64,
    pub chat: ChatMessage
}

pub struct User {
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub birthday: String,
    pub id: u64,
    pub name: String,
    pub event_ids: Option<Vec<u64>>,
    pub verif_code: Option<String>,
    pub is_deleted: bool,
    pub is_online: bool
}

pub struct UserStore { // In-memory user store
    pub users: HashMap<u64, User>,           // id → User
    pub users_by_email: HashMap<String, u64>, // email → id
    pub users_by_username: HashMap<String, u64>, // username → id
    pub sessions: HashMap<String, UserSession>, // token → session
    pub next_id: u64,
}

pub struct UserSession {
    pub user_id: u64,
    pub token: String,
    pub expires_at: DateTime<Utc>
}

pub struct AppState {
    pub tx: broadcast::Sender<ChatMessage>,
}
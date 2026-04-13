use tokio::sync::Mutex;
use serde::{Serialize, Deserialize};
use tokio::sync::broadcast;
use chrono::{DateTime, Utc};
use std::{clone, collections::HashMap, sync::Arc};

use crate::user::UserStore;

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
    pub chat: ChatMessage,
    pub list_user_ids: Option<Vec<u64>>
}

pub struct User {
    pub id: u64,
    pub name: String,
    pub username: String,
    pub email: String,
    pub birthday: Option<String>,
    pub avatar_url: Option<String>,

    pub is_deleted: bool,
    pub is_online: bool,
    pub created_at: DateTime<Utc>,
    pub last_online_at: DateTime<Utc>
}

pub struct UserSession {
    pub user_id: u64,
    pub token: HashMap<String, crate::token::TokenStore>,
    pub created_at: DateTime<Utc>
}

#[derive(Clone)]
pub struct AppState {
    pub tx: broadcast::Sender<ChatMessage>,
    pub user_store: Arc<Mutex<UserStore>>,
    pub verification_store: Arc<Mutex<crate::verification::VerificationStore>>
}
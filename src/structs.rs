use serde::{Serialize, Deserialize};
use tokio::sync::broadcast;

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
    pub data: String,
    pub event_id: u64,
    pub chat: ChatMessage
}

pub struct User {
    pub username: Option<String>,
    pub id: u64,
    pub fullname: String,
    pub event_ids: Option<Vec<u64>>
}

pub struct AppState {
    pub tx: broadcast::Sender<ChatMessage>,
}
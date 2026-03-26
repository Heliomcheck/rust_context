use serde::{Serialize, Deserialize};
use tokio::sync::broadcast;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub username: String,
    pub text: String,
    pub timestamp: u64,
}

pub struct AppState {
    pub tx: broadcast::Sender<ChatMessage>,
}
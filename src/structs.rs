use tokio::sync::Mutex;
use serde::{Serialize, Deserialize};
use tokio::sync::broadcast;
use chrono::{DateTime, Utc};
use std::{collections::HashMap, sync::Arc};
use std::option::Option;

use crate::user::UserStore;

use crate::secrets::verification::VerificationStore;
use crate::secrets::token::TokenStore;
use crate::models::{EditUserRequest, RegisterRequest};

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

#[derive(Debug, Clone)]
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
    pub last_online_at: DateTime<Utc>,
    pub tokens: Option<HashMap<String, TokenStore>> // token -> TokenStore
}

impl User { 
    pub fn create(
        user_id: u64,
        username: String, 
        email: String,
        birthday: Option<String>,
        name: String,
        avatar_url: Option<String>,
        tokens: Option<HashMap<String, TokenStore>>
    ) -> Self {
        User {
            username: username,
            email: email,
            birthday: birthday,      // Set default birthday if not provided
            id: user_id,
            name: name,
            avatar_url: avatar_url,

            is_deleted: false,
            is_online: true,
            created_at: Utc::now(),
            last_online_at: Utc::now(),
            tokens: tokens
        }
    }

    pub fn edit(&mut self, payload: EditUserRequest) { // exit origin data, only update
            self.username = payload.username.unwrap_or(self.username.clone());
            self.email = payload.email.unwrap_or(self.email.clone());
            self.birthday = payload.birthday.or(self.birthday.clone());    // Keep existing birthday if not provided
            self.id = self.id;
            self.name = payload.display_name.unwrap_or(self.name.clone());
            self.avatar_url = payload.avatar_url.or(self.avatar_url.clone());

            self.is_deleted = self.is_deleted;
            self.is_online = self.is_online;
            self.created_at = self.created_at;
            self.last_online_at = self.last_online_at;
            self.tokens = self.tokens.clone();
    }

    pub fn add_token(&mut self, token: TokenStore) -> bool {
        if let Some(tokens) = &mut self.tokens {
            tokens.insert(token.token.clone(), token);
            return true;
        } else {
            self.tokens = Some(HashMap::from([(token.token.clone(), token)]));
            return true;
        }
    }

    pub fn remove_token(&mut self, token_str: &str) {
        if let Some(tokens) = &mut self.tokens {
            tokens.remove(token_str);
        }
    }
    
}

pub struct UserSession {
    pub user_id: u64,
    pub token_store: String,
    pub created_at: DateTime<Utc>
}

impl UserSession {
    pub fn create(user_id: u64, token: String,) -> Self  {
        UserSession {
            user_id,
            token_store: token,
            created_at: Utc::now()
        }
    }
}

#[derive(Clone)]
pub struct AppState {
    pub tx: broadcast::Sender<ChatMessage>,
    pub user_store: Arc<Mutex<UserStore>>,
    pub verification_store: Arc<Mutex<VerificationStore>>
}
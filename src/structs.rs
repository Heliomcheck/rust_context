use sqlx::prelude::FromRow;
use tokio::sync::Mutex;
use serde::{Serialize, Deserialize};
use tokio::sync::broadcast;
use chrono::{DateTime, Utc};
use std::sync::Arc;
use std::option::Option;
use sqlx::PgPool;
use utoipa::{ToSchema};

use crate::{
    user_store::UserStore,
    secrets::verification::VerificationStore,
    models::EditUserRequest,
    permissions::*,
};

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
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


#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Events {
    pub event_name: String,
    pub event_id: i64,
    pub description_event: Option<String>,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub color: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub status_id: i16
}

#[derive(Debug, Clone, FromRow, ToSchema)]
pub struct User {
    pub user_id: i64,
    pub display_name: String,
    pub username: String,
    pub email: String,
    pub birthday: Option<String>,
    pub avatar_url: Option<String>, // make access through user_id
    pub description_profile: Option<String>,

    pub is_deleted: bool,
    pub created_at: DateTime<Utc>,
    pub last_online_at: DateTime<Utc>
}

impl User { 
    pub fn create(
        user_id: i64,
        username: String, 
        email: String,
        birthday: Option<String>,
        display_name: String,
        avatar_url: Option<String>,
        description_profile: Option<String>
    ) -> Self {
        User {
            username: username,
            email: email,
            birthday: birthday,      // Set default birthday if not provided
            user_id: user_id,
            description_profile: description_profile,
            display_name: display_name,
            avatar_url: avatar_url,

            is_deleted: false,
            created_at: Utc::now(),
            last_online_at: Utc::now()
        }
    }

    pub fn edit(&mut self, payload: EditUserRequest) { // exit origin data, only update
            self.username = payload.username.unwrap_or(self.username.clone());
            self.email = payload.email.unwrap_or(self.email.clone());
            self.birthday = payload.birthday.or(self.birthday.clone());    // Keep existing birthday if not provided
            self.user_id = self.user_id;
            self.display_name = payload.display_name.unwrap_or(self.display_name.clone());
            self.avatar_url = payload.avatar_url.or(self.avatar_url.clone());

            self.is_deleted = self.is_deleted;
            self.created_at = self.created_at;
            self.last_online_at = self.last_online_at;
    }
}


#[derive(Clone)]
pub struct AppState {
    pub tx: broadcast::Sender<ChatMessage>,
    pub user_store: Arc<Mutex<UserStore>>,
    pub verification_store: Arc<Mutex<VerificationStore>>,
    pub db_pool: PgPool
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Poll {
    pub poll_id: i64,
    pub question: String,
    pub created_by: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub is_active: bool,
    pub more_than_one_vote: bool
}

#[derive(Debug, Serialize, ToSchema)]
pub struct EventParticipant {
    pub user_id: i64,
    pub username: String
}

#[derive(Debug, Serialize, ToSchema)]
pub struct EventUser {
    pub user_id: i64,
    pub event_id: i64,
    pub permissions: EventPermissions,
    pub joined_at: DateTime<Utc>,
}
use chrono::{Utc, DateTime};

use crate::generator::Generator;

#[derive(Debug, Clone)]
pub struct TokenStore {
    pub id: u64,
    pub user_id: u64,
    pub token: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>
}

impl TokenStore {
    pub fn new(user_id: u64, ttl: i64) -> Self { // add id from database later
        Self {
            id: 0,
            user_id,
            token: Generator::new_session_token(),
            created_at: Utc::now(),
            expires_at: Utc::now() + chrono::Duration::days(ttl)
        }
    }

    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    pub fn is_valid(&self, input_token: &str) -> bool {
        if self.is_expired() {
            return false;
        }
        self.token == input_token
    }
}
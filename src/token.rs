use chrono::{Utc, DateTime};

use crate::models::TokenVerifyRequest;
use crate::generator::Generator;

pub struct TokenStore {
    pub token: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>
}

impl TokenStore {
    pub fn new(TTL: i64) -> Self {
        Self {
            token: Generator::new_session_token(),
            created_at: Utc::now(),
            expires_at: Utc::now() + chrono::Duration::days(TTL)
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
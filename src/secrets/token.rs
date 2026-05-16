use chrono::{Utc, DateTime};

use crate::secrets::generator::Generator;

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
//test
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
fn test_token_store() {
    let user_id = 123;
    let ttl = 7;

    let token_store = TokenStore::new(user_id, ttl);

    assert_eq!(token_store.user_id, user_id);
    assert_eq!(token_store.token.len(), 32);
    assert!(token_store.expires_at > token_store.created_at);
    assert!(!token_store.is_expired());
    assert!(token_store.is_valid(&token_store.token));
    assert!(!token_store.is_valid("invalid_token"));
}

#[test]
fn test_expired_token() {
    let token_store = TokenStore::new(123, -1);

    assert!(token_store.is_expired());
    assert!(!token_store.is_valid(&token_store.token));
}
}
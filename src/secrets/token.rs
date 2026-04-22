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
//test
#[cfg(test)]
mod tests {
    use super::*;

    #[test]// Проверяет, что токен считается просроченным при отрицательном TTL
    fn test_token_expired() {
        let token = TokenStore::new(1, -1);
        assert!(token.is_expired());
    }

    #[test]// Проверяет, что токен валиден, если строка совпадает и он не просрочен
    fn test_token_valid() {
        let token = TokenStore::new(1, 1);
        assert!(token.is_valid(&token.token));
    }

    #[test]// Проверяет, что неверная строка токена не проходит валидацию
    fn test_token_invalid_string() {
        let token = TokenStore::new(1, 1);
        assert!(!token.is_valid("wrong_token"));
    }
}
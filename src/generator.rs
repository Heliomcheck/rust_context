use rand::RngExt;
use uuid::Uuid;

pub struct Generator;

impl Generator {
    pub fn verification_code() -> String {  // for email verification, password reset, etc.
        let mut rng = rand::rng();
        let code: u32 = rng.random_range(0..1_000_000);
        format!("{:06}", code)
    }
    
    pub fn new_session_token() -> String {  // for user sessions
        Uuid::new_v4().to_string().replace("-", "")
    }
    
    pub fn api_token() -> String {          // for integration with external services
        let mut rng = rand::rng(); 
        let bytes: Vec<u8> = (0..8).map(|_| rng.random()).collect();
        hex::encode(bytes)
    }
}
//test
#[cfg(test)]
mod tests {
    use super::*;

    #[test]// Проверяет, что код верификации всегда длиной 6 и состоит только из цифр
    fn test_verification_code_length() {
        let code = Generator::verification_code();
        assert_eq!(code.len(), 6);
        assert!(code.chars().all(|c| c.is_digit(10)));
    }

    #[test]// Проверяет, что session token уникален и не содержит дефисов
    fn test_session_token_unique() {
        let t1 = Generator::new_session_token();
        let t2 = Generator::new_session_token();
        assert_ne!(t1, t2);
        assert!(!t1.contains('-'));
    }

    #[test]// Проверяет длину API токена
    fn test_api_token_length() {
        let token = Generator::api_token();
        assert_eq!(token.len(), 16); // hex 8 bytes
    }
}

#[test]// Проверяет отсутствие коллизий на 1000 генераций
fn test_token_uniqueness_bulk() {
    use std::collections::HashSet;
    let mut set = HashSet::new();
    for _ in 0..1000 {
        let token = Generator::new_session_token();
        assert!(set.insert(token));
    }
}
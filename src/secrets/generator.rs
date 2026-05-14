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

    #[test]
    fn test_verification_code() {
    let code = Generator::verification_code();

    assert_eq!(code.len(), 6);
    assert!(code.chars().all(|c| c.is_ascii_digit()));
}

    #[test]
    fn test_new_session_token() {
    let token1 = Generator::new_session_token();
    let token2 = Generator::new_session_token();

    assert_eq!(token1.len(), 32);
    assert_eq!(token2.len(), 32);

    assert_ne!(token1, token2);
}

    #[test]
    fn test_api_token() {
    let token = Generator::api_token();

    assert_eq!(token.len(), 16);
    assert!(token.chars().all(|c| c.is_ascii_hexdigit()));
}
} 
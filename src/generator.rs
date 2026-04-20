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
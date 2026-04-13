use chrono::{DateTime, Utc, Duration};
use std::collections::HashMap;

#[derive(Debug)]
pub struct VerificationCode {
    pub code: String,
    pub email: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>
}

impl VerificationCode {
    pub const DEFAULT_TTL_MINUTES: i64 = 15;

    pub fn new(email: String, code: String, ttl_minutes: i64) -> Self { // Creating struct
        Self {
            code,
            email,
            created_at: Utc::now(),
            expires_at: Utc::now() + Duration::minutes(ttl_minutes)
        }
    }

    pub fn generate(email: String, ttl_minutes: i64) -> Self { // Generate random number
        Self::new(email, 
            crate::generator::Generator::verification_code(), ttl_minutes)
    }

    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    pub fn is_created(&self) -> bool {
        Utc::now() < self.created_at
    }

    pub fn verify(&mut self, input_code: &str) -> bool { // To final verify
        if self.is_expired() {
            return false;
        }
        
        if self.is_created() {
            return false;
        }
        
        self.code == input_code
    }

    pub fn remaining_seconds(&self) -> i64 { // TTL in seconds
        let now = Utc::now();
        if now > self.expires_at {
            0
        } else {
            (self.expires_at - now).num_seconds()
        }
    }
}


#[derive(Debug, Default)]
pub struct VerificationStore {
    codes: HashMap<String, VerificationCode>, // email -> VerificationCode
}

impl VerificationStore {
    pub fn new() -> Self {
        Self {
            codes: HashMap::new(),
        }
    }
    
    pub fn create(&mut self, email: &str, ttl_minutes: i64) -> String { // Create code
        let code = VerificationCode::generate(email.to_string(), ttl_minutes);
        let code_str = code.code.clone();
        self.codes.insert(email.to_string(), code);
        code_str
    }
    
    pub fn create_default(&mut self, email: &str) -> String { // With TTL in default
        self.create(email, VerificationCode::DEFAULT_TTL_MINUTES)
    }
    
    pub fn get(&self, email: &str) -> Option<&VerificationCode> { // Get strust link
        self.codes.get(email)
    }
    
    pub fn get_mut(&mut self, email: &str) -> Option<&mut VerificationCode> { // Get strust link with mut
        self.codes.get_mut(email)
    }

    pub fn verify(&mut self, email: &str, input_code: &str) -> bool { // Verify code
        if let Some(code) = self.codes.get_mut(email) {
            if code.verify(input_code) {
                self.codes.remove(email);
                return true;
            }
        }
        false
    }
    
    pub fn remove(&mut self, email: &str) -> Option<VerificationCode> { // Delete code
        self.codes.remove(email)
    }

    pub fn cleanup_expired(&mut self) -> usize { // Delete all expired codes
        let expired: Vec<String> = self.codes
            .iter()
            .filter(|(_, code)| code.is_expired())
            .map(|(email, _)| email.clone())
            .collect();
        
        let count = expired.len();
        for email in expired {
            self.codes.remove(&email);
        }
        count
    }
}
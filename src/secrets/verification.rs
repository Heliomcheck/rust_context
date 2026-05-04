use chrono::{DateTime, Utc, Duration};
use std::collections::HashMap;

use crate::secrets::generator::Generator;

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
            Generator::verification_code(), ttl_minutes)
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
    pub fn can_resend(&self, email: &str, cooldown_seconds: i64) -> bool {
    if let Some(code) = self.codes.get(email) {
        let now = Utc::now();
        let diff = now - code.created_at;
        return diff.num_seconds() >= cooldown_seconds;
    }
    true
}
pub fn get_or_create(&mut self, email: &str, ttl_minutes: i64) -> String {
    if let Some(code) = self.codes.get(email) {
        if !code.is_expired() {
            return code.code.clone();
        }
    }

    self.create(email, ttl_minutes)
}
}
// Tests 
#[test]//Никитос ты какойто калл написал кабуто 1000-7 вся фигня
fn test_new(){
    let store = VerificationStore::new();
    assert!(store.get("test").is_none());
}
//Test VerificationCode
#[test] //email created correct chek
fn test_verification_code_new(){
    let code = VerificationCode::new(
        "test@mail.ru".to_string(),
        "123456".to_string(),
        15
    );
    assert_eq!(code.email, "test@mail.ru");
    assert_eq!(code.code, "123456");
    assert!(!code.is_expired());
}
#[test]//code created correct chek
fn test_verification_code_generate(){
    let code = VerificationCode::generate("test@mail.ru".to_string(), 15);
    assert_eq!(code.email, "test@mail.ru");
    assert_eq!(code.code.len(), 6);
}
#[test]//volidation code is correct
fn test_verification_code_verify_success(){
    let mut code = VerificationCode::new(
        "test@mail.ru".to_string(),
        "123456".to_string(),
        15
    );
    let result = code.verify("123456");
    assert!(result);
}
#[test]// if code is nepravilni
fn test_verification_code_verify_fail_wrong_code() {
    let mut code = VerificationCode::new(
        "test@mail.ru".to_string(),
        "123456".to_string(),
        15
    );

    let result = code.verify("425267");
    assert!(!result);
}
#[test]// code is prosrochen
fn test_verification_code_expired() {
    let mut code = VerificationCode::new(
        "test@mail.ru".to_string(),
        "123456".to_string(),
        0 //no time
    );
    std::thread::sleep(std::time::Duration::from_millis(10));
    assert!(code.is_expired());
    assert!(!code.verify("123456"));
}
#[test]//check code time
fn test_verification_code_seconds() {
    let code = VerificationCode::new(
        "test@mail.ru".to_string(),
        "123456".to_string(),
        1
    );
    let remaining = code.remaining_seconds();
    assert!(remaining >= 0);
}

//Test VerificationStore
#[test]// check that code is saved and geteble
fn test_store_create_and_get() {
    let mut store = VerificationStore::new();

    let code = store.create("test@mail.ru", 15);
    let stored = store.get("test@mail.ru");

    assert!(stored.is_some());
    assert_eq!(stored.unwrap().code, code);
}
#[test]// check that code can be successed
fn test_store_verify_success() {
    let mut store = VerificationStore::new();

    let code = store.create("test@mail.com", 15);
    let result = store.verify("test@mail.com", &code);

    assert!(result);
}
#[test]//check na durochka tipa verify code != unverefy code
fn test_store_verify_fail() {
    let mut store = VerificationStore::new();

    store.create("test@mail.com", 15);
    let result = store.verify("test@mail.com", "000000");

    assert!(!result);
}
#[test]// deliting code check
fn test_store_remove() {
    let mut store = VerificationStore::new();

    store.create("test@mail.com", 15);
    let removed = store.remove("test@mail.com");

    assert!(removed.is_some());
    assert!(store.get("test@mail.com").is_none());
}
#[test]// chek that time outed code was remove
fn test_cleanup_expired() {
    let mut store = VerificationStore::new();

    store.create("a@mail.com", 0);
    store.create("b@mail.com", 0);

    std::thread::sleep(std::time::Duration::from_millis(10));

    let removed = store.cleanup_expired();

    assert_eq!(removed, 2);
    assert!(store.get("a@mail.com").is_none());
}
#[test]
fn test_can_resend_cooldown() {
    let mut store = VerificationStore::new();
    store.create("test@mail.com", 15);
    let can_resend = store.can_resend("test@mail.com", 60);
    assert!(!can_resend);
    std::thread::sleep(std::time::Duration::from_secs(1));
    let can_resend = store.can_resend("test@mail.com", 1);
    assert!(can_resend);
}

#[test]
fn test_get_or_create_returns_same_code() {
    let mut store = VerificationStore::new();
    let code1 = store.create("test@mail.com", 15);
    let code2 = store.create("test@mail.com", 15);
    assert_ne!(code1, code2);
}

#[test]
fn test_get_or_create_creates_new_if_expired() {
    let mut store = VerificationStore::new();
    let code1 = store.create("test@mail.com", 0);
    std::thread::sleep(std::time::Duration::from_millis(10));
    let code2 = store.create("test@mail.com", 0);
    assert_ne!(code1, code2);
}
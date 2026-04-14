use crate::{structs::{User, UserSession}, token::TokenStore};
use std::{collections::HashMap, ptr::null};
use chrono::{Utc, Duration};
use anyhow::{Context, Result};
use std::convert::Into;

pub struct UserStore { // In-memory user store
    pub users: HashMap<u64, User>,           // id -> User
    pub users_by_email: HashMap<String, u64>, // email -> id
    pub users_by_username: HashMap<String, u64>, // username -> id
    pub sessions: HashMap<String, UserSession>, // token -> session
    pub next_id: u64,
}

impl UserStore {
    pub fn new() -> Self {
        UserStore {
            users: HashMap::new(),
            users_by_email: HashMap::new(),
            users_by_username: HashMap::new(),
            sessions: HashMap::new(),
            next_id: 1,     // Connect to database and get the last id (in future)
        }
    }

    pub fn add_user(
            &mut self, 
            username: String, 
            email: String,
            birthday: Option<String>,
            name: String,
            avatar_url: Option<String>
        ) -> Result<u64, anyhow::Error> {

        if self.users_by_email.contains_key(&email) {
            return Err(anyhow::anyhow!("Email already exists"));
        }

        let user_id = self.next_id;
        self.next_id += 1;

        let user = User {
            username: username.clone(),
            email: email.clone(),
            birthday: birthday.clone(),      // Set default birthday if not provided
            id: user_id,
            name: name.clone(),
            avatar_url: avatar_url.clone(),

            is_deleted: false,
            is_online: true,
            created_at: Utc::now(),
            last_online_at: Utc::now()
        };

        self.users.insert(user_id, user);
        self.users_by_email.insert(email, user_id);
        self.users_by_username.insert(username, user_id);

        Ok(user_id)
    }

    pub fn get_user_by_email(&self, email: &str) -> Option<&User> {
        self.users_by_email.get(email).and_then(|id| self.users.get(id))
    }

    pub fn get_user_by_id(&self, id: u64) -> Option<&User> {
        self.users.get(&id)
    }

    pub fn get_user_by_username(&self, username: &str) -> Option<&User> {
        self.users_by_username.get(username).and_then(|id| self.users.get(id))
    }

    pub fn create_session(&mut self, user_id: u64, token: Option<&String>) -> Result<String, anyhow::Error> {
        dotenvy::dotenv().ok(); // Load .env file to get TTL_VERIFICATION_CODE
        let ttl_hours = std::env::var("TTL_VERIFICATION_CODE")
            .context("Need TTL_VERIFICATION_CODE in .env")?
            .parse::<i64>().context("TTL_VERIFICATION_CODE must be a number")?;

        if let Some(tok) = token {
            let session = UserSession {
                user_id,
                token: HashMap::from([(tok.clone(), TokenStore {
                    token: tok.clone(),
                    created_at: Utc::now(),
                    expires_at: Utc::now() + Duration::hours(ttl_hours)
                })]),
                created_at: Utc::now()
            };
            self.sessions.insert(tok.clone(), session);
            Ok(tok.clone())
        } else {
            let token = TokenStore::new(ttl_hours);
            let token_str = token.token.clone();
            let session = UserSession {
                user_id,
                token: HashMap::from([(token.token.clone(), token)]),
                created_at: Utc::now()
            };
            self.sessions.insert(token_str.clone(), session);
            Ok(token_str)
        }
    }

    pub fn get_session(&self, token: &String) -> Option<&UserSession> {
        self.sessions.get(token)
    }

    pub fn delete_session(&mut self, token: &str) -> Result<(), anyhow::Error> {    // delete session in memory
        let session = self.sessions.remove(token).context("Session not found")?;
    
        let user_id = session.user_id;
    
        if let Some(user) = self.users.remove(&user_id) {
            self.users_by_email.remove(&user.email);
            self.users_by_username.remove(&user.username);
            self.sessions.remove(token);
            if let Some(user) = self.users.get_mut(&user_id) {
                user.is_online = false;
            }
        }
        Ok(())
    }

    pub fn check_username(&self, username: &str) -> bool {
        self.users_by_username.contains_key(username)
    }

    pub fn is_valid_token(&mut self, token: &str) -> bool {
        match self.sessions.get(token) {
            Some(session) => {
                if session.token.get(token).unwrap().expires_at < Utc::now() {
                    self.sessions.remove(token);
                    false
                } else {
                    true
                }
            },
            None => false
        }
    }

    // pub fn verif_email(&mut self, email: &str, code: &str) -> Result<(), anyhow::Error> {
    //     let user = self.get_user_by_email(email).context("User not found")?;
    //     if let Some(verif_code) = &user.verif_code {
    //         if verif_code == code {
    //             // Mark email as verified (this is just a placeholder, you can implement it as needed)
    //             Ok(())
    //         } else {
    //             Err(anyhow::anyhow!("Invalid verification code"))
    //         }
    //     } else {
    //         Err(anyhow::anyhow!("No verification code found for this user"))
    //     }
    // }

    // async pub fn sign_up(
    //     &mut self, 
    //     email: String, 
    //     name: String, 
    //     birthday: Option<String>, 
    //     username: Option<String>) 
    //     -> Result<String, anyhow::Error> {

    //     let user_id = self.add_user(username, email, birthday, name)?;
    //     let token = self.create_session(user_id, None)?;
    //     Ok(token)
    // }
}

// Tests 
#[test]
fn test_new(){
    let store = UserStore::new();
    assert!(store.users.is_empty());
    assert!(store.users_by_email.is_empty());
    assert!(store.users_by_username.is_empty());
    assert!(store.sessions.is_empty());
    assert_eq!(store.next_id, 1);
}

#[test]
fn test_add_user() {
    let mut store = UserStore::new();
    let user_id = store.add_user(
        "test".to_string(),
        "test@example.com".to_string(),
        None,
        "Test User".to_string(),
        None
    );
    assert!(user_id.is_ok());
}

#[test]
fn test_get_user_by_email() {
    let mut store = UserStore::new();
    let user_id = store.add_user(
        "test".to_string(),
        "test@example.com".to_string(),
        None, 
        "Test User".to_string(),
        None
    );
    let user = store.get_user_by_email("test@example.com");
    assert!(user.is_some());
}

#[test]
fn test_get_user_by_id() {
    let mut store = UserStore::new();
    let user_id = store.add_user(
        "test".to_string(),
        "test@example.com".to_string(),
        None,
        "Test User".to_string(),
        None
    );
    let user = store.get_user_by_id(user_id.unwrap());
    assert!(user.is_some());
}

#[test]
fn test_get_user_by_username() {
    let mut store = UserStore::new();
    let user_id = store.add_user(
        "test".to_string(), 
        "test@example.com".to_string(), 
        None, 
        "Test User".to_string(),
        None
    );
    let user = store.get_user_by_username("test");
    assert!(user.is_some());
}

#[test]
fn test_create_session() {
    let mut store = UserStore::new();
    let user_id = store.add_user(
        "test".to_string(),
        "test@example.com".to_string(),
        None,
        "Test User".to_string(),
        None
    );
    let token = store.create_session(user_id.unwrap(), None);
    assert!(token.is_ok());
}

#[test]
fn test_get_session() {
    let mut store = UserStore::new();
    let user_id = store.add_user(
        "test".to_string(),
        "test@example.com".to_string(), 
        None, 
        "Test User".to_string(),
        None
    );
    let token = store.create_session(user_id.unwrap(), None);
    let session = store.get_session(&token.unwrap());
    assert!(session.is_some());
}

#[test]
fn test_delete_session() {
    let mut store = UserStore::new();
    let user_id = store.add_user(
        "test".to_string(),
        "test@example.com".to_string(),
        None,
        "Test User".to_string(),
        None
    );
    let token = store.create_session(user_id.unwrap(), None);
    let delete_result = store.delete_session(&token.unwrap());
    assert!(delete_result.is_ok());
}
#[test]
fn test_check_username_taken() { //s imenem
    let mut store = UserStore::new();
    store.add_user(//With your feet in the air and your head on the ground
        "test".to_string(),//Try this trick and spin it, yeah
        "test@mail.ru".to_string(),//Your head will collapse
        None,//But there's nothing in it
        "Tets name".to_string(),//And you'll ask yourself
        None,//Where is my mind?
    ).unwrap();//Where is my mind?
    let exists = store.check_username("test");//Where is my mind?
    assert!(exists);
}
#[test]
fn test_check_username_untaken() { //bez imeni
    let store = UserStore::new();
    let exists = store.check_username("newhui");
    assert!(!!exists);
}
#[test]
fn test_check_username_empty() { //pusto
    let store = UserStore::new();
    let exists = store.check_username("");
    assert!(!!exists);
}
#[test]
fn test_check_username_spaces() { //probelli ebanya rot
    let mut store = UserStore::new();
    store.add_user(
        "test nmae".to_string(),
        "test@mail.ru".to_string(),
        None,
        "Tets name".to_string(),
        None,
    ).unwrap();
    let exists = store.check_username("test nmae");
    assert!(exists);
}
#[test]
fn test_check_username_register() { //register (T != t)
    let mut store = UserStore::new();
    store.add_user(
        "testName".to_string(),
        "test@mail.ru".to_string(),
        None,
        "Tets Name".to_string(),
        None,
    ).unwrap();
    let exists = store.check_username("testname");
    assert!(!exists);
}
#[test]
fn test_check_username_long() { //dlinno nemnozhko
    let mut store = UserStore::new();
    let long_username = "sigmaboy".repeat(1000);
    store.add_user(
        long_username.clone(),
        "test@mail.ru".to_string(),
        None,
        "Tets name".to_string(),
        None,
    ).unwrap();
    let exists = store.check_username(&long_username);
    assert!(exists);
}
#[test]
fn test_check_username_special_chars() { //special simvoll's
    let mut store = UserStore::new();
    let username = "sigmaboy_123!@#";
    store.add_user(
        username.to_string(),
        "test@mail.ru".to_string(),
        None,
        "Tets name".to_string(),
        None,
    ).unwrap();
    let exists = store.check_username("username");
    assert!(!exists);
}

#[test]
fn test_check_username_unicode() { //Unicode test na niziu (libo mozhno ebnut' test po ip chtob ne vtikali)
    let mut store = UserStore::new();
    let username = "Валерыч";
    store.add_user(
        username.to_string(),
        "test@mail.ru".to_string(),
        None,
        "Tets name".to_string(),
        None,
    ).unwrap();
    let exists = store.check_username("username");
    assert!(!exists);
}
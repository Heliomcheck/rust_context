use crate::{structs::{User, UserSession}, token::TokenStore, user};
use std::{collections::HashMap, f32::consts::E, ptr::null};
use chrono::{Utc, Duration};
use anyhow::{Context, Ok, Result};
use std::convert::Into;
use std::sync::Arc;

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

    pub fn add_user( // add user from database (IN FUTURE)
            &mut self, 
            username: String, 
            email: String,
            birthday: Option<String>,
            name: String,
            avatar_url: Option<String>,
            tokens: Option<HashMap<String, TokenStore>>
        ) -> Result<u64, anyhow::Error> {

        if self.users_by_email.contains_key(&email) { // only in memory, in future check in database
            return Err(anyhow::anyhow!("Email already exists"));
        }

        if self.users_by_username.contains_key(&username) { // only in memory, in future check in database
            return Err(anyhow::anyhow!("Username already exists"));
        }

        let user_id = self.next_id;
        self.next_id += 1;

        let user = User::create(user_id, username.clone(),
            email.clone(), birthday, name, avatar_url, tokens
        );

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

    pub fn create_session(&mut self, user_id: u64, token: &String) -> Result<(), anyhow::Error> {
        
        let user = self.get_user_by_id(user_id) // check if user exists
            .ok_or_else(|| anyhow::anyhow!("User not found"))?;
        
        let tokens = user.tokens.as_ref() // check if user has tokens
            .ok_or_else(|| anyhow::anyhow!("No tokens found for this user"))?;
        
        let token_store = tokens.get(token) // check if token exists for this user
            .ok_or_else(|| anyhow::anyhow!("Token not found for this user"))?;
        
        if !token_store.is_valid(token) { // check if token is valid
            return Err(anyhow::anyhow!("Token has expired"));
        }
        
        let session = UserSession::create(user_id, token.to_string());
        self.sessions.insert(token.to_string(), session);
        Ok(())
    }

    pub fn get_session(&self, token: &str) -> Option<&UserSession> {
        self.get_token_store(token).map(|_| self.sessions.get(token))?
    }

    pub fn delete_session(&mut self, token: &str) -> Result<(), anyhow::Error> {
        let session = self.sessions.remove(token).context("Session not found")?;
        let user_id = session.user_id;
        
        if let Some(user) = self.users.remove(&user_id) {
            self.users_by_email.remove(&user.email);
            self.users_by_username.remove(&user.username);
        }
        
        Ok(())
    }

    pub fn get_token_store(&self, token: &str) -> Option<&TokenStore> {
        self.sessions.get(token).and_then(|session| {
            self.users.get(&session.user_id).and_then(|user| {
                user.tokens.as_ref().and_then(|tokens| tokens.get(token))
            })
        })
    }

    pub fn check_username(&self, username: &str) -> bool {
        self.users_by_username.contains_key(username)
    }

    pub fn is_valid_token(&self, token: &str) -> bool {
        let session = match self.sessions.get(token) { // find user session by token
            Some(s) => s,
            None => return false,
        };
        
        let user = match self.users.get(&session.user_id) { // find user in UserStoer by session user_id
            Some(u) => u,
            None => return false,
        };
        
        match user.tokens.as_ref() {
            Some(tokens) => tokens // check if token exists for this user and is valid
                .get(token)
                .map(|ts| !ts.is_expired())
                .unwrap_or(false),
            None => false,
        }
    }
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
        None,
        Some(HashMap::from([("ffgg".to_string(), TokenStore::new(30))]))
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
        None,
        Some(HashMap::from([("ffgg".to_string(), TokenStore::new(30))]))
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
        None,
        Some(HashMap::from([("ffgg".to_string(), TokenStore::new(30))]))
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
        None,
        Some(HashMap::from([("ffgg".to_string(), TokenStore::new(30))]))
    );
    let user = store.get_user_by_username("test");
    assert!(user.is_some());
}

#[test]
fn test_create_session() {
    let mut store = UserStore::new();
    let token = TokenStore::new(30);
    let token_str = token.token.clone();
    let user_id = store.add_user(
        "test".to_string(),
        "test@example.com".to_string(),
        None,
        "Test User".to_string(),
        None,
        Some(HashMap::from([(token_str.clone(), token)]))
    );
    let token = store.create_session(user_id.unwrap(), &token_str);
    assert!(token.is_ok());
}

#[test]
fn test_get_session() {
    let mut store = UserStore::new();
    let token = TokenStore::new(30);
    let token_str = token.token.clone();
    let user_id = store.add_user(
        "test".to_string(),
        "test@example.com".to_string(), 
        None, 
        "Test User".to_string(),
        None,
        Some(HashMap::from([(token_str.clone(), token)]))
    );
    let _ = store.create_session(user_id.unwrap(), &token_str);
    let session = store.get_session(&token_str);
    assert!(session.is_some());
}

#[test]
fn test_delete_session() {
    let mut store = UserStore::new();
    let token = TokenStore::new(30);
    let token_str = token.token.clone();
    let user_id = store.add_user(
        "test".to_string(),
        "test@example.com".to_string(),
        None,
        "Test User".to_string(),
        None,
        Some(HashMap::from([(token_str.clone(), token)]))
    );
    let token = store.create_session(user_id.unwrap(), &token_str);
    let delete_result = store.delete_session(&token_str);
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
        None,
        Some(HashMap::from([("ffgg".to_string(), TokenStore::new(30))]))
    ).unwrap();//Where is my mind?
    let exists = store.check_username("test");//Where is my mind?
    assert!(exists);
}
#[test]
fn test_check_username_untaken() { //bez imeni
    let store = UserStore::new();
    let exists = store.check_username("newhui");
    assert!(!exists);
}
#[test]
fn test_check_username_empty() { //pusto
    let store = UserStore::new();
    let exists = store.check_username("");
    assert!(!exists);
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
        Some(HashMap::from([("ffgg".to_string(), TokenStore::new(30))]))
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
        Some(HashMap::from([("ffgg".to_string(), TokenStore::new(30))]))
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
        Some(HashMap::from([("ffgg".to_string(), TokenStore::new(30))]))
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
        Some(HashMap::from([("ffgg".to_string(), TokenStore::new(30))]))
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
        Some(HashMap::from([("ffgg".to_string(), TokenStore::new(30))]))
    ).unwrap();
    let exists = store.check_username("username");
    assert!(!exists);
}
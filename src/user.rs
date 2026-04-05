use crate::structs::{UserStore, UserSession, User};
use std::collections::HashMap;
use chrono::{Utc, Duration};
use anyhow::{Context, Result};
use std::convert::Into;
use std::marker::Copy;

impl UserStore {
    pub fn new() -> Self {
        UserStore {
            users: HashMap::new(),
            users_by_email: HashMap::new(),
            users_by_username: HashMap::new(),
            sessions: HashMap::new(),
            next_id: 1, //Connect to database and get the last id (in future)
        }
    }

    pub fn add_user(
            &mut self, 
            mut username: Option<String>, 
            email: String, 
            password_hash: String,
            birthday: impl Into<Option<String>>, //
            name: String
        ) -> Result<u64, anyhow::Error> {

        if self.users_by_email.contains_key(&email) {
            return Err(anyhow::anyhow!("Email already exists"));
        }

        let user_id = self.next_id;
        self.next_id += 1;

        let final_username = username.unwrap_or_else(|| user_id.to_string());

        let user = User {
            username: final_username.clone(),
            email: email.clone(),
            password_hash,
            birthday: birthday.into().unwrap_or_default(), // Set default birthday if not provided
            id: user_id,
            name: name.clone(),
            event_ids: None,
            verif_code: Option::<String>::None,
            is_deleted: false,
            is_online: false
        };

        self.users.insert(user_id, user);
        self.users_by_email.insert(email, user_id);
        self.users_by_username.insert(final_username, user_id);

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

    pub fn create_session(&mut self, user_id: u64) -> Result<String, anyhow::Error> {
        let token = crate::generator::Generator::session_token();
        let expires_at = Utc::now() + Duration::hours(24);

        let session = UserSession {
            user_id,
            token: token.clone(),
            expires_at,
        };

        self.sessions.insert(token.clone(), session);
        Ok(token)
    }

    pub fn get_session(&self, token: &str) -> Option<&UserSession> {
        self.sessions.get(token)
    }

    pub fn delete_sesseon(&mut self, token: &str) -> Result<(), anyhow::Error> { // delete session in memory
        let session = self.sessions.remove(token).context("Session not found")?;
    
        let user_id = session.user_id;
    
        if let Some(user) = self.users.remove(&user_id) {
            self.users_by_email.remove(&user.email);
            self.users_by_username.remove(&user.username);
            if let Some(user) = self.users.get_mut(&user_id) {
                user.is_online = false;
            }
        }
        Ok(())
    }
}
    // Tests 

    #[test]
    fn test_add_user() {
        let mut store = UserStore::new();
        let user_id = store.add_user(
            Some("test".to_string()), 
            "test@example.com".to_string(), 
            "password_hash".to_string(), 
            None, 
            "Test User".to_string());
        assert!(user_id.is_ok());
    }

    #[test]
    fn test_get_user_by_email() {
        let mut store = UserStore::new();
        let user_id = store.add_user(
            Some("test".to_string()), 
            "test@example.com".to_string(), 
            "password_hash".to_string(), 
            None, 
            "Test User".to_string());
        let user = store.get_user_by_email("test@example.com");
        assert!(user.is_some());
    }

    #[test]
    fn test_get_user_by_id() {
        let mut store = UserStore::new();
        let user_id = store.add_user(
            Some("test".to_string()), 
            "test@example.com".to_string(), 
            "password_hash".to_string(), 
            None, 
            "Test User".to_string());
        let user = store.get_user_by_id(user_id.unwrap());
        assert!(user.is_some());
    }

    #[test]
    fn test_get_user_by_username() {
        let mut store = UserStore::new();
        let user_id = store.add_user(
            Some("test".to_string()), 
            "test@example.com".to_string(), 
            "password_hash".to_string(), 
            None, 
            "Test User".to_string());
        let user = store.get_user_by_username("test");
        assert!(user.is_some());
    }
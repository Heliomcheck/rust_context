use crate::user;
use crate::structs::{UserStore, UserSession, User};
use std::collections::HashMap;
use chrono::{DateTime, Utc, Duration};
use anyhow::Result;

impl UserStore {
    pub fn new() -> Self {
        UserStore {
            users: HashMap::new(),
            users_by_email: HashMap::new(),
            users_by_username: HashMap::new(),
            sessions: HashMap::new(),
            next_id: 1,
        }
    }

    pub fn add_user(&mut self, name: String, email: String, password_hash: String) -> Result<u64, anyhow::Error> {
        if self.users_by_email.contains_key(&email) {
            return Err(anyhow::anyhow!("Email already exists"));
        }
        if self.users_by_username.contains_key(&name) {
            return Err(anyhow::anyhow!("Username already exists"));
        }

        let user_id = self.next_id;
        self.next_id += 1;

        let user = User {
            username: user_id.to_string(),
            email: email.clone(),
            password_hash,
            id: user_id,
            name: name.clone(),
            event_ids: None,
            verif_code: String::new()
        };

        self.users.insert(user_id, user);
        self.users_by_email.insert(email, user_id);
        self.users_by_username.insert(name, user_id);

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
        let token = uuid::Builder::nil().into_uuid().to_string(); //refacktor to generate random token
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

    pub fn del_user(&mut self, user_id: u64) -> Result<(), anyhow::Error> { //refacktor to delete all data
        if let Some(user) = self.users.remove(&user_id) {
            if let email = user.email.clone() {
                self.users_by_email.remove(&email);
            }
            if let username = user.username.clone() {
                self.users_by_username.remove(&username);
            }
            Ok(())
        } else {
            Err(anyhow::anyhow!("User not found"))
        }
    }
}


// async fn sign_in(name: String, email: String, password: String) -> Result<(), anyhow::Error> {
//     let user = User {
//         name: name.to_string(),
//         email: email.to_string(),
//         password_hash: password.to_string(),
//     };
//     Ok(())
// }
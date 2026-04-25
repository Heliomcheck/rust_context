use crate::{structs::{User}, TokenStore, user};
use std::{collections::HashMap, f32::consts::E, ptr::null};
use chrono::{Utc, Duration};
use anyhow::{Context, Ok, Result};
use tracing::info;
use std::convert::Into;
use std::sync::Arc;
use sqlx::PgPool;
use anyhow::Error;
use crate::models::EditUserRequest;

use crate::data_base::user_db::*;
use crate::test_utils::*;
pub struct UserStore { // In-memory user store
    pub users: HashMap<i64, User>,           // id -> User
    pub users_by_email: HashMap<String, i64>, // email -> id
    pub users_by_username: HashMap<String, i64>, // username -> id
}

impl UserStore {
    pub fn new() -> Self {
        UserStore {
            users: HashMap::new(),
            users_by_email: HashMap::new(),
            users_by_username: HashMap::new()  // Connect to database and get the last id (in future)
        }
    }

    pub async fn load_from_db(&mut self, pool: &PgPool) -> Result<(), anyhow::Error> {
        self.users.clear(); // clear old hash
        self.users_by_email.clear();
        self.users_by_username.clear();
        
        let users = load_all_users(pool).await?; // load users from db
        
        for user in users {
            let user_id = user.user_id;
            self.users.insert(user_id, user.clone());
            self.users_by_email.insert(user.email.clone(), user_id);
            self.users_by_username.insert(user.username.clone(), user_id);
        }
        
        info!("Loaded {} users into cache", self.users.len());
        Ok(())
    }

    pub async fn add_user( // add user from database (IN FUTURE) p.s. future is coming ;)
            &mut self,
            user_id: i64, 
            username: String, 
            email: String,
            birthday: Option<String>,
            name: String,
            avatar_url: Option<String>,
            description: Option<String>,
            _: &PgPool
        ) -> Result<i64, anyhow::Error> {

        if self.users_by_email.contains_key(&email) { // only in memory, in future check in database
            return Err(anyhow::anyhow!("Email already exists"));
        }

        if self.users_by_username.contains_key(&username) { // only in memory, in future check in database
            return Err(anyhow::anyhow!("Username already exists"));
        }

        let user = User::create(user_id, username.clone(),
            email.clone(), birthday.clone(), name.clone(), avatar_url.clone(), description.clone()
        );

        self.users.insert(user_id, user);
        self.users_by_email.insert(email.to_string(), user_id);
        self.users_by_username.insert(username.to_string(), user_id);

        Ok(user_id)
    }

    // pub async fn edit_user(&mut self, payload: EditUserRequest) {
    //     if let Some(user_id) = self.users_by_username.get(&payload.username.unwrap_or_else(|| )) {
    //         if let Some(user) = self.users.get_mut(user_id) {
    //             user.name = payload.display_name.clone().unwrap_or_else(|| user.name.clone());
    //             user.email = payload.email.clone().unwrap_or_else(|| user.email.clone());
    //             user.birthday = payload.birthday.clone().or_else(|| user.birthday.clone());
    //             user.avatar_url = payload.avatar_url.clone().or_else(|| user.avatar_url.clone());
    //         }
    //     }
    // }

    pub fn get_user_by_email(&self, email: &str) -> Option<&User> {
        self.users_by_email.get(email).and_then(|id| self.users.get(id))
    }

    pub fn get_user_by_id(&self, id: i64) -> Option<&User> {
        self.users.get(&id)
    }

    pub fn get_user_by_username(&self, username: &str) -> Option<&User> {
        self.users_by_username.get(username).and_then(|id| self.users.get(id))
    }

    pub fn check_username(&self, username: &str) -> bool { // refactor
        self.users_by_username.contains_key(username)
    }
}

// Tests 
#[test]
fn test_new(){
    let store = UserStore::new();
    assert!(store.users.is_empty());
    assert!(store.users_by_email.is_empty());
    assert!(store.users_by_username.is_empty());
}

#[tokio::test]
async fn test_add_user() -> anyhow::Result<()> {
    let mut store = UserStore::new();
    let pool = setup_test_db().await;
    let user_id = store.add_user(
        10,
        "test".to_string(),
        "test@example.com".to_string(),
        None,
        "Test User".to_string(),
        None,
        Some("test".to_string()),
        &pool
    );
    assert!(user_id.await.is_ok());

    Ok(())
}

#[tokio::test]
async fn test_get_user_by_email() -> anyhow::Result<()> {
    let mut store = UserStore::new();
    let pool = setup_test_db().await;
    let _ = store.add_user(
        10,
        "test".to_string(),
        "test@example.com".to_string(),
        None, 
        "Test User".to_string(),
        None,
        Some("test".to_string()),
        &pool
    ).await?;
    let user = store.get_user_by_email("test@example.com");
    assert!(user.is_some());
    Ok(())
}

#[tokio::test]
async fn test_get_user_by_id() -> anyhow::Result<()> {
    let pool = setup_test_db().await;
    let mut store = UserStore::new();
    let user_id = store.add_user(
        10,
        "test".to_string(),
        "test@example.com".to_string(),
        None,
        "Test User".to_string(),
        None,
        Some("test".to_string()),
        &pool
    ).await?;
    let user = store.get_user_by_id(user_id);
    assert!(user.is_some());
    Ok(())
}

#[tokio::test]
async fn test_get_user_by_username() -> anyhow::Result<()> {
    let pool = setup_test_db().await;

    let _ = create_user_db(
        &pool,
        "testuser",
        "test@example.com",
        "Test User",
        &None,
        &None,
        &Some("test".to_string())
    )
    .await?;

    // Ищем пользователя через БД
    let user = find_user_by_username(&pool, "testuser").await?;
    assert!(user.is_some());
    assert_eq!(user.unwrap().username, "testuser");

    Ok(())
}

#[tokio::test]
async fn test_check_username_taken() -> anyhow::Result<()> { //s imenem
    let pool = setup_test_db().await;

    let mut store = UserStore::new();
    store.add_user(//With your feet in the air and your head on the ground
        10,
        "test".to_string(),//Try this trick and spin it, yeah
        "test@mail.ru".to_string(),//Your head will collapse
        None,//But there's nothing in it
        "Tets name".to_string(),//And you'll ask yourself
        None,
        Some("test".to_string()),
        &pool
    ).await?;
    let exists = store.check_username("test");
    assert!(exists);
    Ok(())
}
#[tokio::test]
async fn test_check_username_untaken() -> anyhow::Result<()> { //bez imeni
    let store = UserStore::new();
    let exists = store.check_username("newhui");
    assert!(!exists);
    Ok(())
}
#[tokio::test]
async fn test_check_username_empty() -> anyhow::Result<()> { //pusto
    let store = UserStore::new();
    let exists = store.check_username("");
    assert!(!exists);
    Ok(())
}
#[tokio::test]
async fn test_check_username_spaces() -> anyhow::Result<()> { //probelli ebanya rot
    let pool = setup_test_db().await;

    let mut store = UserStore::new();
    store.add_user(
        10,
        "test nmae".to_string(),
        "test@mail.ru".to_string(),
        None,
        "Tets name".to_string(),
        None,
        Some("test".to_string()),
        &pool
    ).await?;
    let exists = store.check_username("test nmae");
    assert!(exists);
    Ok(())
}
#[tokio::test]
async fn test_check_username_register() -> anyhow::Result<()> { //register (T != t)
    let pool = setup_test_db().await;

    let mut store = UserStore::new();
    store.add_user(
        10,
        "testName".to_string(),
        "test@mail.ru".to_string(),
        None,
        "Tets Name".to_string(),
        None,
        Some("test".to_string()),
        &pool
    ).await?;
    let exists = store.check_username("testname");
    assert!(!exists);
    Ok(())
}
#[tokio::test]
async fn test_check_username_long() -> anyhow::Result<()> { //dlinno nemnozhko
    let pool = setup_test_db().await;

    let mut store = UserStore::new();
    let long_username = "sigmaboy".repeat(1000);
    store.add_user(
        10,
        long_username.clone(),
        "test@mail.ru".to_string(),
        None,
        "Tets name".to_string(),
        None,
        Some("test".to_string()),
        &pool
    ).await?;
    let exists = store.check_username(&long_username);
    assert!(exists);
    Ok(())
}

#[tokio::test]
async fn test_check_username_special_chars() -> anyhow::Result<()> { //special simvoll's
    let pool = setup_test_db().await;

    let mut store = UserStore::new();
    let username = "sigmaboy_123!@#";
    store.add_user(
        10,
        username.to_string(),
        "test@mail.ru".to_string(),
        None,
        "Tets name".to_string(),
        None,
        Some("test".to_string()),
        &pool
    ).await?;
    let exists = store.check_username("username");
    assert!(!exists);
    Ok(())
}

#[tokio::test]
async fn test_check_username_unicode() -> anyhow::Result<()> { //Unicode test na niziu (libo mozhno ebnut' test po ip chtob ne vtikali)
    let pool = setup_test_db().await;

    let mut store = UserStore::new();
    let username = "Валерыч";
    store.add_user(
        10,
        username.to_string(),
        "test@mail.ru".to_string(),
        None,
        "Tets name".to_string(),
        None,
        Some("test".to_string()),
        &pool
    ).await?;
    let exists = store.check_username("username");
    assert!(!exists);
    Ok(())
}
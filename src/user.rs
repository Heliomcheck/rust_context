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

use crate::secrets::*;
use crate::data_base::user_db::*;

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

    pub async fn add_user( // add user from database (IN FUTURE) p.s. future is coming ;)
            &mut self, 
            username: String, 
            email: String,
            birthday: Option<String>,
            name: String,
            avatar_url: Option<String>,
            pool: &PgPool
        ) -> Result<i64, anyhow::Error> {

        if self.users_by_email.contains_key(&email) { // only in memory, in future check in database
            return Err(anyhow::anyhow!("Email already exists"));
        }

        if self.users_by_username.contains_key(&username) { // only in memory, in future check in database
            return Err(anyhow::anyhow!("Username already exists"));
        }

        let user_id = create_user_db(pool, &username, &email, &name, &birthday, &avatar_url).await?;
        
        let user = User::create(user_id, username.clone(),
            email.clone(), birthday.clone(), name.clone(), avatar_url.clone()
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
    dotenvy::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL")
        .context("DATABASE_URL not set")?;

    let mut store = UserStore::new();
    let pool = create_pool(&database_url).await?;
    let user_id = store.add_user(
        "test".to_string(),
        "test@example.com".to_string(),
        None,
        "Test User".to_string(),
        None,
        &pool
    );
    assert!(user_id.await.is_ok());

    Ok(())
}

#[tokio::test]
async fn test_get_user_by_email() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL")
        .context("DATABASE_URL not set")?;

    let mut store = UserStore::new();
    let pool = create_pool(&database_url).await?;
    let _ = store.add_user(
        "test".to_string(),
        "test@example.com".to_string(),
        None, 
        "Test User".to_string(),
        None,
        &pool
    ).await?;
    let user = store.get_user_by_email("test@example.com");
    assert!(user.is_some());
    Ok(())
}

#[tokio::test]
async fn test_get_user_by_id() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL")
        .context("DATABASE_URL not set")?;
    let pool = create_pool(&database_url).await?;

    let mut store = UserStore::new();
    let user_id = store.add_user(
        "test".to_string(),
        "test@example.com".to_string(),
        None,
        "Test User".to_string(),
        None,
        &pool
    ).await?;
    let user = store.get_user_by_id(user_id);
    assert!(user.is_some());
    Ok(())
}

#[tokio::test]
async fn test_get_user_by_username() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL")
        .context("DATABASE_URL not set")?;
    let pool = create_pool(&database_url).await?;

    let mut store = UserStore::new();
    let _ = store.add_user(
        "test".to_string(), 
        "test@example.com".to_string(), 
        None, 
        "Test User".to_string(),
        None,
        &pool
    ).await?;
    let user = store.get_user_by_username("test");
    assert!(user.is_some());
    Ok(())
}

#[tokio::test]
async fn test_check_username_taken() -> anyhow::Result<()> { //s imenem
    dotenvy::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL")
        .context("DATABASE_URL not set")?;
    let pool = create_pool(&database_url).await?;

    let mut store = UserStore::new();
    store.add_user(//With your feet in the air and your head on the ground
        "test".to_string(),//Try this trick and spin it, yeah
        "test@mail.ru".to_string(),//Your head will collapse
        None,//But there's nothing in it
        "Tets name".to_string(),//And you'll ask yourself
        None,
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
    dotenvy::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL")
        .context("DATABASE_URL not set")?;
    let pool = create_pool(&database_url).await?;

    let mut store = UserStore::new();
    store.add_user(
        "test nmae".to_string(),
        "test@mail.ru".to_string(),
        None,
        "Tets name".to_string(),
        None,
        &pool
    ).await?;
    let exists = store.check_username("test nmae");
    assert!(exists);
    Ok(())
}
#[tokio::test]
async fn test_check_username_register() -> anyhow::Result<()> { //register (T != t)
    dotenvy::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL")
        .context("DATABASE_URL not set")?;
    let pool = create_pool(&database_url).await?;

    let mut store = UserStore::new();
    store.add_user(
        "testName".to_string(),
        "test@mail.ru".to_string(),
        None,
        "Tets Name".to_string(),
        None,
        &pool
    ).await?;
    let exists = store.check_username("testname");
    assert!(!exists);
    Ok(())
}
#[tokio::test]
async fn test_check_username_long() -> anyhow::Result<()> { //dlinno nemnozhko
    dotenvy::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL")
        .context("DATABASE_URL not set")?;
    let pool = create_pool(&database_url).await?;

    let mut store = UserStore::new();
    let long_username = "sigmaboy".repeat(1000);
    store.add_user(
        long_username.clone(),
        "test@mail.ru".to_string(),
        None,
        "Tets name".to_string(),
        None,
        &pool
    ).await?;
    let exists = store.check_username(&long_username);
    assert!(exists);
    Ok(())
}

#[tokio::test]
async fn test_check_username_special_chars() -> anyhow::Result<()> { //special simvoll's
    dotenvy::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL")
        .context("DATABASE_URL not set")?;
    let pool = create_pool(&database_url).await?;

    let mut store = UserStore::new();
    let username = "sigmaboy_123!@#";
    store.add_user(
        username.to_string(),
        "test@mail.ru".to_string(),
        None,
        "Tets name".to_string(),
        None,
        &pool
    ).await?;
    let exists = store.check_username("username");
    assert!(!exists);
    Ok(())
}

#[tokio::test]
async fn test_check_username_unicode() -> anyhow::Result<()> { //Unicode test na niziu (libo mozhno ebnut' test po ip chtob ne vtikali)
    dotenvy::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL")
        .context("DATABASE_URL not set")?;
    let pool = create_pool(&database_url).await?;

    let mut store = UserStore::new();
    let username = "Валерыч";
    store.add_user(
        username.to_string(),
        "test@mail.ru".to_string(),
        None,
        "Tets name".to_string(),
        None,
        &pool
    ).await?;
    let exists = store.check_username("username");
    assert!(!exists);
    Ok(())
}
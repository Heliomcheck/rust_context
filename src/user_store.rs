use crate::{structs::{User}, TokenStore, user_store};
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
            display_name: String,
            avatar_url: Option<String>,
            description_profile: Option<String>,
            _: &PgPool
        ) -> Result<i64, anyhow::Error> {

        if self.users_by_email.contains_key(&email) { // only in memory, in future check in database
            return Err(anyhow::anyhow!("Email already exists"));
        }

        if self.users_by_username.contains_key(&username) { // only in memory, in future check in database
            return Err(anyhow::anyhow!("Username already exists"));
        }

        let user = User::create(user_id, username.clone(),
            email.clone(), birthday.clone(), display_name.clone(), avatar_url.clone(), description_profile.clone()
        );

        self.users.insert(user_id, user);
        self.users_by_email.insert(email.to_string(), user_id);
        self.users_by_username.insert(username.to_string(), user_id);

        Ok(user_id)
    }



    pub async fn get_user_by_id(
        &mut self,
        pool: &PgPool,
        user_id: i64,
    ) -> Result<Option<User>, anyhow::Error> {
        // 1. Проверяем кэш
        if let Some(user) = self.users.get(&user_id) {
            return Ok(Some(user.clone()));
        }

        // 2. Ищем в БД
        let user_opt = find_user_by_id(pool, user_id).await?;

        // 3. Если нашли – добавляем в кэш
        if let Some(ref user) = user_opt {
            self.users.insert(user_id, user.clone());
            self.users_by_email.insert(user.email.clone(), user_id);
            self.users_by_username.insert(user.username.clone(), user_id);
        }

        Ok(user_opt)
    }

    pub async fn get_user_by_email(
        &mut self,
        pool: &PgPool,
        email: &str,
    ) -> Result<Option<User>, anyhow::Error> {
        if let Some(user) = self.users_by_email.get(email).and_then(|id| self.users.get(id)) {
            return Ok(Some(user.clone()));
        }

        let user_opt = find_user_by_email(pool, email).await?;

        if let Some(ref user) = user_opt {
            self.users.insert(user.user_id, user.clone());
            self.users_by_email.insert(user.email.clone(), user.user_id);
            self.users_by_username.insert(user.username.clone(), user.user_id);
        }

        Ok(user_opt)
    }

    pub async fn get_user_by_username(
        &mut self,
        pool: &PgPool,
        username: &str,
    ) -> Result<Option<User>, anyhow::Error> {
        if let Some(user) = self.users_by_username.get(username).and_then(|id| self.users.get(id)) {
            return Ok(Some(user.clone()));
        }

        let user_opt = find_user_by_username(pool, username).await?;

        if let Some(ref user) = user_opt {
            self.users.insert(user.user_id, user.clone());
            self.users_by_email.insert(user.email.clone(), user.user_id);
            self.users_by_username.insert(user.username.clone(), user.user_id);
        }

        Ok(user_opt)
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
    let user = store.get_user_by_email(&pool,"test@example.com").await?;
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
    let user = store.get_user_by_id(&pool, user_id).await?;
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
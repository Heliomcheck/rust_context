use std::{
    collections::HashMap
};
use anyhow::{
    Ok, 
    Result
};
use tracing::info;
use sqlx::PgPool;

use crate::{
    data_base::user_db::*,
    structs::User
};


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
            email.clone(), birthday.clone(), display_name.clone(), description_profile.clone()
        );

        self.users.insert(user_id, user);
        self.users_by_email.insert(email.to_string(), user_id);
        self.users_by_username.insert(username.to_string(), user_id);

        Ok(user_id)
    }

    #[allow(dead_code)]
    pub fn get_user_by_id(&self, id: i64) -> Option<&User> {
        self.users.get(&id)
    }

    #[allow(dead_code)]
    pub fn get_user_by_email(&self, email: &str) -> Option<&User> {
        self.users_by_email.get(email).and_then(|id| self.users.get(id))
    }

    #[allow(dead_code)]
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

    #[allow(dead_code)]
    pub fn check_username(&self, username: &str) -> bool { // refactor
        self.users_by_username.contains_key(username)
    }
}

//test
    #[cfg(test)]
mod tests {
    use anyhow::Ok;
    use crate::UserStore;
    use crate::data_base::user_db::*;
    use crate::test_utils::*;

    #[test]
    fn test_new() {
        let store = UserStore::new();
        assert!(store.users.is_empty());
        assert!(store.users_by_email.is_empty());
        assert!(store.users_by_username.is_empty());
    }

    #[tokio::test]
    async fn test_add_user() -> anyhow::Result<()> {
        let mut store = UserStore::new();
        let pool = setup_test_db().await;
        let result = store.add_user(
            10,
            "test".to_string(),
            "test@example.com".to_string(),
            None,
            "Test User".to_string(),
            None,
            &pool
        ).await;
        assert!(result.is_ok());
        Ok(())
    }

    #[tokio::test]
    async fn test_get_user_by_email() -> anyhow::Result<()> {
        let mut store = UserStore::new();
        let pool = setup_test_db().await;
        store.add_user(
            10,
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
        let pool = setup_test_db().await;
        let mut store = UserStore::new();
        let user_id = store.add_user(
            10,
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
        let pool = setup_test_db().await;
        let mut store = UserStore::new();
        store.add_user(
            10,
            "test".to_string(),
            "test@example.com".to_string(),
            None,
            "Test User".to_string(),
            None,
            &pool
        ).await?;
        let user = store.get_user_by_username(&pool, "test").await?;
        assert!(user.is_some());
        Ok(())
    }

    #[tokio::test]
    async fn test_load_from_db() -> anyhow::Result<()> {
        let pool = setup_test_db().await;
        let user_id = create_user_db(
            &pool,
            "dbuser",
            "dbuser@mail.com",
            "DB User",
            &None,
            &None,
        ).await?;

        let mut store = UserStore::new();
        store.load_from_db(&pool).await?;

        let user = store.get_user_by_id(user_id);
        assert!(user.is_some());
        assert_eq!(user.unwrap().user_id, user_id);
        Ok(())
    }

    #[tokio::test]
    async fn test_check_username_taken() -> anyhow::Result<()> {
        let pool = setup_test_db().await;
        let mut store = UserStore::new();
        store.add_user(
            10,
            "test".to_string(),
            "test@mail.ru".to_string(),
            None,
            "Tets name".to_string(),
            None,
            &pool
        ).await?;
        let exists = store.check_username("test");
        assert!(exists);
        Ok(())
    }

    #[tokio::test]
    async fn test_check_username_untaken() {
        let store = UserStore::new();
        let exists = store.check_username("newhui");
        assert!(!exists);
    }

    #[tokio::test]
    async fn test_check_username_empty() {
        let store = UserStore::new();
        let exists = store.check_username("");
        assert!(!exists);
    }

    #[tokio::test]
    async fn test_check_username_spaces() -> anyhow::Result<()> {
        let pool = setup_test_db().await;
        let mut store = UserStore::new();
        store.add_user(
            10,
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
    async fn test_check_username_register() -> anyhow::Result<()> {
        let pool = setup_test_db().await;
        let mut store = UserStore::new();
        store.add_user(
            10,
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
    async fn test_check_username_long() -> anyhow::Result<()> {
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
            &pool
        ).await?;
        let exists = store.check_username(&long_username);
        assert!(exists);
        Ok(())
    }

    #[tokio::test]
    async fn test_check_username_special_chars() -> anyhow::Result<()> {
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
            &pool
        ).await?;
        let exists = store.check_username("username");
        assert!(!exists);
        Ok(())
    }

    #[tokio::test]
    async fn test_check_username_unicode() -> anyhow::Result<()> {
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
            &pool
        ).await?;
        let exists = store.check_username("username");
        assert!(!exists);
        Ok(())
    }
}
use sqlx::postgres::{PgPoolOptions, PgPool};
use anyhow::{Context, Ok};
use crate::test_utils::*;

use chrono::{DateTime, Utc};

use crate::structs::User;


pub async fn create_pool(database_url: &str) -> Result<PgPool, anyhow::Error> {
    let pool = PgPoolOptions::new()
    .max_connections(10)
    .connect(database_url)
    .await
    .context("Failed to connect to database")?;

    sqlx::migrate!().run(&pool).await?;
    Ok(pool)
}

pub async fn create_user_db(
    pool: &PgPool,
    username: &str,
    email: &str,
    name: &str,
    birthday: &Option<String>,
    avatar_url: &Option<String>,
) -> Result<i64, anyhow::Error> {
    let row = sqlx::query!(
        r#"
        INSERT INTO users (username, email, name, birthday, avatar_url)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id
        "#, 
        username, 
        email, 
        name, 
        birthday.clone(), 
        avatar_url.clone()
    )
    .fetch_one(pool)
    .await
    .context("Failed to create user")?;
    
    Ok(row.id)
}

pub async fn edit_user_db(
    pool: &PgPool,
    user_id: i64,
    username: Option<&str>,
    email: Option<&str>,
    name: Option<&str>,
    birthday: Option<&str>,
    avatar_url: Option<&str>
) -> Result<(), anyhow::Error> {
    sqlx::query(
        r#"
        UPDATE users
        SET username = COALESCE($1, username),
            email = COALESCE($2, email),
            name = COALESCE($3, name),
            birthday = COALESCE($4, birthday),
            avatar_url = COALESCE($5, avatar_url)
        WHERE id = $6
        "#
    )
    .bind(username)
    .bind(email)
    .bind(name)
    .bind(birthday)
    .bind(avatar_url)
    .bind(user_id)
    .execute(pool)
    .await
    .context("Failed to edit user")?;
    
    Ok(())
} // add delete user later, check later

pub async fn find_user_by_email(pool: &PgPool, email: &str) -> Result<Option<User>, anyhow::Error> {
    let user = sqlx::query_as::<_, User>(
        r#"
        SELECT id, username, email, name, birthday, avatar_url,
               is_deleted, created_at, last_online_at
        FROM users
        WHERE email = $1 AND is_deleted = false
        "#
    )
    .bind(email)
    .fetch_optional(pool)
    .await
    .context("Failed to find user by email")?;
    
    Ok(user)
}

pub async fn find_user_by_token(pool: &PgPool, token: &str) -> Result<Option<User>, anyhow::Error> {
    let user = sqlx::query_as::<_, User>(
        r#"
        SELECT u.id, u.username, u.email, u.name, u.birthday, u.avatar_url,
               u.is_deleted, u.created_at, u.last_online_at
        FROM users u
        JOIN tokenstore t ON u.id = t.user_id
        WHERE t.token = $1 AND t.is_active = true AND t.expires_at > NOW()
        "#
    )
    .bind(token)
    .fetch_optional(pool)
    .await
    .context("Failed to find user by token")?;
    
    Ok(user)
}

pub async fn find_user_by_id(pool: &PgPool, user_id: i64) -> Result<Option<User>, anyhow::Error> {
    let user = sqlx::query_as!(
        User,
        r#"
        SELECT id, username, email, name, birthday, avatar_url,
               is_deleted, created_at, last_online_at
        FROM users
        WHERE id = $1 AND is_deleted = false
        "#,
        user_id  // ← параметр передаётся сюда, а не через .bind()
    )
    .fetch_optional(pool) 
    .await
    .context("Failed to find user by id")?;
    
    Ok(user)
}

pub async fn find_user_by_username(
    pool: &PgPool,
    username: &str,
) -> Result<Option<User>, anyhow::Error> {
    let user = sqlx::query_as!(
        User,
        r#"
        SELECT id, username, email, name, birthday, avatar_url,
               is_deleted, created_at, last_online_at
        FROM users
        WHERE username = $1 AND is_deleted = false
        "#,
        username
    )
    .fetch_optional(pool)
    .await
    .context("Failed to find user by username")?;
    
    Ok(user)
}

pub async fn create_token(
    pool: &PgPool,
    user_id: i64,
    token: &str,
    expires_at: DateTime<Utc>,
) -> Result<(), anyhow::Error> {
    sqlx::query!(
        r#"
        INSERT INTO tokenstore (user_id, token, expires_at, is_active)
        VALUES ($1, $2, $3, true)
        "#,
        user_id,
        token,
        expires_at
    )
    .execute(pool)
    .await
    .context("Failed to create token")?;

    Ok(())
}

pub async fn validate_token(pool: &PgPool, token: &str) -> Result<bool, anyhow::Error> {
    let user = sqlx::query_as!(
        User,
        r#"
        SELECT u.id, u.username, u.email, u.name, u.birthday, u.avatar_url,
               u.is_deleted, u.created_at, u.last_online_at
        FROM users u
        JOIN tokenstore t ON u.id = t.user_id
        WHERE t.token = $1 
          AND t.is_active = true 
          AND t.expires_at > NOW()
          AND u.is_deleted = false
        "#,
        token
    )
    .fetch_optional(pool)
    .await
    .context("Failed to validate token")?;

    Ok(user.is_some())
}

pub async fn deactivate_token(pool: &PgPool, token: &str) -> Result<(), anyhow::Error> {
    sqlx::query!(
        r#"
        UPDATE tokenstore
        SET is_active = false
        WHERE token = $1
        "#,
        token
    )
    .execute(pool)
    .await
    .context("Failed to deactivate token")?;

    Ok(())
}

pub async fn deactivate_all_user_tokens(pool: &PgPool, user_id: i64) -> Result<(), anyhow::Error> {
    sqlx::query!(
        r#"
        UPDATE tokenstore
        SET is_active = false
        WHERE user_id = $1 AND is_active = true
        "#,
        user_id
    )
    .execute(pool)
    .await
    .context("Failed to deactivate all user tokens")?;

    Ok(())
}

pub async fn find_token_by_user_id(pool: &PgPool, user_id: i64) -> Result<Option<String>, anyhow::Error> {
    let row = sqlx::query!(
        r#"
        SELECT token
        FROM tokenstore
        WHERE user_id = $1 AND is_active = true AND expires_at > NOW()
        ORDER BY created_at DESC
        LIMIT 1
        "#,
        user_id
    )
    .fetch_optional(pool)
    .await
    .context("Failed to find token by user id")?;

    Ok(row.map(|r| r.token))
}

pub async fn refresh_token(
    pool: &PgPool,
    old_token: &str,
    new_token: &str,
    expires_in_hours: DateTime<Utc>,
) -> Result<(), anyhow::Error> {
    deactivate_token(pool, old_token).await?;
    
    let row = sqlx::query!(
        r#"
        SELECT user_id FROM tokenstore
        WHERE token = $1
        "#, 
        old_token
    )
    .fetch_optional(pool)
    .await
    .context("Failed to get user_id from old token")?;

    let user_id: i64 = match row {
        Some(r) => r.user_id,
        None => return Err(anyhow::anyhow!("Old token not found")),
    };

    create_token(pool, user_id, new_token, expires_in_hours).await?;

    Ok(())
}

pub async fn cleanup_expired_tokens(pool: &PgPool) -> Result<u64, anyhow::Error> {
    let result = sqlx::query!(
        r#"
        DELETE FROM tokenstore
        WHERE expires_at <= NOW() OR is_active = false
        "#
    )
    .execute(pool)
    .await
    .context("Failed to cleanup expired tokens")?;

    Ok(result.rows_affected())
}

pub async fn update_user_avatar(
    pool: &PgPool,
    user_id: i64,
    avatar_url: &str,
) -> Result<(), anyhow::Error> {
    sqlx::query!(
        r#"
        UPDATE users
        SET avatar_url = $1
        WHERE id = $2
        "#,
        avatar_url,
        user_id
    )
    .execute(pool)
    .await
    .context("Failed to update avatar")?;
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::generator;

    use super::*;
    use sqlx::Executor;
    use chrono::Utc;

    #[tokio::test]
    async fn test_create_user_db() {
        let pool = setup_test_db().await;
        let user_id = create_user_db(
            &pool,
            "testuser",
            "test@mail.com",
            "Test",
            &None,
            &None,
        )
        .await
        .unwrap();
        assert!(user_id > 0);
    }

    #[tokio::test]
    async fn test_find_user_by_email() {
        let pool = setup_test_db().await;
        let email = "find@mail.com";
        create_user_db(
            &pool,
            "finduser",
            email,
            "Test",
            &None,
            &None,
        )
        .await
        .unwrap();
        let user = find_user_by_email(&pool, email).await.unwrap();
        assert!(user.is_some());
        assert_eq!(user.unwrap().email, email);
    }

    #[tokio::test]
    async fn test_edit_user_db() {
        let pool = setup_test_db().await;
        let user_id = create_user_db(
            &pool,
            "edituser",
            "edit@mail.com",
            "Test",
            &None,
            &None,
        )
        .await
        .unwrap();
        edit_user_db(
            &pool,
            user_id,
            Some("newusername"),
            None,
            Some("New Name"),
            None,
            None,
        )
        .await
        .unwrap();
        let user = find_user_by_id(&pool, user_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(user.username, "newusername");
        assert_eq!(user.name, "New Name");
    }

    #[tokio::test]
    async fn test_create_and_validate_token() {
        let pool = setup_test_db().await;
        let user_id = create_user_db(
            &pool,
            "tokenuser",
            "token@mail.com",
            "Test",
            &None,
            &None,
        )
        .await
        .unwrap();
        let token = &generator::Generator::new_session_token();
        create_token(
            &pool,
            user_id,
            token,
            Utc::now() + chrono::Duration::days(30),
        )
        .await
        .unwrap();
        let valid = validate_token(&pool, token).await.unwrap();
        assert!(valid);
    }

    #[tokio::test]
    async fn test_find_user_by_token() {
        let pool = setup_test_db().await;
        let user_id = create_user_db(
            &pool,
            "tokenfind",
            "tokenfind@mail.com",
            "Test",
            &None,
            &None,
        )
        .await
        .unwrap();
        let token = "find_token";
        create_token(
            &pool,
            user_id,
            token,
            Utc::now() + chrono::Duration::hours(1),
        )
        .await
        .unwrap();
        let user = find_user_by_token(&pool, token).await.unwrap();
        assert!(user.is_some());
        assert_eq!(user.unwrap().id, user_id);
    }

    #[tokio::test]
    async fn test_deactivate_token() {
        let pool = setup_test_db().await;

        let user_id = create_user_db(
            &pool,
            "deactuser",
            "deact@mail.com",
            "Test",
            &None,
            &None,
        )
        .await
        .unwrap();
        let token = "deact_token";
        create_token(
            &pool,
            user_id,
            token,
            Utc::now() + chrono::Duration::hours(1),
        )
        .await
        .unwrap();
        deactivate_token(&pool, token).await.unwrap();
        let user = find_user_by_token(&pool, token).await.unwrap();
        assert!(user.is_none());
    }

    #[tokio::test]
    async fn test_cleanup_expired_tokens() {
        let pool = setup_test_db().await;
        let user_id = create_user_db(
            &pool,
            "cleanupuser",
            "cleanup@mail.com",
            "Test",
            &None,
            &None,
        )
        .await
        .unwrap();
        create_token(
            &pool,
            user_id,
            "expired_token",
            Utc::now() - chrono::Duration::hours(1),
        )
        .await
        .unwrap();
        let deleted = cleanup_expired_tokens(&pool).await.unwrap();
        assert!(deleted > 0);
    }
    #[tokio::test]
    async fn test_find_user_by_id() {
        let pool = setup_test_db().await;
        let user_id = create_user_db(
            &pool,
            "iduser",
            "id@mail.com",
            "Test",
            &None,
            &None,
        )
        .await
        .unwrap();
        let user = find_user_by_id(&pool, user_id).await.unwrap();
        assert!(user.is_some());
        assert_eq!(user.unwrap().id, user_id);
    }

    #[tokio::test]
    async fn test_find_user_by_username() {
        let pool = setup_test_db().await;
        let username = "username_test";
        create_user_db(
            &pool,
            username,
            "username@mail.com",
            "Test",
            &None,
            &None,
        )
        .await
        .unwrap();
        let user = find_user_by_username(&pool, username).await.unwrap();
        assert!(user.is_some());
        assert_eq!(user.unwrap().username, username);
    }

    #[tokio::test]
    async fn test_deactivate_all_user_tokens() {
        let pool = setup_test_db().await;
        let user_id = create_user_db(
            &pool,
            "multi_token_user",
            "multi@mail.com",
            "Test",
            &None,
            &None,
        )
        .await
        .unwrap();
        create_token(
            &pool,
            user_id,
            "token1",
            Utc::now() + chrono::Duration::hours(1),
        )
        .await
        .unwrap();
        create_token(
            &pool,
            user_id,
            "token2",
            Utc::now() + chrono::Duration::hours(1),
        )
        .await
        .unwrap();
        deactivate_all_user_tokens(&pool, user_id)
            .await
            .unwrap();
        let user = find_user_by_token(&pool, "token1").await.unwrap();
        assert!(user.is_none());
        let user = find_user_by_token(&pool, "token2").await.unwrap();
        assert!(user.is_none());
    }

    #[tokio::test]
    async fn test_find_token_by_user_id() {
        let pool = setup_test_db().await;
        let user_id = create_user_db(
            &pool,
            "token_lookup_user",
            "lookup@mail.com",
            "Test",
            &None,
            &None,
        )
        .await
        .unwrap();
        let token = "lookup_token";
        create_token(
            &pool,
            user_id,
            token,
            Utc::now() + chrono::Duration::hours(1),
        )
        .await
        .unwrap();
        let found = find_token_by_user_id(&pool, user_id)
            .await
            .unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap(), token);
    }

    #[tokio::test]
    async fn test_refresh_token() {
        let pool = setup_test_db().await;
        let user_id = create_user_db(
            &pool,
            "refresh_user",
            "refresh@mail.com",
            "Test",
            &None,
            &None,
        )
        .await
        .unwrap();
        let old_token = "old_token";
        let new_token = "new_token";
        create_token(
            &pool,
            user_id,
            old_token,
            Utc::now() + chrono::Duration::hours(1),
        )
        .await
        .unwrap();
        refresh_token(
            &pool,
            old_token,
            new_token,
            Utc::now() + chrono::Duration::hours(2),
        )
        .await
        .unwrap();
        let old = find_user_by_token(&pool, old_token).await.unwrap();// старый невалиден
        assert!(old.is_none());
        let new = find_user_by_token(&pool, new_token).await.unwrap();// новый валиден
        assert!(new.is_some());
    }

    #[tokio::test]
    async fn test_validate_token_false() {
        let pool = setup_test_db().await;
        let valid = validate_token(&pool, "non_existing_token")
            .await
            .unwrap();
        assert!(!valid);
    }

    #[tokio::test]
    async fn test_create_token() -> anyhow::Result<()> {
        let pool = setup_test_db().await;
        
        let user_id = create_user_db(
            &pool,
            "tokenuser",
            "token@example.com",
            "Token User",
            &None,
            &None,
        )
        .await?;
        
        let token = &generator::Generator::new_session_token();
        let expires_at = Utc::now() + chrono::Duration::days(30);
        
        create_token(&pool, user_id, token, expires_at).await?;
        
        let row = sqlx::query!(
            r#"
            SELECT user_id, token, is_active, expires_at
            FROM tokenstore
            WHERE token = $1
            "#,
            token
        )
        .fetch_one(&pool)
        .await?;
        
        assert_eq!(row.user_id, user_id);
        assert_eq!(row.token, token.to_owned());
        assert_eq!(row.is_active, Some(true));
        assert!(row.expires_at > Utc::now());
        
        Ok(())
    }
}

#[tokio::test]// Проверяет обновление avatar_url в базе
async fn test_update_user_avatar() {
    let pool = setup_test_db().await;
    let user_id = create_user_db(
        &pool,
        "avatar_user",
        "avatar@mail.com",
        "User",
        &None,
        &None,
    ).await.unwrap();
    update_user_avatar(&pool, user_id, "/avatar/test.jpg")
        .await
        .unwrap();
    let user = find_user_by_id(&pool, user_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(user.avatar_url.unwrap(), "/avatar/test.jpg");
}
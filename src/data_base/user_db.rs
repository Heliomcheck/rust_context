use sqlx::postgres::{PgPoolOptions, PgPool};
use sqlx::Row;
use anyhow::{Context, Ok};

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
        expires_at  // ← DateTime<Utc>
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
    .context("Failed to validate token");

    Ok(true)
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
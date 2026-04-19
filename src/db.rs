use sqlx::postgres::{PgPoolOptions, PgPool};
use sqlx::{migrate, Row};
use anyhow::Context;

use crate::structs::User;


pub async fn create_pool(database_url: &str) -> Result<PgPool, anyhow::Error> {
    let pool = PgPoolOptions::new()
    .max_connections(10)
    .connect(database_url)
    .await
    .context("Failed to connect to database")?;

    migrate!().run(&pool).await?;
    Ok(pool)
}

pub async fn create_user(
    pool: &PgPool,
    username: &str,
    email: &str,
    name: &str,
    birthday: Option<&str>,
    avatar_url: Option<&str>,
) -> Result<i64, anyhow::Error> {
    let row = sqlx::query(
        r#"
        INSERT INTO users (username, email, name, birthday, avatar_url)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id
        "#
    )
    .bind(username)
    .bind(email)
    .bind(name)
    .bind(birthday)
    .bind(avatar_url)
    .fetch_one(pool)
    .await
    .context("Failed to create user")?;
    
    Ok(row.get(0))
}

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


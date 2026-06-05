use sqlx::postgres::{PgPoolOptions, PgPool};
use std::result::Result;

use chrono::{DateTime, Utc};

use crate::structs::User;


pub async fn create_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    let pool = PgPoolOptions::new()
    .max_connections(10)
    .connect(database_url)
    .await?;

    sqlx::migrate!().run(&pool).await?;
    Ok(pool)
}

pub async fn create_user_db(
    pool: &PgPool,
    username: &str,
    email: &str,
    display_name: &str,
    birthday: &Option<String>,
    description_profile: &Option<String>
) -> Result<i64, sqlx::Error> {
    let row = sqlx::query!(
        r#"
        INSERT INTO users (username, email, display_name, birthday, description_profile)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING user_id
        "#, 
        username, 
        email, 
        display_name, 
        birthday.clone(), 
        description_profile.clone()
    )
    .fetch_one(pool)
    .await?;
    
    Ok(row.user_id)
}

pub async fn edit_user_db(
    pool: &PgPool,
    user_id: i64,
    username: Option<&str>,
    display_name: Option<&str>,
    birthday: Option<&str>,
    description_profile: Option<&str>
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE users
        SET username = COALESCE($1, username),
            display_name = COALESCE($2, display_name),
            birthday = COALESCE($3, birthday),
            description_profile = COALESCE($4, description_profile)
        WHERE user_id = $5
        "#
    )
    .bind(username)
    .bind(display_name)
    .bind(birthday)
    .bind(description_profile)
    .bind(user_id)
    .execute(pool)
    .await?;
    
    Ok(())
} // add delete user later, check later

pub async fn find_user_by_email(pool: &PgPool, email: &str) -> Result<Option<User>, sqlx::Error> {
    let user = sqlx::query_as::<_, User>(
        r#"
        SELECT user_id, username, email, display_name, birthday, avatar_url,
               is_deleted, created_at, last_online_at, description_profile
        FROM users
        WHERE email = $1 AND is_deleted = false
        "#
    )
    .bind(email)
    .fetch_optional(pool)
    .await?;
    
    Ok(user)
}

pub async fn find_user_by_token(pool: &PgPool, token: &str) -> Result<Option<User>, sqlx::Error> {
    let user = sqlx::query_as::<_, User>(
        r#"
        SELECT u.user_id, u.username, u.email, u.display_name, u.birthday, u.avatar_url,
               u.is_deleted, u.created_at, u.last_online_at, description_profile
        FROM users u
        JOIN token_store t ON u.user_id = t.user_id
        WHERE t.token = $1 AND t.is_active = true AND t.expires_at > NOW()
        "#
    )
    .bind(token)
    .fetch_optional(pool)
    .await?;
    
    Ok(user)
}

pub async fn find_user_by_id(pool: &PgPool, user_id: i64) -> Result<Option<User>, sqlx::Error> {
    let user = sqlx::query_as!(
        User,
        r#"
        SELECT user_id, username, email, display_name, birthday,
               is_deleted, created_at, last_online_at, description_profile
        FROM users
        WHERE user_id = $1 AND is_deleted = false
        "#,
        user_id
    )
    .fetch_optional(pool) 
    .await?;
    
    Ok(user)
}

pub async fn find_user_by_username(
    pool: &PgPool,
    username: &str,
) -> Result<Option<User>, sqlx::Error> {
    let user = sqlx::query_as!(
        User,
        r#"
        SELECT user_id, username, email, display_name, birthday,
               is_deleted, created_at, last_online_at, description_profile
        FROM users
        WHERE username = $1 AND is_deleted = false
        "#,
        username
    )
    .fetch_optional(pool)
    .await?;
    
    Ok(user)
}

pub async fn create_token(
    pool: &PgPool,
    user_id: i64,
    token: &str,
    expires_at: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO token_store (user_id, token, expires_at, is_active)
        VALUES ($1, $2, $3, true)
        "#,
        user_id,
        token,
        expires_at
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn validate_token(pool: &PgPool, token: &str) -> Result<bool, sqlx::Error> {
    let user = sqlx::query_as::<_, User>(
        r#"
        SELECT u.user_id, u.username, u.email, u.display_name, u.birthday,
               u.is_deleted, u.created_at, u.last_online_at, description_profile
        FROM users u
        JOIN token_store t ON u.user_id = t.user_id
        WHERE t.token = $1 
          AND t.is_active = true 
          AND t.expires_at > NOW()
          AND u.is_deleted = false
        "#,
    )
    .bind(token)
    .fetch_optional(pool)
    .await?;

    Ok(user.is_some())
}

pub async fn deactivate_token(pool: &PgPool, token: &str) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        UPDATE token_store
        SET is_active = false
        WHERE token = $1
        "#,
        token
    )
    .execute(pool)
    .await?;

    Ok(())
}

#[allow(dead_code)]
pub async fn deactivate_all_user_tokens(pool: &PgPool, user_id: i64) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        UPDATE token_store
        SET is_active = false
        WHERE user_id = $1 AND is_active = true
        "#,
        user_id
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn find_token_by_user_id(pool: &PgPool, user_id: i64) -> Result<Option<String>, sqlx::Error> {
    let row = sqlx::query!(
        r#"
        SELECT token
        FROM token_store
        WHERE user_id = $1 AND is_active = true AND expires_at > NOW()
        ORDER BY created_at DESC
        LIMIT 1
        "#,
        user_id
    )
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| r.token))
}

#[allow(dead_code)]
pub async fn refresh_token(
    pool: &PgPool,
    old_token: &str,
    new_token: &str,
    expires_in_hours: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    deactivate_token(pool, old_token).await?;
    
    let row = sqlx::query!(
        r#"
        SELECT user_id FROM token_store
        WHERE token = $1
        "#, 
        old_token
    )
    .fetch_optional(pool)
    .await?;

    let user_id: i64 = match row {
        Some(r) => r.user_id,
        None => return Err(sqlx::Error::RowNotFound),
    };

    create_token(pool, user_id, new_token, expires_in_hours).await?;

    Ok(())
}

#[allow(dead_code)]
pub async fn cleanup_expired_tokens(pool: &PgPool) -> Result<u64, sqlx::Error> {
    let result = sqlx::query!(
        r#"
        DELETE FROM token_store
        WHERE expires_at <= NOW() OR is_active = false
        "#
    )
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}

pub async fn update_user_avatar(
    pool: &PgPool,
    user_id: i64,
    avatar_url: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        UPDATE users
        SET avatar_url = $1
        WHERE user_id = $2
        "#,
        avatar_url,
        user_id
    )
    .execute(pool)
    .await?;
    
    Ok(())
}

pub async fn update_last_online(pool: &PgPool, user_id: i64) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        UPDATE users
        SET last_online_at = NOW()
        WHERE user_id = $1
        "#,
        user_id
    )
    .execute(pool)
    .await?;
    
    Ok(())
}
//test
#[cfg(test)]
mod tests {
    use crate::secrets::generator;
    use super::*;
    use chrono::Utc;
    use crate::test_utils::*;

    #[tokio::test]
    async fn test_create_user_db() -> anyhow::Result<()> {
        let _guard = lock_db().await;
        let pool = setup_test_db().await;
        let user_id = create_user_db(
            &pool,
            "testuser",
            "test@mail.com",
            "Test",
            &None,
            &None,
        ).await?;
        assert!(user_id > 0);
        Ok(())
    }

    #[tokio::test]
    async fn test_find_user_by_email() -> anyhow::Result<()> {
        let _guard = lock_db().await;
        let pool = setup_test_db().await;
        let email = "find@mail.com";
        create_user_db(&pool, "finduser", email, "Test", &None, &None).await?;
        let user = find_user_by_email(&pool, email).await?;
        assert!(user.is_some());
        assert_eq!(user.unwrap().email, email);
        Ok(())
    }

    #[tokio::test]
    async fn test_edit_user_db() -> anyhow::Result<()> {
        let _guard = lock_db().await;
        let pool = setup_test_db().await;
        let user_id = create_user_db(
            &pool,
            "edituser",
            "edit@mail.com",
            "Test",
            &None,
            &None,
        ).await?;

        edit_user_db(
            &pool,
            user_id,
            Some("newusername"),
            Some("New Name"),
            Some("31-01-1999"),
            Some("Updated description"),
        ).await?;

        let user = find_user_by_id(&pool, user_id).await?.unwrap();
        assert_eq!(user.username, "newusername");
        assert_eq!(user.display_name, "New Name");
        assert_eq!(user.birthday, Some("31-01-1999".to_string()));
        assert_eq!(user.description_profile, Some("Updated description".to_string()));
        Ok(())
    }

    #[tokio::test]
    async fn test_create_and_validate_token() -> anyhow::Result<()> {
        let _guard = lock_db().await;
        let pool = setup_test_db().await;
        let user_id = create_user_db(
            &pool,
            "tokenuser",
            "token@mail.com",
            "Test",
            &None,
            &None,
        ).await?;
        let token = &generator::Generator::new_session_token();
        create_token(
            &pool,
            user_id,
            token,
            Utc::now() + chrono::Duration::days(30),
        ).await?;
        let valid = validate_token(&pool, token).await?;
        assert!(valid);
        Ok(())
    }

    #[tokio::test]
    async fn test_find_user_by_token() -> anyhow::Result<()> {
        let _guard = lock_db().await;
        let pool = setup_test_db().await;
        let user_id = create_user_db(
            &pool,
            "tokenfind",
            "tokenfind@mail.com",
            "Test",
            &None,
            &None,
        ).await?;
        let token = "find_token";
        create_token(&pool, user_id, token, Utc::now() + chrono::Duration::hours(1)).await?;
        let user = find_user_by_token(&pool, token).await?;
        assert!(user.is_some());
        assert_eq!(user.unwrap().user_id, user_id);
        Ok(())
    }

    #[tokio::test]
    async fn test_deactivate_token() -> anyhow::Result<()> {
        let _guard = lock_db().await;
        let pool = setup_test_db().await;
        let user_id = create_user_db(
            &pool,
            "deactuser",
            "deact@mail.com",
            "Test",
            &None,
            &None,
        ).await?;
        let token = "deact_token";
        create_token(&pool, user_id, token, Utc::now() + chrono::Duration::hours(1)).await?;
        deactivate_token(&pool, token).await?;
        let user = find_user_by_token(&pool, token).await?;
        assert!(user.is_none());
        Ok(())
    }

    #[tokio::test]
    async fn test_cleanup_expired_tokens() -> anyhow::Result<()> {
        let _guard = lock_db().await;
        let pool = setup_test_db().await;
        let user_id = create_user_db(
            &pool,
            "cleanupuser",
            "cleanup@mail.com",
            "Test",
            &None,
            &None,
        ).await?;
        create_token(
            &pool,
            user_id,
            "expired_token",
            Utc::now() - chrono::Duration::hours(1),
        ).await?;
        let deleted = cleanup_expired_tokens(&pool).await?;
        assert!(deleted > 0);
        Ok(())
    }

    #[tokio::test]
    async fn test_find_user_by_id() -> anyhow::Result<()> {
        let _guard = lock_db().await;
        let pool = setup_test_db().await;
        let user_id = create_user_db(
            &pool,
            "iduser",
            "id@mail.com",
            "Test",
            &None,
            &None,
        ).await?;
        let user = find_user_by_id(&pool, user_id).await?;
        assert!(user.is_some());
        assert_eq!(user.unwrap().user_id, user_id);
        Ok(())
    }

    #[tokio::test]
    async fn test_find_user_by_username() -> anyhow::Result<()> {
        let _guard = lock_db().await;
        let pool = setup_test_db().await;
        let username = "username_test";
        create_user_db(&pool, username, "username@mail.com", "Test", &None, &None).await?;
        let user = find_user_by_username(&pool, username).await?;
        assert!(user.is_some());
        assert_eq!(user.unwrap().username, username);
        Ok(())
    }

    #[tokio::test]
    async fn test_deactivate_all_user_tokens() -> anyhow::Result<()> {
        let _guard = lock_db().await;
        let pool = setup_test_db().await;
        let user_id = create_user_db(
            &pool,
            "multi_token_user",
            "multi@mail.com",
            "Test",
            &None,
            &None,
        ).await?;

        create_token(&pool, user_id, "token1", Utc::now() + chrono::Duration::hours(1)).await?;
        create_token(&pool, user_id, "token2", Utc::now() + chrono::Duration::hours(1)).await?;
        deactivate_all_user_tokens(&pool, user_id).await?;

        let user = find_user_by_token(&pool, "token1").await?;
        assert!(user.is_none());
        let user = find_user_by_token(&pool, "token2").await?;
        assert!(user.is_none());
        Ok(())
    }

    #[tokio::test]
    async fn test_find_token_by_user_id() -> anyhow::Result<()> {
        let _guard = lock_db().await;
        let pool = setup_test_db().await;
        let user_id = create_user_db(
            &pool,
            "token_lookup_user",
            "lookup@mail.com",
            "Test",
            &None,
            &None,
        ).await?;
        let token = "lookup_token";
        create_token(&pool, user_id, token, Utc::now() + chrono::Duration::hours(1)).await?;
        let found = find_token_by_user_id(&pool, user_id).await?;
        assert!(found.is_some());
        assert_eq!(found.unwrap(), token);
        Ok(())
    }

    #[tokio::test]
    async fn test_refresh_token() -> anyhow::Result<()> {
        let _guard = lock_db().await;
        let pool = setup_test_db().await;
        let user_id = create_user_db(
            &pool,
            "refresh_user",
            "refresh@mail.com",
            "Test",
            &None,
            &None,
        ).await?;
        let old_token = "old_token";
        let new_token = "new_token";
        create_token(&pool, user_id, old_token, Utc::now() + chrono::Duration::hours(1)).await?;
        refresh_token(&pool, old_token, new_token, Utc::now() + chrono::Duration::hours(2)).await?;
        let old = find_user_by_token(&pool, old_token).await?;
        assert!(old.is_none());
        let new = find_user_by_token(&pool, new_token).await?;
        assert!(new.is_some());
        Ok(())
    }

    #[tokio::test]
    async fn test_validate_token_false() -> anyhow::Result<()> {
        let _guard = lock_db().await;
        let pool = setup_test_db().await;
        let valid = validate_token(&pool, "non_existing_token").await?;
        assert!(!valid);
        Ok(())
    }

    #[tokio::test]
    async fn test_create_token() -> anyhow::Result<()> {
        let _guard = lock_db().await;
        let pool = setup_test_db().await;
        let user_id = create_user_db(
            &pool,
            "tokenuser",
            "token@example.com",
            "Token User",
            &None,
            &None,
        ).await?;

        let token = &generator::Generator::new_session_token();
        let expires_at = Utc::now() + chrono::Duration::days(30);
        create_token(&pool, user_id, token, expires_at).await?;

        let row = sqlx::query!(
            r#"SELECT user_id, token, is_active, expires_at FROM token_store WHERE token = $1"#,
            token
        ).fetch_one(&pool).await?;

        assert_eq!(row.user_id, user_id);
        assert_eq!(row.token, token.to_owned());
        assert_eq!(row.is_active, Some(true));
        assert!(row.expires_at > Utc::now());
        Ok(())
    }
}
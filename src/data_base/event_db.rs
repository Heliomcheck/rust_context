use axum::response::sse::Event;
use sqlx::PgPool;
use anyhow::{Context};
use chrono::{DateTime, Utc};
use tower::layer::util;
use crate::{
    errors::AppError,
    models::*,
    structs::Events,
};
use std::result::Result;
use std::string::String;
use std::str::*;

pub async fn create_event(
    pool: &PgPool,
    event_name: &str,
    description_event: Option<&str>,
    start_date: Option<DateTime<Utc>>,
    end_date: Option<DateTime<Utc>>,
    color: String
) -> Result<i64, AppError> {
    let row = sqlx::query!(
        r#"
        INSERT INTO events (event_name, description_event, start_date, end_date, color)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING event_id
        "#,
        event_name,
        description_event,
        start_date,
        end_date,
        color
    )
    .fetch_one(pool)
    .await?;

    Ok(row.event_id)
}

pub async fn get_event_by_id(
    pool: &PgPool,
    event_id: i64,
) -> Result<Events, AppError> {
    let row = sqlx::query!(
        r#"
        SELECT event_id, event_name, description_event, start_date, end_date, color, is_active, created_at, status_id
        FROM events
        WHERE event_id = $1
        "#,
        event_id
    )
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::EventNotFound)?;

    Ok( Events {
        event_name: row.event_name,
        event_id: event_id,
        description_event: row.description_event,
        start_date: row.start_date,
        end_date: row.end_date,
        color: row.color,
        is_active: row.is_active,
        created_at: row.created_at,
        status_id: row.status_id
    })
}

pub async fn get_user_events(
    pool: &PgPool,
    user_id: i64,
    limit: i64,
    offset: i64,
) -> Result<Vec<(i64, String, Option<String>, Option<DateTime<Utc>>, Option<DateTime<Utc>>, bool, DateTime<Utc>, i16)>, AppError> {
    let rows = sqlx::query!(
        r#"
        SELECT e.event_id, e.event_name, e.description_event, e.start_date, e.end_date, e.is_active, e.created_at, e.status_id
        FROM events e
        JOIN event_user eu ON e.event_id = eu.event_id
        WHERE eu.user_id = $1
        ORDER BY e.created_at DESC
        LIMIT $2 OFFSET $3
        "#,
        user_id,
        limit,
        offset
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|r| (r.event_id, r.event_name, r.description_event, r.start_date, 
        r.end_date, r.is_active, r.created_at, r.status_id)).collect())
}

pub async fn get_event_members(
    pool: &PgPool,
    event_id: i64,
) -> Result<Vec<(i64, String, i16, i16, DateTime<Utc>)>, AppError> {
    let rows = sqlx::query!(
        r#"
        SELECT u.user_id, u.username, eu.role_id, eu.status_id, eu.joined_at
        FROM event_user eu
        JOIN users u ON eu.user_id = u.user_id
        WHERE eu.event_id = $1
        ORDER BY eu.joined_at ASC
        "#,
        event_id
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|r| (r.user_id, r.username, r.role_id, r.status_id, 
        r.joined_at.unwrap())).collect())
}

pub async fn update_event(
    pool: &PgPool,
    event_id: i64,
    event_name: Option<&str>,
    description_event: Option<&str>,
    start_date: Option<DateTime<Utc>>,
    end_date: Option<DateTime<Utc>>,
    is_active: Option<bool>,
) -> Result<(), AppError> {
    sqlx::query!(
        r#"
        UPDATE events
        SET 
            event_name = COALESCE($1, event_name),
            description_event = COALESCE($2, description_event),
            start_date = COALESCE($3, start_date),
            end_date = COALESCE($4, end_date),
            is_active = COALESCE($5, is_active)
        WHERE event_id = $6
        "#,
        event_name,
        description_event,
        start_date,
        end_date,
        is_active,
        event_id
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn update_event_status(
    pool: &PgPool,
    event_id: i64,
    status_id: i16,
) -> Result<(), AppError> {
    sqlx::query!(
        r#"
        UPDATE events
        SET status_id = $1
        WHERE event_id = $2
        "#,
        status_id,
        event_id
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn get_event_owner_id(
    pool: &PgPool,
    event_id: i64,
) -> Result<i64, AppError> {
    let row = sqlx::query!(
        r#"
        SELECT user_id
        FROM event_user
        WHERE event_id = $1 AND role_id = 1
        "#,
        event_id
    )
    .fetch_one(pool)
    .await?;

    Ok(row.user_id)
}

// Members

pub async fn add_member(
    pool: &PgPool,
    user_id: i64,
    event_id: i64,
    role_id: i16,
    status_id: i16,
) -> Result<(), AppError> {
    sqlx::query!(
        r#"
        INSERT INTO event_user (user_id, event_id, role_id, status_id)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (user_id, event_id) DO UPDATE
        SET role_id = $3, status_id = $4
        "#,
        user_id,
        event_id,
        role_id,
        status_id
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn remove_member(
    pool: &PgPool,
    user_id: i64,
    event_id: i64,
) -> Result<(), AppError> {
    sqlx::query!(
        r#"
        DELETE FROM event_user
        WHERE user_id = $1 AND event_id = $2
        "#,
        user_id,
        event_id
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn update_member_role(
    pool: &PgPool,
    user_id: i64,
    event_id: i64,
    role_id: i16,
) -> Result<(), AppError> {
    sqlx::query!(
        r#"
        UPDATE event_user
        SET role_id = $1
        WHERE user_id = $2 AND event_id = $3
        "#,
        role_id,
        user_id,
        event_id
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn update_member_status(
    pool: &PgPool,
    user_id: i64,
    event_id: i64,
    status_id: i16,
) -> Result<(), AppError> {
    sqlx::query!(
        r#"
        UPDATE event_user
        SET status_id = $1
        WHERE user_id = $2 AND event_id = $3
        "#,
        status_id,
        user_id,
        event_id
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn create_event_token(
    pool: &PgPool,
    event_id: i64,
    expires_in_hours: i64,
) -> Result<String, AppError> {
    let token = uuid::Uuid::new_v4().to_string().replace("-", "");
    let expires_at = Utc::now() + chrono::Duration::hours(expires_in_hours);
    
    sqlx::query!(
        r#"
        INSERT INTO event_token (event_token, event_id, expires_at)
        VALUES ($1, $2, $3)
        "#,
        token,
        event_id,
        expires_at
    )
    .execute(pool)
    .await?;
    
    Ok(token)
}

pub async fn get_event_id_by_token(
    pool: &PgPool,
    token: &str,
) -> Result<Option<i64>, AppError> {
    let row = sqlx::query!(
        r#"
        SELECT event_id
        FROM event_token
        WHERE event_token = $1 AND expires_at > NOW()
        "#,
        token
    )
    .fetch_optional(pool)
    .await?;
    
    Ok(row.map(|r| r.event_id))
}

pub async fn is_user_in_event(
    pool: &PgPool,
    user_id: i64,
    event_id: i64,
) -> Result<bool, AppError> {
    let row = sqlx::query!(
        r#"
        SELECT EXISTS (
            SELECT 1 FROM event_user
            WHERE user_id = $1 AND event_id = $2
        ) as "exists!"
        "#,
        user_id,
        event_id
    )
    .fetch_one(pool)
    .await?;
    
    Ok(row.exists)
}
//test
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::setup_test_db;
    use crate::data_base::user_db::create_user_db;
    use chrono::Utc;

//EVENT
    #[tokio::test]
    async fn test_create_and_get_event() -> anyhow::Result<()> {
        let pool = setup_test_db().await;
        let event_id = create_event(
            &pool,
            "Test Event",
            Some("Description"),
            None,
            None,
            "#123456".to_string(),
        ).await?;
        let event = get_event_by_id(&pool, event_id).await?;
        assert_eq!(event.event_name, "Test Event");
        assert_eq!(event.color, "#123456");
        Ok(())
    }

    #[tokio::test]
    async fn test_get_event_not_found() -> anyhow::Result<()> {
        let pool = setup_test_db().await;
        let result = get_event_by_id(&pool, 9999).await;
        assert!(result.is_err());
        Ok(())
    }

    #[tokio::test]
    async fn test_update_event() -> anyhow::Result<()> {
        let pool = setup_test_db().await;
        let event_id = create_event(
            &pool, 
            "Old", 
            None, 
            None, 
            None,
            "#123456".to_string()
        ).await?;
        update_event(
            &pool,
            event_id,
            Some("New"),
            Some("Updated"),
            None,
            None,
            Some(false)
        ).await?;
        let event = get_event_by_id(&pool, event_id).await?;
        assert_eq!(event.event_name, "New");
        Ok(())
    }

    // #[tokio::test]
    // async fn test_update_event_status() -> anyhow::Result<()> {
    //     let pool = setup_test_db().await;
    //     let event_id = create_event(
    //         &pool, 
    //         "Test", 
    //         None, 
    //         None, 
    //         None,
    //         "#123456".to_string()
    //     ).await?;
    //     update_event_status(&pool, event_id, 2).await?;
    //     let event = get_event_by_id(&pool, event_id).await?;
    //     assert_eq!(event.7, 2);
    //     Ok(())
    // }

// Members

    #[tokio::test]
    async fn test_add_and_check_member() -> anyhow::Result<()> {
        let pool = setup_test_db().await;
        let user_id = create_user_db(
            &pool,
            "user1",
            "user1@mail.com",
            "User One",
            &None,
            &None,
            &None
        ).await?;
        let event_id = create_event(
            &pool, 
            "Event", 
            None, 
            None, 
            None,
            "#123456".to_string()
        ).await?;
        add_member(&pool, user_id, event_id, 1, 1).await?;
        let exists = is_user_in_event(&pool, user_id, event_id).await?;
        assert!(exists);
        Ok(())
    }

    #[tokio::test]
    async fn test_remove_member() -> anyhow::Result<()> {
        let pool = setup_test_db().await;
        let user_id = create_user_db(
            &pool,
            "user2",
            "user2@mail.com",
            "User Two",
            &None,
            &None,
            &None
        ).await?;
        let event_id = create_event(
            &pool, "Event", 
            None, 
            None, 
            None,
            "#123456".to_string()
        ).await?;
        add_member(&pool, user_id, event_id, 1, 1).await?;
        remove_member(&pool, user_id, event_id).await?;
        let exists = is_user_in_event(&pool, user_id, event_id).await?;
        assert!(!exists);
        Ok(())
    }

    #[tokio::test]
    async fn test_update_member_role() -> anyhow::Result<()> {
        let pool = setup_test_db().await;
        let user_id = create_user_db(
            &pool,
            "user3",
            "user3@mail.com",
            "User Three",
            &None,
            &None,
            &None
        ).await?;
        let event_id = create_event(
            &pool, "Event", 
            None, 
            None, 
            None,
            "#123456".to_string()
        ).await?;
        add_member(&pool, user_id, event_id, 1, 2).await?;
        update_member_role(&pool, user_id, event_id, 2).await?;
        let members = get_event_members(&pool, event_id).await?;
        assert_eq!(members[0].2, 2);
        Ok(())
    }

    #[tokio::test]
    async fn test_update_member_status() -> anyhow::Result<()> {
        let pool = setup_test_db().await;
        let user_id = create_user_db(
            &pool,
            "user4",
            "user4@mail.com",
            "User Four",
            &None,
            &None,
            &None
        ).await?;
        let event_id = create_event(
            &pool, "Event", 
            None, 
            None, 
            None,
            "#123456".to_string()
        ).await?;
        add_member(&pool, user_id, event_id, 1, 1).await?;
        update_member_status(&pool, user_id, event_id, 3).await?;
        let members = get_event_members(&pool, event_id).await?;
        assert_eq!(members[0].3, 3);
        Ok(())
    }

//USER EVENTS

    #[tokio::test]
    async fn test_get_user_events() -> anyhow::Result<()> {
        let pool = setup_test_db().await;
        let user_id = create_user_db(
            &pool,
            "user5",
            "user5@mail.com",
            "User Five",
            &None,
            &None,
            &None
        ).await?;
        let event_id = create_event(
            &pool, "Event", 
            None, 
            None, 
            None,
            "#123456".to_string()
        ).await?;
        add_member(&pool, user_id, event_id, 1, 1).await?;
        let events = get_user_events(&pool, user_id, 10, 0).await?;
        assert_eq!(events.len(), 1);
        Ok(())
    }

//TOKENS

    #[tokio::test]
    async fn test_event_token() -> anyhow::Result<()> {
        let pool = setup_test_db().await;
        let event_id = create_event(
            &pool, "Event", 
            None, 
            None, 
            None,
            "#123456".to_string()
        ).await?;
        let token = create_event_token(&pool, event_id, 1).await?;
        let found_event_id = get_event_id_by_token(&pool, &token).await?;
        assert_eq!(found_event_id.unwrap(), event_id);
        Ok(())
    }
}
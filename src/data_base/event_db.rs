use sqlx::PgPool;
use chrono::{DateTime, Utc};
#[allow(unused_imports)]
use crate::{
    errors::AppError,
    structs::*,
    models::*
};
use std::result::Result;
use std::string::String;
use chrono::{Duration};

pub async fn create_event(
    pool: &PgPool,
    title: &str,
    description_event: Option<String>,
    start_date_time: Option<DateTime<Utc>>,
    end_date_time: Option<DateTime<Utc>>,
    location: Option<String>,
    color: String
) -> Result<i64, AppError> {
    let row = sqlx::query!(
        r#"
        INSERT INTO events (
            title, description_event, start_date_time, end_date_time, color, location
        )
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING event_id
        "#,
        title,
        description_event,
        start_date_time,
        end_date_time,
        color,
        location
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
        SELECT event_id, title, description_event, start_date_time, end_date_time, color, created_at, status_event, location
        FROM events
        WHERE event_id = $1
        "#,
        event_id
    )
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::EventNotFound)?;

    Ok( Events {
        title: row.title,
        event_id: event_id,
        description_event: row.description_event,
        start_date_time: row.start_date_time,
        end_date_time: row.end_date_time,
        color: row.color,
        created_at: row.created_at,
        status_event: row.status_event,
        location: row.location
    })
}

pub async fn get_user_events(
    pool: &PgPool,
    user_id: i64,
    limit: i64,
    offset: i64,
    status: String,  // 👈 обязательный
) -> Result<Vec<Events>, AppError> {
    let rows = sqlx::query!(
        r#"
        SELECT e.event_id, e.title, e.description_event, e.start_date_time, e.end_date_time, 
            e.status_event, e.created_at, e.color, e.location
        FROM events e
        JOIN event_user eu ON e.event_id = eu.event_id
        WHERE eu.user_id = $1 AND e.status_event = $2
        ORDER BY 
            CASE e.status_event
                WHEN 'active' THEN 1
                WHEN 'archived' THEN 2
                ELSE 99
            END ASC,
            e.created_at DESC
        LIMIT $3 OFFSET $4
        "#,
        user_id,
        status,
        limit,
        offset
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|r| Events {
        event_id: r.event_id,
        title: r.title,
        description_event: r.description_event,
        start_date_time: r.start_date_time,
        end_date_time: r.end_date_time,
        created_at: r.created_at,
        status_event: r.status_event,
        color: r.color,
        location: r.location,
    }).collect())
}

#[allow(dead_code)]
pub async fn get_user_event(
    pool: &PgPool,
    user_id: i64,
    limit: i64,
    offset: i64,
    status: String,
) -> Result<Events, AppError> {
    let row = sqlx::query!(
        r#"
        SELECT e.event_id, e.title, e.description_event, e.start_date_time, e.end_date_time, e.status_event, e.created_at, e.color, e.location
        FROM events e
        JOIN event_user eu ON e.event_id = eu.event_id
        WHERE eu.user_id = $1 AND e.status_event = $4
        ORDER BY e.created_at DESC
        LIMIT $2 OFFSET $3
        "#,
        user_id,
        limit,
        offset,
        status
    )
    .fetch_one(pool)
    .await?;

    Ok(Events {
        event_id: row.event_id,
        title: row.title,
        description_event: row.description_event,
        start_date_time: row.start_date_time,
        end_date_time: row.end_date_time,
        created_at: row.created_at,
        status_event: row.status_event,
        color: row.color,
        location: row.location
    })
}

pub async fn get_users_in_event(
    pool: &PgPool,
    event_id: i64,
) -> Result<Vec<EventParticipant>, AppError> {
    let rows = sqlx::query!(
        r#"
        SELECT 
            u.user_id,
            COALESCE(u.display_name, u.username) AS name,
            eu.permissions
        FROM event_user eu
        JOIN users u ON eu.user_id = u.user_id
        WHERE eu.event_id = $1
        ORDER BY COALESCE(u.display_name, u.username) ASC
        "#,
        event_id
    )
    .fetch_all(pool)
    .await
    .map_err(AppError::DbError)?;

    Ok(rows
        .into_iter()
        .map(|row| EventParticipant {
            user_id: row.user_id,
            display_name: row.name.unwrap_or_else(|| "Unknown".to_string()),
            permissions: format!("{:b}", row.permissions),
        })
        .collect())
}

pub async fn check_user_in_event(
    pool: &PgPool,
    event_id: i64,
    user_id: i64
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query!(
        r#"
        SELECT eu.user_id
        FROM event_user eu
        WHERE event_id = $1 and user_id = $2
        "#,
        event_id,
        user_id
    )
    .fetch_optional(pool)
    .await?;

    if result.is_some() {
        return Ok(true)
    } else {
        return Ok(false)
    }
}
//test
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::setup_test_db;
    use crate::data_base::user_db::create_user_db;

    #[tokio::test]
    async fn test_create_and_get_event() -> anyhow::Result<()> {
        let pool = setup_test_db().await;
        let event_id = create_event(
            &pool,
            "Test Event",
            Some("Description".to_string()),
            None,
            None,
            Some("uiu".to_string()),
            "#123456".to_string(),
        ).await?;
        let event = get_event_by_id(&pool, event_id).await?;
        assert_eq!(event.title, "Test Event");
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
            Some("uiu".to_string()),
            "#123456".to_string()
        ).await?;
        update_event(
            &pool,
            event_id,
            Some("New".to_string()),
            Some("Updated".to_string()),
            None,
            None,
            Some("#654321".to_string()),
            Some("1".to_string())
        ).await?;
        let event = get_event_by_id(&pool, event_id).await?;
        assert_eq!(event.title, "New");
        Ok(())
    }

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
        ).await?;
        let event_id = create_event(
            &pool,
            "Event",
            None,
            None,
            None,
            Some("uiu".to_string()),
            "#123456".to_string()
        ).await?;
        add_member(&pool, user_id, event_id, 10).await?;
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
        ).await?;
        let event_id = create_event(
            &pool, "Event",
            None,
            None,
            None,
            Some("uiu".to_string()),
            "#123456".to_string()
        ).await?;
        add_member(&pool, user_id, event_id, 1).await?;
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
        ).await?;
        let event_id = create_event(
            &pool, "Event",
            None,
            None,
            None,
            Some("uiu".to_string()),
            "#123456".to_string()
        ).await?;
        add_member(&pool, user_id, event_id, 10).await?;
        update_member_role(&pool, user_id, event_id, 01).await?;
        let members = get_event_members(&pool, event_id).await?;
        assert_eq!(members[0].permissions, 01);
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
        ).await?;
        let event_id = create_event(
            &pool, "Event",
            None,
            None,
            None,
            Some("uiu".to_string()),
            "#123456".to_string()
        ).await?;
        add_member(&pool, user_id, event_id, 10).await?;
        let members = get_event_members(&pool, event_id).await?;
        assert_eq!(members[0].permissions, 10);
        Ok(())
    }

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
        ).await?;
        let event_id = create_event(
            &pool, "Event",
            None,
            None,
            None,
            Some("uiu".to_string()),
            "#123456".to_string()
        ).await?;
        add_member(&pool, user_id, event_id, 10).await?;
        let events = get_user_events(&pool, user_id, 10, 0, "active".to_string()).await?;
        assert_eq!(events.len(), 1);
        Ok(())
    }

    #[tokio::test]
    async fn test_event_token() -> anyhow::Result<()> {
        let pool = setup_test_db().await;
        let event_id = create_event(
            &pool, "Event",
            None,
            None,
            None,
            Some("uiu".to_string()),
            "#123456".to_string()
        ).await?;
        let event_token = create_event_token(&pool, event_id, 1).await?;
        let found_event_id = get_event_id_by_token(&pool, &event_token).await?;
        assert_eq!(found_event_id.unwrap(), event_id);
        Ok(())
    }

    #[tokio::test]
    async fn test_get_user_event() -> anyhow::Result<()> {
        let pool = setup_test_db().await;
        let user_id = create_user_db(
            &pool,
            "user_get_event",
            "getevent@mail.com",
            "User Event",
            &None,
            &None,
        ).await?;
        let event_id = create_event(
            &pool,
            "Event",
            None,
            None,
            None,
            Some("uiu".to_string()),
            "#123456".to_string()
        ).await?;
        add_member(&pool, user_id, event_id, 10).await?;

        let event = get_user_event(&pool, user_id, 1, 0, "active".to_string()).await?;
        assert_eq!(event.event_id, event_id);
        Ok(())
    }

    #[tokio::test]
    async fn test_get_users_in_event() -> anyhow::Result<()> {
        let pool = setup_test_db().await;
        let user_id = create_user_db(
            &pool,
            "user_list",
            "userlist@mail.com",
            "User List",
            &None,
            &None,
        ).await?;
        let event_id = create_event(
            &pool,
            "Event",
            None,
            None,
            None,
            Some("uiu".to_string()),
            "#123456".to_string()
        ).await?;
        add_member(&pool, user_id, event_id, 10).await?;

        let users = get_users_in_event(&pool, event_id).await?;
        assert_eq!(users.len(), 1);
        assert_eq!(users[0].user_id, user_id);
        Ok(())
    }

    #[tokio::test]
    async fn test_check_user_in_event() -> anyhow::Result<()> {
        let pool = setup_test_db().await;
        let user_id = create_user_db(
            &pool,
            "user_check",
            "usercheck@mail.com",
            "User Check",
            &None,
            &None,
        ).await?;
        let event_id = create_event(
            &pool,
            "Event",
            None,
            None,
            None,
            Some("uiu".to_string()),
            "#123456".to_string()
        ).await?;
        add_member(&pool, user_id, event_id, 10).await?;

        assert!(check_user_in_event(&pool, event_id, user_id).await?);
        assert!(!check_user_in_event(&pool, event_id, user_id + 1).await?);
        Ok(())
    }

    #[tokio::test]
    async fn test_get_event_members() -> anyhow::Result<()> {
        let pool = setup_test_db().await;
        let user_id = create_user_db(
            &pool,
            "member_user",
            "member@mail.com",
            "Member User",
            &None,
            &None,
        ).await?;
        let event_id = create_event(
            &pool,
            "Event",
            None,
            None,
            None,
            Some("uiu".to_string()),
            "#123456".to_string()
        ).await?;
        add_member(&pool, user_id, event_id, 10).await?;

        let members = get_event_members(&pool, event_id).await?;
        assert_eq!(members.len(), 1);
        assert_eq!(members[0].user_id, user_id);
        assert_eq!(members[0].username, "member_user");
        assert_eq!(members[0].permissions, 10);
        Ok(())
    }

    #[tokio::test]
    async fn test_find_users_by_permission_and_has_permission() -> anyhow::Result<()> {
        let pool = setup_test_db().await;
        let user_id = create_user_db(
            &pool,
            "perm_user",
            "perm@mail.com",
            "Perm User",
            &None,
            &None,
        ).await?;
        let event_id = create_event(
            &pool,
            "Event",
            None,
            None,
            None,
            Some("uiu".to_string()),
            "#123456".to_string()
        ).await?;

        add_member(&pool, user_id, event_id, 4).await?;

        let users = find_users_by_permission(&pool, event_id, 4).await?;
        assert_eq!(users, vec![user_id]);
        assert!(has_permission(&pool, event_id, user_id, 4).await?);
        Ok(())
    }

    #[tokio::test]
    async fn test_update_event_status() -> anyhow::Result<()> {
        let pool = setup_test_db().await;
        let event_id = create_event(
            &pool,
            "Event",
            None,
            None,
            None,
            Some("uiu".to_string()),
            "#123456".to_string()
        ).await?;
        update_event_status(&pool, event_id, "archived".to_string()).await?;
        let event = get_event_by_id(&pool, event_id).await?;
        assert_eq!(event.status_event, "archived");
        Ok(())
    }
}
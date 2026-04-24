use sqlx::PgPool;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};

pub async fn create_event(
    pool: &PgPool,
    event_name: &str,
    description: Option<&str>,
    start_date: Option<DateTime<Utc>>,
    end_date: Option<DateTime<Utc>>
) -> Result<i64> {
    let row = sqlx::query!(
        r#"
        INSERT INTO events (event_name, description, start_date, end_date)
        VALUES ($1, $2, $3, $4)
        RETURNING event_id
        "#,
        event_name,
        description,
        start_date,
        end_date
    )
    .fetch_one(pool)
    .await
    .context("Failed to create event")?;

    Ok(row.event_id)
}

pub async fn get_event_by_id(
    pool: &PgPool,
    event_id: i64,
) -> Result<Option<(i64, String, Option<String>, Option<DateTime<Utc>>, 
                    Option<DateTime<Utc>>, bool, DateTime<Utc>, i16)>> {
    let row = sqlx::query!(
        r#"
        SELECT event_id, event_name, description, start_date, end_date, is_active, created_at, status_id
        FROM events
        WHERE event_id = $1
        "#,
        event_id
    )
    .fetch_optional(pool)
    .await
    .context("Failed to get event")?;

    Ok(row.map(|r| (r.event_id, r.event_name, r.description, r.start_date, r.end_date, 
                    r.is_active.unwrap_or(true), r.created_at.unwrap(), r.status_id)))
}

pub async fn get_user_events(
    pool: &PgPool,
    user_id: i64,
    limit: i64,
    offset: i64,
) -> Result<Vec<(i64, String, Option<String>, Option<DateTime<Utc>>, Option<DateTime<Utc>>, bool, DateTime<Utc>, i16)>> {
    let rows = sqlx::query!(
        r#"
        SELECT e.event_id, e.event_name, e.description, e.start_date, e.end_date, e.is_active, e.created_at, e.status_id
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
    .await
    .context("Failed to get user events")?;

    Ok(rows.into_iter().map(|r| (r.event_id, r.event_name, r.description, r.start_date, 
        r.end_date, r.is_active.unwrap_or(true), r.created_at.unwrap(), r.status_id)).collect())
}

pub async fn get_event_participants(
    pool: &PgPool,
    event_id: i64,
) -> Result<Vec<(i64, String, i16, i16, DateTime<Utc>)>> {
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
    .await
    .context("Failed to get event participants")?;

    Ok(rows.into_iter().map(|r| (r.user_id, r.username, r.role_id, r.status_id, 
        r.joined_at.unwrap())).collect())
}

pub async fn update_event(
    pool: &PgPool,
    event_id: i64,
    event_name: Option<&str>,
    description: Option<&str>,
    start_date: Option<DateTime<Utc>>,
    end_date: Option<DateTime<Utc>>,
    is_active: Option<bool>,
) -> Result<()> {
    sqlx::query!(
        r#"
        UPDATE events
        SET 
            event_name = COALESCE($1, event_name),
            description = COALESCE($2, description),
            start_date = COALESCE($3, start_date),
            end_date = COALESCE($4, end_date),
            is_active = COALESCE($5, is_active)
        WHERE event_id = $6
        "#,
        event_name,
        description,
        start_date,
        end_date,
        is_active,
        event_id
    )
    .execute(pool)
    .await
    .context("Failed to update event")?;

    Ok(())
}

pub async fn update_event_status(
    pool: &PgPool,
    event_id: i64,
    status_id: i16,
) -> Result<()> {
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
    .await
    .context("Failed to update event status")?;

    Ok(())
}

// ============== УЧАСТНИКИ ==============

pub async fn add_participant(
    pool: &PgPool,
    user_id: i64,
    event_id: i64,
    role_id: i16,
    status_id: i16,
) -> Result<()> {
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
    .await
    .context("Failed to add participant")?;

    Ok(())
}

pub async fn remove_participant(
    pool: &PgPool,
    user_id: i64,
    event_id: i64,
) -> Result<()> {
    sqlx::query!(
        r#"
        DELETE FROM event_user
        WHERE user_id = $1 AND event_id = $2
        "#,
        user_id,
        event_id
    )
    .execute(pool)
    .await
    .context("Failed to remove participant")?;

    Ok(())
}

pub async fn update_participant_role(
    pool: &PgPool,
    user_id: i64,
    event_id: i64,
    role_id: i16,
) -> Result<()> {
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
    .await
    .context("Failed to update participant role")?;

    Ok(())
}

pub async fn update_participant_status(
    pool: &PgPool,
    user_id: i64,
    event_id: i64,
    status_id: i16,
) -> Result<()> {
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
    .await
    .context("Failed to update participant status")?;

    Ok(())
}

pub async fn create_event_token(
    pool: &PgPool,
    event_id: i64,
    expires_in_hours: i64,
) -> Result<String> {
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
    .await
    .context("Failed to create event token")?;
    
    Ok(token)
}

pub async fn get_event_id_by_token(
    pool: &PgPool,
    token: &str,
) -> Result<Option<i64>> {
    let row = sqlx::query!(
        r#"
        SELECT event_id
        FROM event_token
        WHERE event_token = $1 AND expires_at > NOW()
        "#,
        token
    )
    .fetch_optional(pool)
    .await
    .context("Failed to get event by token")?;
    
    Ok(row.map(|r| r.event_id))
}

pub async fn is_user_in_event(
    pool: &PgPool,
    user_id: i64,
    event_id: i64,
) -> Result<bool> {
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
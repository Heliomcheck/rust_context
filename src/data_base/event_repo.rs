use sqlx::PgPool;
use chrono::{DateTime, Utc};
use crate::models::{NewEvent, UpdateEventRequest};
use crate::errors::AppError;

// Структура для представления события из БД
#[derive(Debug, Clone)]
pub struct EventRow {
    pub id: i64,
    pub title: String,
    pub description: Option<String>,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub color: Option<String>,
    pub created_by: i64, // user_id владельца (role_id = 1)
    pub created_at: DateTime<Utc>,
    pub status_id: i16, // 1=draft, 2=open, 3=in_progress, 4=completed, 5=cancelled, 6=archived
    pub avatar_uploaded: bool,
}

// Создание события с владельцем
pub async fn create_event(pool: &PgPool, event: NewEvent) -> Result<EventRow, AppError> {
    let mut tx = pool.begin().await
        .map_err(|e| AppError::Internal(format!("Transaction error: {}", e)))?;

    // Создаём событие
    let event_row = sqlx::query!(
        r#"
        INSERT INTO events (event_name, description_profile, start_date, end_date, color, status_id, created_at)
        VALUES ($1, $2, $3, $4, $5, 2, NOW())
        RETURNING event_id, event_name, description_profile, start_date, end_date, color, created_at, is_active, avatar_uploaded
        "#,
        event.title,
        event.description,
        event.start_date_time,
        event.end_date_time,
        event.color
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to create event: {}", e)))?;

    // Добавляем владельца с ролью "Dungeon Master" (role_id = 1)
    sqlx::query!(
        r#"
        INSERT INTO event_user (user_id, event_id, role_id, status_id)
        VALUES ($1, $2, 1, 2)
        "#,
        event.created_by,
        event_row.event_id
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to add event owner: {}", e)))?;

    tx.commit().await
        .map_err(|e| AppError::Internal(format!("Commit error: {}", e)))?;

    Ok(EventRow {
        id: event_row.event_id,
        title: event_row.event_name,
        description: event_row.description_profile,
        start_date: event_row.start_date,
        end_date: event_row.end_date,
        color: event_row.color,
        created_by: event.created_by,
        created_at: event_row.created_at,
        status_id: 2, // open
        avatar_uploaded: event_row.avatar_uploaded.unwrap_or(false),
    })
}

// Получение события по ID с информацией о владельце
pub async fn get_event_by_id(pool: &PgPool, id: i64) -> Result<Option<EventRow>, AppError> {
    let row = sqlx::query!(
        r#"
        SELECT e.event_id, e.event_name, e.description_profile, e.start_date, e.end_date, 
               e.color, e.created_at, e.status_id, e.avatar_uploaded,
               eu.user_id
        FROM events e
        LEFT JOIN event_user eu ON e.event_id = eu.event_id AND eu.role_id = 1
        WHERE e.event_id = $1
        "#,
        id
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::Internal(format!("Database error: {}", e)))?;

    Ok(row.map(|r| EventRow {
        id: r.event_id,
        title: r.event_name,
        description: r.description_profile,
        start_date: r.start_date,
        end_date: r.end_date,
        color: r.color,
        created_by: r.user_id.unwrap_or(0),
        created_at: r.created_at,
        status_id: r.status_id as i16,
        avatar_uploaded: r.avatar_uploaded.unwrap_or(false),
    }))
}

// Получение событий пользователя (владельца) с фильтром
// status: "active" (is_active=true) или "archived" (status_id=6)
pub async fn get_events_by_user(
    pool: &PgPool,
    user_id: i64,
    status: &str,
    limit: i64,
    offset: i64,
) -> Result<(Vec<EventRow>, i64), AppError> {
    // Определяем условие фильтрации
    let is_archived = status == "archived";

    // Получаем общее количество событий
    let total_row = sqlx::query!(
        r#"
        SELECT COUNT(*) as count
        FROM events e
        INNER JOIN event_user eu ON e.event_id = eu.event_id AND eu.role_id = 1
        WHERE eu.user_id = $1 
          AND (($2 = true AND e.status_id = 6) OR ($2 = false AND e.status_id != 6))
        "#,
        user_id,
        is_archived
    )
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to count events: {}", e)))?;

    let total = total_row.count.unwrap_or(0);

    // Получаем постраничные результаты
    let rows = sqlx::query!(
        r#"
        SELECT e.event_id, e.event_name, e.description_profile, e.start_date, e.end_date, 
               e.color, e.created_at, e.status_id, e.avatar_uploaded,
               eu.user_id
        FROM events e
        INNER JOIN event_user eu ON e.event_id = eu.event_id AND eu.role_id = 1
        WHERE eu.user_id = $1 
          AND (($2 = true AND e.status_id = 6) OR ($2 = false AND e.status_id != 6))
        ORDER BY e.created_at DESC
        LIMIT $3 OFFSET $4
        "#,
        user_id,
        is_archived,
        limit,
        offset
    )
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to fetch events: {}", e)))?;

    let events = rows
        .into_iter()
        .map(|r| EventRow {
            id: r.event_id,
            title: r.event_name,
            description: r.description_profile,
            start_date: r.start_date,
            end_date: r.end_date,
            color: r.color,
            created_by: r.user_id.unwrap_or(0),
            created_at: r.created_at,
            status_id: r.status_id as i16,
            avatar_uploaded: r.avatar_uploaded.unwrap_or(false),
        })
        .collect();

    Ok((events, total))
}

// Обновление события
pub async fn update_event(
    pool: &PgPool,
    id: i64,
    payload: UpdateEventRequest,
    start: Option<DateTime<Utc>>,
    end: Option<DateTime<Utc>>,
) -> Result<EventRow, AppError> {
    let row = sqlx::query!(
        r#"
        UPDATE events
        SET event_name = COALESCE($1, event_name),
            description_profile = COALESCE($2, description_profile),
            start_date = COALESCE($3, start_date),
            end_date = COALESCE($4, end_date),
            color = COALESCE($5, color)
        WHERE event_id = $6
        RETURNING event_id, event_name, description_profile, start_date, end_date, color, created_at, status_id, avatar_uploaded
        "#,
        payload.title,
        payload.description,
        start,
        end,
        payload.color,
        id
    )
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to update event: {}", e)))?;

    // Получаем владельца
    let owner = sqlx::query!(
        "SELECT user_id FROM event_user WHERE event_id = $1 AND role_id = 1",
        id
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to fetch owner: {}", e)))?;

    Ok(EventRow {
        id: row.event_id,
        title: row.event_name,
        description: row.description_profile,
        start_date: row.start_date,
        end_date: row.end_date,
        color: row.color,
        created_by: owner.and_then(|o| o.user_id).unwrap_or(0),
        created_at: row.created_at,
        status_id: row.status_id as i16,
        avatar_uploaded: row.avatar_uploaded.unwrap_or(false),
    })
}

// Изменение статуса события (archive: false -> open, archive: true -> archived)
pub async fn update_event_status(pool: &PgPool, id: i64, status: &str) -> Result<(), AppError> {
    let status_id = match status {
        "active" => 2,   // open
        "archived" => 6, // archived
        _ => return Err(AppError::BadRequest("Invalid status".into())),
    };

    sqlx::query!(
        r#"
        UPDATE events
        SET status_id = $1
        WHERE event_id = $2
        "#,
        status_id as i16,
        id
    )
    .execute(pool)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to update status: {}", e)))?;

    Ok(())
}

// Отметить, что аватар загружен
pub async fn set_avatar_uploaded(pool: &PgPool, id: i64, uploaded: bool) -> Result<(), AppError> {
    sqlx::query!(
        r#"
        UPDATE events
        SET avatar_uploaded = $1
        WHERE event_id = $2
        "#,
        uploaded,
        id
    )
    .execute(pool)
    .await
    .map_err(|e| AppError::Internal(format!("Failed to set avatar flag: {}", e)))?;

    Ok(())
}

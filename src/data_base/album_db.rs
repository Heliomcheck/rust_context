use crate::models::{PhotoResponse, PhotoSyncInfo};
use chrono::{DateTime, Utc};

// Получить все фото события (для первого синка)
pub async fn get_all_photos(
    pool: &PgPool,
    event_id: i64,
) -> Result<Vec<PhotoResponse>, AppError> {
    let rows = sqlx::query!(
        r#"
        SELECT photo_id, etag, file_name, mime_type, file_size, created_at
        FROM event_photos
        WHERE event_id = $1 AND is_active = true
        ORDER BY created_at DESC
        "#,
        event_id
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| PhotoResponse {
            photo_id: r.photo_id,
            etag: r.etag.unwrap_or_else(|| format!("\"{}\"", r.photo_id)),
            url: format!("/events/{}/photos/{}", event_id, r.photo_id),
            mime_type: r.mime_type.unwrap_or_else(|| "image/jpeg".to_string()),
            file_size: r.file_size.unwrap_or(0),
            created_at: r.created_at.unwrap_or_else(Utc::now),
        })
        .collect())
}

// Получить изменения по сравнению с клиентскими ETag
pub async fn get_photo_changes(
    pool: &PgPool,
    event_id: i64,
    client_photos: &[ClientPhotoInfo],
) -> Result<AlbumSyncResponse, AppError> {
    // Получаем все актуальные фото с сервера
    let server_rows = sqlx::query!(
        r#"
        SELECT photo_id, etag, file_name, mime_type, file_size, created_at
        FROM event_photos
        WHERE event_id = $1 AND is_active = true
        "#,
        event_id
    )
    .fetch_all(pool)
    .await?;

    let server_map: std::collections::HashMap<i64, (String, String, String, i64, DateTime<Utc>)> = server_rows
        .into_iter()
        .map(|r| {
            (
                r.photo_id,
                (
                    r.etag.unwrap_or_else(|| format!("\"{}\"", r.photo_id)),
                    r.file_name,
                    r.mime_type.unwrap_or_else(|| "image/jpeg".to_string()),
                    r.file_size.unwrap_or(0),
                    r.created_at.unwrap_or_else(Utc::now),
                ),
            )
        })
        .collect();

    let client_map: std::collections::HashMap<i64, &str> = client_photos
        .iter()
        .map(|p| (p.photo_id, p.etag.as_str()))
        .collect();

    let mut added = Vec::new();
    let mut changed = Vec::new();
    let mut removed = Vec::new();

    // 1. Находим добавленные и изменённые
    for (id, (server_etag, file_name, mime_type, file_size, created_at)) in server_map {
        match client_map.get(&id) {
            Some(client_etag) if *client_etag != server_etag => {
                // ETag не совпадает → изменилось
                changed.push(PhotoSyncInfo {
                    photo_id: id,
                    etag: server_etag,
                    url: format!("/events/{}/photos/{}", event_id, id),
                    mime_type,
                    file_size,
                    created_at,
                });
            }
            None => {
                // Фото есть на сервере, нет у клиента → добавилось
                added.push(PhotoSyncInfo {
                    photo_id: id,
                    etag: server_etag,
                    url: format!("/events/{}/photos/{}", event_id, id),
                    mime_type,
                    file_size,
                    created_at,
                });
            }
            _ => {} // ETag совпадает → ничего не делаем
        }
    }

    // 2. Находим удалённые
    for client_id in client_map.keys() {
        if !server_map.contains_key(client_id) {
            removed.push(*client_id);
        }
    }

    Ok(AlbumSyncResponse { added, changed, removed })
}

// Добавить фото (при загрузке)
pub async fn add_photo(
    pool: &PgPool,
    event_id: i64,
    file_name: &str,
    original_name: &str,
    mime_type: &str,
    file_size: i64,
    uploaded_by: i64,
) -> Result<PhotoResponse, AppError> {
    let row = sqlx::query!(
        r#"
        INSERT INTO event_photos (event_id, file_name, original_name, mime_type, file_size, uploaded_by)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING photo_id, etag, created_at
        "#,
        event_id,
        file_name,
        original_name,
        mime_type,
        file_size,
        uploaded_by
    )
    .fetch_one(pool)
    .await?;

    Ok(PhotoResponse {
        photo_id: row.photo_id,
        etag: row.etag.unwrap_or_else(|| format!("\"{}\"", row.photo_id)),
        url: format!("/events/{}/photos/{}", event_id, row.photo_id),
        mime_type: mime_type.to_string(),
        file_size,
        created_at: row.created_at.unwrap_or_else(Utc::now),
    })
}
use sqlx::PgPool;
use chrono::Utc;
use crate::errors::AppError;
use crate::models::{AlbumResponse, AlbumWithPhotosResponse, PhotoResponse};

/// Создать альбом
pub async fn create_album(
    pool: &PgPool,
    event_id: i64,
    title: &str,
    description: Option<&str>,
    created_by: i64,
) -> Result<AlbumResponse, AppError> {
    let row = sqlx::query!(
        r#"INSERT INTO albums (event_id, title, description, created_by)
           VALUES ($1, $2, $3, $4)
           RETURNING album_id, created_at, updated_at"#,
        event_id,
        title,
        description,
        created_by,
    )
    .fetch_one(pool)
    .await
    .map_err(AppError::DbError)?;

    Ok(AlbumResponse {
        album_id: row.album_id,
        event_id,
        title: title.to_owned(),
        description: description.map(String::from),
        created_by,
        created_at: row.created_at.unwrap_or_else(Utc::now),
        updated_at: row.updated_at.unwrap_or_else(Utc::now),
    })
}

/// Получить список активных альбомов события
pub async fn get_event_albums(
    pool: &PgPool,
    event_id: i64,
) -> Result<Vec<AlbumResponse>, AppError> {
    let rows = sqlx::query!(
        r#"SELECT album_id, title, description, created_by, created_at, updated_at
           FROM albums
           WHERE event_id = $1 AND is_active = true
           ORDER BY created_at DESC"#,
        event_id,
    )
    .fetch_all(pool)
    .await
    .map_err(AppError::DbError)?;

    let albums = rows
        .into_iter()
        .map(|r| AlbumResponse {
            album_id: r.album_id,
            event_id,
            title: r.title,
            description: r.description,
            created_by: r.created_by,
            created_at: r.created_at.unwrap_or_else(Utc::now),
            updated_at: r.updated_at.unwrap_or_else(Utc::now),
        })
        .collect();
    Ok(albums)
}

/// Получить конкретный альбом вместе с фотографиями
pub async fn get_album_with_photos(
    pool: &PgPool,
    album_id: i64,
) -> Result<AlbumWithPhotosResponse, AppError> {
    let album = sqlx::query!(
        r#"SELECT event_id, title, description, created_by, created_at
           FROM albums
           WHERE album_id = $1 AND is_active = true"#,
        album_id,
    )
    .fetch_optional(pool)
    .await
    .map_err(AppError::DbError)?
    .ok_or(AppError::NotFound("Album not found".into()))?;

    let photos = get_photos_by_album(pool, album_id).await?;

    Ok(AlbumWithPhotosResponse {
        album_id,
        event_id: album.event_id,
        title: album.title,
        description: album.description,
        created_by: album.created_by,
        created_at: album.created_at.unwrap_or_else(Utc::now),
        photos,
    })
}

/// Вспомогательная: получить список активных фото альбома
pub async fn get_photos_by_album(
    pool: &PgPool,
    album_id: i64,
) -> Result<Vec<PhotoResponse>, AppError> {
    let rows = sqlx::query!(
        r#"SELECT photo_id, file_name, original_name, mime_type, file_size, uploaded_by, created_at
           FROM album_photos
           WHERE album_id = $1 AND is_active = true
           ORDER BY created_at ASC"#,
        album_id,
    )
    .fetch_all(pool)
    .await
    .map_err(AppError::DbError)?;

    Ok(rows
        .into_iter()
        .map(|r| PhotoResponse {
            photo_id: r.photo_id,
            file_name: r.file_name,
            original_name: r.original_name,
            mime_type: r.mime_type,
            file_size: r.file_size,
            uploaded_by: r.uploaded_by,
            created_at: r.created_at.unwrap_or_else(Utc::now),
            url: String::new(), // будет заполнено в обработчике
        })
        .collect())
}

/// Вставить запись о фото
pub async fn insert_photo(
    pool: &PgPool,
    album_id: i64,
    file_name: &str,
    original_name: &str,
    mime_type: &str,
    file_size: i64,
    uploaded_by: i64,
) -> Result<PhotoResponse, AppError> {
    let row = sqlx::query!(
        r#"INSERT INTO album_photos (album_id, file_name, original_name, mime_type, file_size, uploaded_by)
           VALUES ($1, $2, $3, $4, $5, $6)
           RETURNING photo_id, created_at"#,
        album_id,
        file_name,
        original_name,
        mime_type,
        file_size,
        uploaded_by,
    )
    .fetch_one(pool)
    .await
    .map_err(AppError::DbError)?;

    Ok(PhotoResponse {
        photo_id: row.photo_id,
        file_name: file_name.to_owned(),
        original_name: Some(original_name.to_owned()),
        mime_type: Some(mime_type.to_owned()),
        file_size: Some(file_size),
        uploaded_by,
        created_at: row.created_at.unwrap_or_else(Utc::now),
        url: String::new(),
    })
}

/// Полное удаление альбома: удаляет записи в БД и папку с фотографиями
pub async fn delete_album(
    pool: &PgPool,
    album_id: i64,
    event_id: i64,
) -> Result<(), AppError> {
    let result = sqlx::query!(
        "DELETE FROM albums WHERE album_id = $1 AND event_id = $2 AND is_active = true",
        album_id,
        event_id,
    )
    .execute(pool)
    .await
    .map_err(AppError::DbError)?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Album not found".into()));
    }

    let album_dir = crate::handlers::album::album_dir(event_id, album_id);
    if album_dir.exists() {
        tokio::fs::remove_dir_all(&album_dir).await.map_err(|e| {
            tracing::error!("Failed to delete album directory: {}", e);
            AppError::Internal("Failed to delete album files".into())
        })?;
    }

    Ok(())
}

/// Полное удаление фото: удаляет запись и файл
pub async fn delete_photo(
    pool: &PgPool,
    photo_id: i64,
    album_id: i64,
    event_id: i64,
) -> Result<(), AppError> {
    let row = sqlx::query!(
        "SELECT file_name FROM album_photos WHERE photo_id = $1 AND is_active = true",
        photo_id,
    )
    .fetch_optional(pool)
    .await
    .map_err(AppError::DbError)?
    .ok_or(AppError::NotFound("Photo not found".into()))?;

    sqlx::query!(
        "DELETE FROM album_photos WHERE photo_id = $1",
        photo_id,
    )
    .execute(pool)
    .await
    .map_err(AppError::DbError)?;

    let file_path = crate::handlers::album::album_dir(event_id, album_id).join(&row.file_name);
    if file_path.exists() {
        tokio::fs::remove_file(&file_path).await.map_err(|e| {
            tracing::error!("Failed to delete photo file: {}", e);
            AppError::Internal("Failed to delete photo file".into())
        })?;
    }

    Ok(())
}

/// Проверить, принадлежит ли альбом событию (активный)
pub async fn verify_album_in_event(
    pool: &PgPool,
    album_id: i64,
    event_id: i64,
) -> Result<bool, AppError> {
    let row = sqlx::query!(
        r#"SELECT EXISTS(
            SELECT 1 FROM albums
            WHERE album_id = $1 AND event_id = $2 AND is_active = true
        ) as "exists!"
        "#,
        album_id,
        event_id,
    )
    .fetch_one(pool)
    .await
    .map_err(AppError::DbError)?;

    Ok(row.exists)
}

/// Получить информацию об одном фото по id (вспомогательная)
#[allow(dead_code)]
pub async fn get_photo_by_id(
    pool: &PgPool,
    photo_id: i64,
) -> Result<Option<(String, String, i64)>, AppError> {
    let row = sqlx::query!(
        r#"
        SELECT file_name, mime_type, album_id
        FROM album_photos
        WHERE photo_id = $1 AND is_active = true
        "#,
        photo_id,
    )
    .fetch_optional(pool)
    .await
    .map_err(AppError::DbError)?;

    Ok(row.map(|r| (
        r.file_name,
        r.mime_type.unwrap_or_else(|| "application/octet-stream".into()),
        r.album_id,
    )))
}

// ====================== Тесты ======================

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use crate::data_base::event_db;
    use crate::data_base::user_db;
    use crate::permissions::EventPermissions;
    use crate::test_utils::setup_test_db;

    async fn setup() -> Result<(PgPool, i64, i64)> {
        let pool = setup_test_db().await;
        let user_id = user_db::create_user_db(
            &pool,
            "albumuser",
            "album@example.com",
            "Album User",
            &None,
            &None,
        )
        .await?;
        let event_id = event_db::create_event(
            &pool,
            "Album Event",
            Some("desc".into()),
            None,
            None,
            Some("loc".into()),
            "#ff0000".into(),
        )
        .await?;
        event_db::add_member(&pool, user_id, event_id, EventPermissions::OWNER).await?;
        Ok((pool, user_id, event_id))
    }

    #[tokio::test]
    async fn test_create_album() -> Result<()> {
        let (pool, user_id, event_id) = setup().await?;
        let album = create_album(&pool, event_id, "Vacation", Some("Summer pics"), user_id).await?;
        assert_eq!(album.title, "Vacation");
        assert_eq!(album.event_id, event_id);
        assert_eq!(album.created_by, user_id);
        Ok(())
    }

    #[tokio::test]
    async fn test_get_event_albums() -> Result<()> {
        let (pool, user_id, event_id) = setup().await?;
        create_album(&pool, event_id, "One", None, user_id).await?;
        create_album(&pool, event_id, "Two", None, user_id).await?;
        let albums = get_event_albums(&pool, event_id).await?;
        assert_eq!(albums.len(), 2);
        // сортировка по created_at DESC
        assert_eq!(albums[0].title, "Two");
        Ok(())
    }

    #[tokio::test]
    async fn test_verify_album_in_event() -> Result<()> {
        let (pool, user_id, event_id) = setup().await?;
        let album = create_album(&pool, event_id, "Test", None, user_id).await?;
        assert!(verify_album_in_event(&pool, album.album_id, event_id).await?);
        assert!(!verify_album_in_event(&pool, album.album_id, event_id + 1).await?);
        Ok(())
    }

    #[tokio::test]
    async fn test_insert_and_get_photo() -> Result<()> {
        let (pool, user_id, event_id) = setup().await?;
        let album = create_album(&pool, event_id, "Pics", None, user_id).await?;
        let photo = insert_photo(
            &pool,
            album.album_id,
            "temp",
            "original.jpg",
            "image/jpeg",
            12345,
            user_id,
        )
        .await?;
        assert_eq!(photo.file_name, "temp");
        assert!(photo.photo_id > 0);
        Ok(())
    }

    #[tokio::test]
    async fn test_delete_album() -> Result<()> {
        let (pool, user_id, event_id) = setup().await?;
        let album = create_album(&pool, event_id, "Del", None, user_id).await?;
        // Полное физическое удаление альбома
        delete_album(&pool, album.album_id, event_id).await?;
        let albums = get_event_albums(&pool, event_id).await?;
        assert!(albums.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn test_delete_photo() -> Result<()> {
        let (pool, user_id, event_id) = setup().await?;
        let album = create_album(&pool, event_id, "Pics", None, user_id).await?;
        let photo = insert_photo(&pool, album.album_id, "temp", "o.jpg", "image/jpeg", 100, user_id).await?;
        // Полное физическое удаление фото
        delete_photo(&pool, photo.photo_id, album.album_id, event_id).await?;
        let photos = get_photos_by_album(&pool, album.album_id).await?;
        assert!(photos.is_empty());
        Ok(())
    }
}
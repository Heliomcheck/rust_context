use axum::{
    extract::{Path, State},
    http::{header, HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use axum_extra::{
    extract::Multipart,
    TypedHeader,
};
use headers::{Authorization, authorization::Bearer};
use std::path::PathBuf;
use std::sync::Arc;
use validator::Validate;

use crate::data_base::album_db;
use crate::data_base::event_db;
use crate::errors::AppError;
use crate::handlers::user::get_user_for_handler_from_token;
use crate::models::*;
use crate::permissions::EventPermissions;
use crate::structs::AppState;

// ====================== Константы ======================
const UPLOAD_BASE: &str = "uploads/events";

pub fn album_dir(event_id: i64, album_id: i64) -> PathBuf {
    PathBuf::from(UPLOAD_BASE)
        .join(event_id.to_string())
        .join("albums")
        .join(album_id.to_string())
        .join("photos")
}

fn photo_url(event_id: i64, album_id: i64, photo_id: i64) -> String {
    format!("/events/{}/albums/{}/photos/{}", event_id, album_id, photo_id)
}

// ====================== 1. Создание альбома ======================
#[utoipa::path(
    post,
    path = "/events/{event_id}/albums",
    tag = "Albums",
    security(("bearerAuth" = [])),
    params(("event_id" = String, Path, description = "Event ID")),
    request_body = CreateAlbumRequest,
    responses(
        (status = 201, description = "Album created", body = AlbumResponse),
        (status = 400, description = "Bad request"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Event not found")
    )
)]
pub async fn create_album_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    Path(event_id_str): Path<String>,
    Json(payload): Json<CreateAlbumRequest>,
) -> Result<impl IntoResponse, AppError> {
    let event_id: i64 = event_id_str.parse().map_err(|_| AppError::BadRequest("Invalid event_id".into()))?;

    if let Err(errors) = payload.validate() {
        return Err(validation_errors_to_response(errors));
    }

    let user = get_user_for_handler_from_token(&state.db_pool, auth.token()).await?;
    let is_member = event_db::check_user_in_event(&state.db_pool, event_id, user.user_id).await?;
    if !is_member {
        return Err(AppError::UserNotInEvent("User not in event".into()));
    }

    // Упрощённые права: только владелец или админ
    let can_create = event_db::has_permission(&state.db_pool, event_id, user.user_id, EventPermissions::OWNER).await?
        || event_db::has_permission(&state.db_pool, event_id, user.user_id, EventPermissions::ADMIN).await?;
    if !can_create {
        return Err(AppError::Forbidden("Not enough permissions to create album".into()));
    }

    let album = album_db::create_album(
        &state.db_pool,
        event_id,
        &payload.title,
        payload.description.as_deref(),
        user.user_id,
    )
    .await?;

    Ok((StatusCode::CREATED, Json(album)))
}

// ====================== 2. Список альбомов события ======================
#[utoipa::path(
    get,
    path = "/events/{event_id}/albums",
    tag = "Albums",
    security(("bearerAuth" = [])),
    params(("event_id" = String, Path, description = "Event ID")),
    responses(
        (status = 200, description = "List of albums", body = Vec<AlbumResponse>),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Event not found")
    )
)]
pub async fn get_albums_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    Path(event_id_str): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let event_id: i64 = event_id_str.parse().map_err(|_| AppError::BadRequest("Invalid event_id".into()))?;

    let user = get_user_for_handler_from_token(&state.db_pool, auth.token()).await?;
    let is_member = event_db::check_user_in_event(&state.db_pool, event_id, user.user_id).await?;
    if !is_member {
        return Err(AppError::UserNotInEvent("User not in event".into()));
    }

    let albums = album_db::get_event_albums(&state.db_pool, event_id).await?;
    Ok((StatusCode::OK, Json(albums)))
}

// ====================== 3. Альбом с фотографиями ======================
#[utoipa::path(
    get,
    path = "/events/{event_id}/albums/{album_id}",
    tag = "Albums",
    security(("bearerAuth" = [])),
    params(
        ("event_id" = String, Path, description = "Event ID"),
        ("album_id" = String, Path, description = "Album ID")
    ),
    responses(
        (status = 200, description = "Album with photos", body = AlbumWithPhotosResponse),
        (status = 404, description = "Album or event not found")
    )
)]
pub async fn get_album_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    Path((event_id_str, album_id_str)): Path<(String, String)>,
) -> Result<impl IntoResponse, AppError> {
    let event_id: i64 = event_id_str.parse().map_err(|_| AppError::BadRequest("Invalid event_id".into()))?;
    let album_id: i64 = album_id_str.parse().map_err(|_| AppError::BadRequest("Invalid album_id".into()))?;

    let user = get_user_for_handler_from_token(&state.db_pool, auth.token()).await?;
    let is_member = event_db::check_user_in_event(&state.db_pool, event_id, user.user_id).await?;
    if !is_member {
        return Err(AppError::UserNotInEvent("User not in event".into()));
    }

    let belongs = album_db::verify_album_in_event(&state.db_pool, album_id, event_id).await?;
    if !belongs {
        return Err(AppError::NotFound("Album not found in this event".into()));
    }

    let mut album = album_db::get_album_with_photos(&state.db_pool, album_id).await?;
    for photo in &mut album.photos {
        photo.url = photo_url(event_id, album_id, photo.photo_id);
    }

    Ok((StatusCode::OK, Json(album)))
}

// ====================== 4. Загрузка фото ======================
#[utoipa::path(
    post,
    path = "/events/{event_id}/albums/{album_id}/photos",
    tag = "Albums",
    security(("bearerAuth" = [])),
    params(
        ("event_id" = String, Path),
        ("album_id" = String, Path)
    ),
    request_body(content_type = "multipart/form-data"),
    responses(
        (status = 201, description = "Photo uploaded", body = PhotoResponse),
        (status = 400, description = "Bad request"),
        (status = 403, description = "Forbidden")
    )
)]
pub async fn upload_photo_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    Path((event_id_str, album_id_str)): Path<(String, String)>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, AppError> {
    let event_id: i64 = event_id_str.parse().map_err(|_| AppError::BadRequest("Invalid event_id".into()))?;
    let album_id: i64 = album_id_str.parse().map_err(|_| AppError::BadRequest("Invalid album_id".into()))?;

    let user = get_user_for_handler_from_token(&state.db_pool, auth.token()).await?;
    let is_member = event_db::check_user_in_event(&state.db_pool, event_id, user.user_id).await?;
    if !is_member {
        return Err(AppError::UserNotInEvent("User not in event".into()));
    }

    let belongs = album_db::verify_album_in_event(&state.db_pool, album_id, event_id).await?;
    if !belongs {
        return Err(AppError::NotFound("Album not found".into()));
    }

    while let Ok(Some(field)) = multipart.next_field().await {
        let field_name = field.name().unwrap_or("").to_string();
        if field_name != "photo" {
            continue;
        }

        let file_name = field.file_name().unwrap_or("photo.jpg").to_string();
        let ext = std::path::Path::new(&file_name)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("jpg");
        let mime_type = field.content_type().unwrap_or("application/octet-stream").to_string();
        let data = field.bytes().await.map_err(|e| {
            tracing::error!("Failed to read file: {}", e);
            AppError::BadRequest("Failed to read file".into())
        })?;
        let file_size = data.len() as i64;

        let photo = album_db::insert_photo(
            &state.db_pool,
            album_id,
            "temp",
            &file_name,
            &mime_type,
            file_size,
            user.user_id,
        )
        .await?;

        let dir = album_dir(event_id, album_id);
        tokio::fs::create_dir_all(&dir).await.map_err(|e| {
            tracing::error!("Failed to create dir: {}", e);
            AppError::Internal("Failed to create directory".into())
        })?;

        let stored_name = format!("{}.{}", photo.photo_id, ext);
        let file_path = dir.join(&stored_name);
        tokio::fs::write(&file_path, &data).await.map_err(|e| {
            tracing::error!("Failed to save file: {}", e);
            AppError::Internal("Failed to save file".into())
        })?;

        sqlx::query!(
            "UPDATE album_photos SET file_name = $1 WHERE photo_id = $2",
            stored_name,
            photo.photo_id,
        )
        .execute(&state.db_pool)
        .await
        .map_err(AppError::DbError)?;

        let response = PhotoResponse {
            photo_id: photo.photo_id,
            file_name: stored_name,
            original_name: Some(file_name),
            mime_type: Some(mime_type),
            file_size: Some(file_size),
            uploaded_by: user.user_id,
            created_at: photo.created_at,
            url: photo_url(event_id, album_id, photo.photo_id),
        };

        return Ok((StatusCode::CREATED, Json(response)));
    }

    Err(AppError::BadRequest("No photo file provided".into()))
}

// ====================== 5. Получение файла фото ======================
#[utoipa::path(
    get,
    path = "/events/{event_id}/albums/{album_id}/photos/{photo_id}",
    tag = "Albums",
    security(("bearerAuth" = [])),
    params(
        ("event_id" = String, Path, description = "Event ID"),
        ("album_id" = String, Path, description = "Album ID"),
        ("photo_id" = String, Path, description = "Photo ID")
    ),
    responses(
        (status = 200, description = "Photo file"),
        (status = 304, description = "Not modified"),
        (status = 403, description = "User not in event or token invalid"),
        (status = 404, description = "Photo not found")
    )
)]
pub async fn get_photo_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    headers: HeaderMap,
    Path((event_id_str, album_id_str, photo_id_str)): Path<(String, String, String)>,
) -> Result<impl IntoResponse, AppError> {
    let event_id: i64 = event_id_str.parse().map_err(|_| AppError::BadRequest("Invalid event_id".into()))?;
    let album_id: i64 = album_id_str.parse().map_err(|_| AppError::BadRequest("Invalid album_id".into()))?;
    let photo_id: i64 = photo_id_str.parse().map_err(|_| AppError::BadRequest("Invalid photo_id".into()))?;

    let user = get_user_for_handler_from_token(&state.db_pool, auth.token()).await?;
    let is_member = event_db::check_user_in_event(&state.db_pool, event_id, user.user_id).await?;
    if !is_member {
        return Err(AppError::UserNotInEvent("User not in event".into()));
    }

    let dir = album_dir(event_id, album_id);
    let mut found_path = None;
    if let Ok(mut entries) = tokio::fs::read_dir(&dir).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.starts_with(&format!("{}.", photo_id)) {
                found_path = Some(entry.path());
                break;
            }
        }
    }

    let path = found_path.ok_or_else(|| AppError::NotFound("Photo not found".into()))?;
    let mime = mime_guess::from_path(&path).first_or_octet_stream();
    let data = tokio::fs::read(&path).await.map_err(|e| {
        tracing::error!("Failed to read photo: {}", e);
        AppError::Internal("Failed to read photo".into())
    })?;

    let hash = blake3::hash(&data);
    let etag = format!("\"{}\"", hash.to_hex());
    if let Some(if_none_match) = headers.get(header::IF_NONE_MATCH) {
        if if_none_match.to_str().unwrap_or("") == etag {
            return Ok(StatusCode::NOT_MODIFIED.into_response());
        }
    }

    let mut response = (StatusCode::OK, data).into_response();
    response.headers_mut().insert(header::CONTENT_TYPE, mime.to_string().parse().unwrap());
    response.headers_mut().insert(header::ETAG, etag.parse().unwrap());
    let cache_value = format!("public, max-age={}", state.config.photo_cache_max_age);
    response.headers_mut().insert(header::CACHE_CONTROL, cache_value.parse().unwrap());

    Ok(response)
}

// ====================== 6. Удаление альбома ======================
#[utoipa::path(
    delete,
    path = "/events/{event_id}/albums/{album_id}",
    tag = "Albums",
    security(("bearerAuth" = [])),
    params(
        ("event_id" = String, Path),
        ("album_id" = String, Path)
    ),
    responses(
        (status = 200, description = "Album deleted", body = SuccessResponse),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Not found")
    )
)]
pub async fn delete_album_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    Path((event_id_str, album_id_str)): Path<(String, String)>,
) -> Result<impl IntoResponse, AppError> {
    let event_id: i64 = event_id_str.parse().map_err(|_| AppError::BadRequest("Invalid event_id".into()))?;
    let album_id: i64 = album_id_str.parse().map_err(|_| AppError::BadRequest("Invalid album_id".into()))?;

    let user = get_user_for_handler_from_token(&state.db_pool, auth.token()).await?;
    let is_member = event_db::check_user_in_event(&state.db_pool, event_id, user.user_id).await?;
    if !is_member {
        return Err(AppError::UserNotInEvent("User not in event".into()));
    }

    let can_delete = event_db::has_permission(&state.db_pool, event_id, user.user_id, EventPermissions::OWNER).await?
        || event_db::has_permission(&state.db_pool, event_id, user.user_id, EventPermissions::ADMIN).await?;
    if !can_delete {
        return Err(AppError::Forbidden("Not enough permissions to delete album".into()));
    }

    album_db::delete_album(&state.db_pool, album_id, event_id).await?;
    Ok((StatusCode::OK, Json(SuccessResponse { success: true })))
}

// ====================== 7. Удаление фото ======================
#[utoipa::path(
    delete,
    path = "/events/{event_id}/albums/{album_id}/photos/{photo_id}",
    tag = "Albums",
    security(("bearerAuth" = [])),
    params(
        ("event_id" = String, Path),
        ("album_id" = String, Path),
        ("photo_id" = String, Path)
    ),
    responses(
        (status = 200, description = "Photo deleted", body = SuccessResponse),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Not found")
    )
)]
pub async fn delete_photo_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    Path((event_id_str, album_id_str, photo_id_str)): Path<(String, String, String)>,
) -> Result<impl IntoResponse, AppError> {
    let event_id: i64 = event_id_str.parse().map_err(|_| AppError::BadRequest("Invalid event_id".into()))?;
    let album_id: i64 = album_id_str.parse().map_err(|_| AppError::BadRequest("Invalid album_id".into()))?;
    let photo_id: i64 = photo_id_str.parse().map_err(|_| AppError::BadRequest("Invalid photo_id".into()))?;

    let user = get_user_for_handler_from_token(&state.db_pool, auth.token()).await?;
    let is_member = event_db::check_user_in_event(&state.db_pool, event_id, user.user_id).await?;
    if !is_member {
        return Err(AppError::UserNotInEvent("User not in event".into()));
    }

    let is_owner_or_admin = event_db::has_permission(&state.db_pool, event_id, user.user_id, EventPermissions::OWNER).await?
        || event_db::has_permission(&state.db_pool, event_id, user.user_id, EventPermissions::ADMIN).await?;

    let photo = sqlx::query!(
        "SELECT uploaded_by FROM album_photos WHERE photo_id = $1 AND is_active = true",
        photo_id
    )
    .fetch_optional(&state.db_pool)
    .await
    .map_err(AppError::DbError)?
    .ok_or(AppError::NotFound("Photo not found".into()))?;

    if !is_owner_or_admin && photo.uploaded_by != user.user_id {
        return Err(AppError::Forbidden("Not enough permissions to delete this photo".into()));
    }

    album_db::delete_photo(&state.db_pool, photo_id, album_id, event_id).await?;
    Ok((StatusCode::OK, Json(SuccessResponse { success: true })))
}

// ====================== Тесты ======================
#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use axum::{body::Body, http::Request, Router, routing};
    use http_body_util::BodyExt;
    use serde_json::json;
    use tower::ServiceExt;
    use tokio::sync::{broadcast, Mutex};

    use crate::config::Config;
    use crate::data_base::user_db;
    use crate::data_base::event_db;
    use crate::permissions::EventPermissions;
    use crate::secrets::verification::VerificationStore;
    use crate::test_utils::setup_test_db;

    async fn create_test_state(pool: &sqlx::PgPool) -> Arc<AppState> {
        Arc::new(AppState {
            tx: broadcast::channel(10).0,
            verification_store: Arc::new(Mutex::new(VerificationStore::new())),
            db_pool: pool.clone(),
            config: Config::from_env(),
        })
    }

    async fn create_user_and_token(pool: &sqlx::PgPool, username: &str, email: &str) -> Result<(i64, String)> {
        let user_id = user_db::create_user_db(pool, username, email, "User", &None, &None).await?;
        let token = format!("token_{}", username);
        user_db::create_token(pool, user_id, &token, chrono::Utc::now() + chrono::Duration::hours(1)).await?;
        Ok((user_id, token))
    }

    async fn create_event(pool: &sqlx::PgPool, user_id: i64) -> Result<i64> {
        let event_id = event_db::create_event(pool, "Test", None, None, None, Some("loc".into()), "#123".into()).await?;
        event_db::add_member(pool, user_id, event_id, EventPermissions::OWNER).await?;
        Ok(event_id)
    }

    #[tokio::test]
    async fn test_create_album_success() -> Result<()> {
        let pool = setup_test_db().await;
        let state = create_test_state(&pool).await;
        let (user_id, token) = create_user_and_token(&pool, "user1", "user1@test.com").await?;
        let event_id = create_event(&pool, user_id).await?;

        let app = Router::new()
            .route("/events/{event_id}/albums", routing::post(create_album_handler))
            .with_state(state);

        let payload = json!({ "title": "My Album", "description": "test" });
        let request = Request::builder()
            .method("POST")
            .uri(&format!("/events/{}/albums", event_id))
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .body(Body::from(payload.to_string()))?;

        let response = app.oneshot(request).await?;
        assert_eq!(response.status(), StatusCode::CREATED);
        let body = response.into_body().collect().await?.to_bytes();
        let album: AlbumResponse = serde_json::from_slice(&body)?;
        assert_eq!(album.title, "My Album");
        Ok(())
    }

    #[tokio::test]
    async fn test_create_album_forbidden() -> Result<()> {
        let pool = setup_test_db().await;
        let state = create_test_state(&pool).await;
        let (user_id, _token) = create_user_and_token(&pool, "user2", "user2@test.com").await?;
        let event_id = create_event(&pool, user_id).await?;
        let (other_id, other_token) = create_user_and_token(&pool, "other", "other@test.com").await?;
        event_db::add_member(&pool, other_id, event_id, EventPermissions::MEMBER).await?;

        let app = Router::new()
            .route("/events/{event_id}/albums", routing::post(create_album_handler))
            .with_state(state);

        let payload = json!({ "title": "Album" });
        let request = Request::builder()
            .method("POST")
            .uri(&format!("/events/{}/albums", event_id))
            .header("Authorization", format!("Bearer {}", other_token))
            .header("Content-Type", "application/json")
            .body(Body::from(payload.to_string()))?;

        let response = app.oneshot(request).await?;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        Ok(())
    }

    #[tokio::test]
    async fn test_get_albums_success() -> Result<()> {
        let pool = setup_test_db().await;
        let state = create_test_state(&pool).await;
        let (user_id, token) = create_user_and_token(&pool, "user3", "user3@test.com").await?;
        let event_id = create_event(&pool, user_id).await?;
        album_db::create_album(&pool, event_id, "First", None, user_id).await?;

        let app = Router::new()
            .route("/events/{event_id}/albums", routing::get(get_albums_handler))
            .with_state(state);

        let request = Request::builder()
            .method("GET")
            .uri(&format!("/events/{}/albums", event_id))
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())?;

        let response = app.oneshot(request).await?;
        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await?.to_bytes();
        let albums: Vec<AlbumResponse> = serde_json::from_slice(&body)?;
        assert_eq!(albums.len(), 1);
        assert_eq!(albums[0].title, "First");
        Ok(())
    }

    #[tokio::test]
    async fn test_get_album_with_photos() -> Result<()> {
        let pool = setup_test_db().await;
        let state = create_test_state(&pool).await;
        let (user_id, token) = create_user_and_token(&pool, "user4", "user4@test.com").await?;
        let event_id = create_event(&pool, user_id).await?;
        let album = album_db::create_album(&pool, event_id, "Album", None, user_id).await?;
        album_db::insert_photo(&pool, album.album_id, "1.jpg", "orig.jpg", "image/jpeg", 1000, user_id).await?;

        let app = Router::new()
            .route("/events/{event_id}/albums/{album_id}", routing::get(get_album_handler))
            .with_state(state);

        let request = Request::builder()
            .method("GET")
            .uri(&format!("/events/{}/albums/{}", event_id, album.album_id))
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())?;

        let response = app.oneshot(request).await?;
        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await?.to_bytes();
        let album_resp: AlbumWithPhotosResponse = serde_json::from_slice(&body)?;
        assert_eq!(album_resp.photos.len(), 1);
        assert!(album_resp.photos[0].url.contains("/photos/"));
        Ok(())
    }

    #[tokio::test]
    async fn test_upload_photo_success() -> Result<()> {
        let pool = setup_test_db().await;
        let state = create_test_state(&pool).await;
        let (user_id, token) = create_user_and_token(&pool, "user5", "user5@test.com").await?;
        let event_id = create_event(&pool, user_id).await?;
        let album = album_db::create_album(&pool, event_id, "Upload", None, user_id).await?;

        let app = Router::new()
            .route("/events/{event_id}/albums/{album_id}/photos", routing::post(upload_photo_handler))
            .with_state(state);

        let boundary = "testboundary";
        let body_data = format!(
            "--{}\r\nContent-Disposition: form-data; name=\"photo\"; filename=\"test.jpg\"\r\nContent-Type: image/jpeg\r\n\r\nfakeimagecontent\r\n--{}--\r\n",
            boundary, boundary
        );
        let request = Request::builder()
            .method("POST")
            .uri(&format!("/events/{}/albums/{}/photos", event_id, album.album_id))
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", format!("multipart/form-data; boundary={}", boundary))
            .body(Body::from(body_data))?;

        let response = app.oneshot(request).await?;
        assert_eq!(response.status(), StatusCode::CREATED);
        let body = response.into_body().collect().await?.to_bytes();
        let photo: PhotoResponse = serde_json::from_slice(&body)?;
        assert!(photo.photo_id > 0);
        Ok(())
    }

    #[tokio::test]
    async fn test_get_photo_authorized() -> Result<()> {
        let pool = setup_test_db().await;
        let state = create_test_state(&pool).await;
        let (user_id, token) = create_user_and_token(&pool, "user6", "user6@test.com").await?;
        let event_id = create_event(&pool, user_id).await?;
        let album = album_db::create_album(&pool, event_id, "Photo", None, user_id).await?;
        let photo = album_db::insert_photo(&pool, album.album_id, "1.jpg", "o.jpg", "image/jpeg", 10, user_id).await?;
        // Обновляем file_name на правильный формат и кладём файл с таким именем
        let correct_name = format!("{}.jpg", photo.photo_id);
        sqlx::query!("UPDATE album_photos SET file_name = $1 WHERE photo_id = $2", correct_name, photo.photo_id)
            .execute(&pool)
            .await?;
        let dir = album_dir(event_id, album.album_id);
        tokio::fs::create_dir_all(&dir).await?;
        tokio::fs::write(dir.join(&correct_name), b"test").await?;

        let app = Router::new()
            .route("/events/{event_id}/albums/{album_id}/photos/{photo_id}", routing::get(get_photo_handler))
            .with_state(state);

        let request = Request::builder()
            .method("GET")
            .uri(&format!("/events/{}/albums/{}/photos/{}", event_id, album.album_id, photo.photo_id))
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())?;

        let response = app.oneshot(request).await?;
        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await?.to_bytes();
        assert_eq!(&body[..], b"test");
        Ok(())
    }

    #[tokio::test]
    async fn test_get_photo_unauthorized() -> Result<()> {
        let pool = setup_test_db().await;
        let state = create_test_state(&pool).await;
        let (user_id, _) = create_user_and_token(&pool, "user7", "user7@test.com").await?;
        let event_id = create_event(&pool, user_id).await?;
        let album = album_db::create_album(&pool, event_id, "Unauth", None, user_id).await?;
        let photo = album_db::insert_photo(&pool, album.album_id, "2.jpg", "o.jpg", "image/jpeg", 10, user_id).await?;
        let dir = album_dir(event_id, album.album_id);
        tokio::fs::create_dir_all(&dir).await?;
        tokio::fs::write(dir.join("2.jpg"), b"secret").await?;

        let app = Router::new()
            .route("/events/{event_id}/albums/{album_id}/photos/{photo_id}", routing::get(get_photo_handler))
            .with_state(state);

        let request = Request::builder()
            .method("GET")
            .uri(&format!("/events/{}/albums/{}/photos/{}", event_id, album.album_id, photo.photo_id))
            .header("Authorization", "Bearer invalidtoken")
            .body(Body::empty())?;

        let response = app.oneshot(request).await?;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        Ok(())
    }

    #[tokio::test]
    async fn test_delete_album_success() -> Result<()> {
        let pool = setup_test_db().await;
        let state = create_test_state(&pool).await;
        let (user_id, token) = create_user_and_token(&pool, "user8", "user8@test.com").await?;
        let event_id = create_event(&pool, user_id).await?;
        let album = album_db::create_album(&pool, event_id, "Del", None, user_id).await?;

        let app = Router::new()
            .route("/events/{event_id}/albums/{album_id}", routing::delete(delete_album_handler))
            .with_state(state);

        let request = Request::builder()
            .method("DELETE")
            .uri(&format!("/events/{}/albums/{}", event_id, album.album_id))
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())?;

        let response = app.oneshot(request).await?;
        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await?.to_bytes();
        let success: SuccessResponse = serde_json::from_slice(&body)?;
        assert!(success.success);

        let albums = album_db::get_event_albums(&pool, event_id).await?;
        assert!(albums.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn test_delete_photo_success() -> Result<()> {
        let pool = setup_test_db().await;
        let state = create_test_state(&pool).await;
        let (user_id, token) = create_user_and_token(&pool, "user9", "user9@test.com").await?;
        let event_id = create_event(&pool, user_id).await?;
        let album = album_db::create_album(&pool, event_id, "DelPhoto", None, user_id).await?;
        let photo = album_db::insert_photo(&pool, album.album_id, "3.jpg", "o.jpg", "image/jpeg", 10, user_id).await?;
        let dir = album_dir(event_id, album.album_id);
        tokio::fs::create_dir_all(&dir).await?;
        tokio::fs::write(dir.join("3.jpg"), b"data").await?;

        let app = Router::new()
            .route("/events/{event_id}/albums/{album_id}/photos/{photo_id}", routing::delete(delete_photo_handler))
            .with_state(state);

        let request = Request::builder()
            .method("DELETE")
            .uri(&format!("/events/{}/albums/{}/photos/{}", event_id, album.album_id, photo.photo_id))
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())?;

        let response = app.oneshot(request).await?;
        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await?.to_bytes();
        let success: SuccessResponse = serde_json::from_slice(&body)?;
        assert!(success.success);

        let photos = album_db::get_photos_by_album(&pool, album.album_id).await?;
        assert!(photos.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn test_delete_album_removes_directory() -> Result<()> {
        let pool = setup_test_db().await;
        let state = create_test_state(&pool).await;
        let (user_id, token) = create_user_and_token(&pool, "album_dir", "dir@test.com").await?;
        let event_id = create_event(&pool, user_id).await?;
        let album = album_db::create_album(&pool, event_id, "DirTest", None, user_id).await?;

        let dir = album_dir(event_id, album.album_id);
        tokio::fs::create_dir_all(&dir).await?;
        assert!(dir.exists());

        let app = Router::new()
            .route("/events/{event_id}/albums/{album_id}", routing::delete(delete_album_handler))
            .with_state(state);

        let request = Request::builder()
            .method("DELETE")
            .uri(&format!("/events/{}/albums/{}", event_id, album.album_id))
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())?;
        let response = app.oneshot(request).await?;
        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await?.to_bytes();
        let success: SuccessResponse = serde_json::from_slice(&body)?;
        assert!(success.success);
        assert!(!dir.exists());
        Ok(())
    }

    #[tokio::test]
    async fn test_delete_photo_removes_file() -> Result<()> {
        let pool = setup_test_db().await;
        let state = create_test_state(&pool).await;
        let (user_id, token) = create_user_and_token(&pool, "photo_file", "photofile@test.com").await?;
        let event_id = create_event(&pool, user_id).await?;
        let album = album_db::create_album(&pool, event_id, "FileTest", None, user_id).await?;
        let photo = album_db::insert_photo(&pool, album.album_id, "temp", "o.jpg", "image/jpeg", 10, user_id).await?;
        let correct_name = format!("{}.jpg", photo.photo_id);
        sqlx::query!("UPDATE album_photos SET file_name = $1 WHERE photo_id = $2", correct_name, photo.photo_id)
            .execute(&pool).await?;
        let dir = album_dir(event_id, album.album_id);
        tokio::fs::create_dir_all(&dir).await?;
        let file_path = dir.join(&correct_name);
        tokio::fs::write(&file_path, b"test").await?;
        assert!(file_path.exists());

        let app = Router::new()
            .route("/events/{event_id}/albums/{album_id}/photos/{photo_id}", routing::delete(delete_photo_handler))
            .with_state(state);

        let request = Request::builder()
            .method("DELETE")
            .uri(&format!("/events/{}/albums/{}/photos/{}", event_id, album.album_id, photo.photo_id))
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())?;
        let response = app.oneshot(request).await?;
        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await?.to_bytes();
        let success: SuccessResponse = serde_json::from_slice(&body)?;
        assert!(success.success);
        assert!(!file_path.exists());
        Ok(())
    }

    #[tokio::test]
    async fn test_create_album_invalid_event_id() -> Result<()> {
        let pool = setup_test_db().await;
        let state = create_test_state(&pool).await;
        let (_user_id, token) = create_user_and_token(&pool, "inval_id", "inval@test.com").await?;

        let app = Router::new()
            .route("/events/{event_id}/albums", routing::post(create_album_handler))
            .with_state(state);

        let payload = json!({ "title": "Test" });
        let request = Request::builder()
            .method("POST")
            .uri("/events/abc/albums") // невалидный event_id
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .body(Body::from(payload.to_string()))?;
        let response = app.oneshot(request).await?;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        Ok(())
    }

    #[tokio::test]
    async fn test_delete_photo_by_non_owner() -> Result<()> {
        let pool = setup_test_db().await;
        let state = create_test_state(&pool).await;
        let (owner_id, _owner_token) = create_user_and_token(&pool, "owner_ph", "owner_ph@test.com").await?;
        let (other_id, other_token) = create_user_and_token(&pool, "other_ph", "other_ph@test.com").await?;
        let event_id = create_event(&pool, owner_id).await?;
        event_db::add_member(&pool, other_id, event_id, EventPermissions::MEMBER).await?;

        let album = album_db::create_album(&pool, event_id, "DelPhotoTest", None, owner_id).await?;
        let photo = album_db::insert_photo(&pool, album.album_id, "temp", "o.jpg", "image/jpeg", 10, owner_id).await?;
        let correct_name = format!("{}.jpg", photo.photo_id);
        sqlx::query!("UPDATE album_photos SET file_name = $1 WHERE photo_id = $2", correct_name, photo.photo_id)
            .execute(&pool).await?;

        let app = Router::new()
            .route("/events/{event_id}/albums/{album_id}/photos/{photo_id}", routing::delete(delete_photo_handler))
            .with_state(state);

        let request = Request::builder()
            .method("DELETE")
            .uri(&format!("/events/{}/albums/{}/photos/{}", event_id, album.album_id, photo.photo_id))
            .header("Authorization", format!("Bearer {}", other_token))
            .body(Body::empty())?;
        let response = app.oneshot(request).await?;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        Ok(())
    }
}
use axum::{
    extract::{Multipart, Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use axum_extra::{TypedHeader, headers::Authorization};
use headers::authorization::Bearer;
use std::sync::Arc;
use uuid::Uuid;
use tokio::fs;
use std::path::PathBuf;
use chrono::{DateTime, Utc, NaiveDate, NaiveDateTime};
use image::{self, GenericImageView};
use sha2::{Sha256, Digest};

use crate::{
    errors::AppError,
    models::*,
    data_base::{event_repo, user_db},
    AppState,
};

// Папка для аватарок событий
const EVENT_AVATAR_DIR: &str = "event-avatars";

fn parse_optional_datetime(s: &Option<String>) -> Result<Option<DateTime<Utc>>, AppError> {
    match s {
        None => Ok(None),
        Some(str) if str.is_empty() => Ok(None),
        Some(str) => {
            //Пробуем парсить как ISO 8601 с временем
            if let Ok(dt) = DateTime::parse_from_rfc3339(str) {
                return Ok(Some(dt.with_timezone(&Utc)));
            }
            //Пробуем парсить как дату без времени
            if let Ok(naive_date) = NaiveDate::parse_from_str(str, "%Y-%m-%d") {
                let dt = naive_date.and_hms_opt(0, 0, 0)
                    .map(|ndt| DateTime::<Utc>::from_utc(ndt, Utc))
                    .flatten();
                if let Some(dt) = dt {
                    return Ok(Some(dt));
                }
            }
            //Пробуем как "YYYY-MM-DD HH:MM:SS"
            if let Ok(naive_dt) = NaiveDateTime::parse_from_str(str, "%Y-%m-%d %H:%M:%S") {
                return Ok(Some(DateTime::from_utc(naive_dt, Utc)));
            }
            Err(AppError::BadRequest(format!("Invalid date format: {}", str)))
        }
    }
}

//Валидация hex цвета
fn validate_color(color: &str) -> Result<(), AppError> {
    if color.len() == 7 && color.starts_with('#') && color[1..].chars().all(|c| c.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(AppError::BadRequest("Invalid color format".into()))
    }
}

// Получить пользователя из токена
async fn get_user_from_token(
    state: &Arc<AppState>,
    auth: &TypedHeader<Authorization<Bearer>>,
) -> Result<crate::structs::User, AppError> {
    let token = auth.token();
    let user = user_db::find_user_by_token(&state.db_pool, token)
        .await?
        .ok_or(AppError::InvalidToken)?;
    Ok(user)
}

// Преобразование строки БД в ответ JSON
fn event_row_to_response(row: &event_repo::EventRow) -> EventResponse {
    EventResponse {
        id: row.id,
        title: row.title.clone(),
        description: row.description.clone(),
        startDateTime: row.start_date_time.map(|d| d.to_rfc3339()),
        endDateTime: row.end_date_time.map(|d| d.to_rfc3339()),
        color: row.color.clone(),
        created_by: row.created_by.to_string(),
        createdAt: row.created_at.to_rfc3339(),
        status: row.status.clone(),
    }
}

// POST /events
pub async fn create_event_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    Json(payload): Json<CreateEventRequest>,
) -> Result<impl IntoResponse, AppError> {
    // Валидация title
    if payload.title.trim().is_empty() {
        return Err(AppError::BadRequest("Title must not be empty".into()));
    }
    // Парсинг дат
    let start = parse_optional_datetime(&payload.startDateTime)?;
    let end = parse_optional_datetime(&payload.endDateTime)?;
    // Валидация цвета
    if let Some(ref c) = payload.color {
        validate_color(c)?;
    }

    let user = get_user_from_token(&state, &auth).await?;
    let new_event = NewEvent {
        title: payload.title,
        description: payload.description,
        start_date_time: start,
        end_date_time: end,
        color: payload.color,
        created_by: user.user_id,
    };

    let event_row = event_repo::create_event(&state.db_pool, new_event).await?;
    let resp = event_row_to_response(&event_row);

    Ok((StatusCode::CREATED, Json(resp)))
}

// GET /events?status=active&limit=20&offset=0
pub async fn get_events_list_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    Query(query): Query<EventsQuery>,
) -> Result<impl IntoResponse, AppError> {
    let user = get_user_from_token(&state, &auth).await?;

    let status = match query.status.as_str() {
        "active" | "archived" => query.status.clone(),
        _ => return Err(AppError::BadRequest("Status must be 'active' or 'archived'".into())),
    };

    let limit = query.limit.unwrap_or(20).min(100).max(1);
    let offset = query.offset.unwrap_or(0).max(0);

    let (rows, total) = event_repo::get_events_by_user(
        &state.db_pool,
        user.user_id,
        &status,
        limit,
        offset,
    )
    .await?;

    let items: Vec<EventResponse> = rows.iter().map(|r| event_row_to_response(r)).collect();

    Ok(Json(EventListResponse { items, total }))
}

// GET /events/{id}
pub async fn get_event_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let user = get_user_from_token(&state, &auth).await?;
    let event = event_repo::get_event_by_id(&state.db_pool, id)
        .await?
        .ok_or(AppError::EventNotFound)?;

    // Проверка прав: только создатель имеет доступ
    if event.created_by != user.user_id {
        return Err(AppError::EventNotFound);
    }

    Ok(Json(event_row_to_response(&event)))
}

// PUT /events/{id}
pub async fn update_event_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateEventRequest>,
) -> Result<impl IntoResponse, AppError> {
    let user = get_user_from_token(&state, &auth).await?;
    let event = event_repo::get_event_by_id(&state.db_pool, id)
        .await?
        .ok_or(AppError::EventNotFound)?;

    if event.created_by != user.user_id {
        return Err(AppError::EventNotFound);
    }

    // Парсим даты если переданы
    let start = parse_optional_datetime(&payload.startDateTime)?;
    let end = parse_optional_datetime(&payload.endDateTime)?;

    // Валидация цвета
    if let Some(ref c) = payload.color {
        validate_color(c)?;
    }

    let updated_row = event_repo::update_event(&state.db_pool, id, payload, start, end).await?;
    Ok(Json(event_row_to_response(&updated_row)))
}

// PATCH /events/{id}/status
pub async fn change_status_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    Path(id): Path<Uuid>,
    Json(payload): Json<ChangeEventStatusRequest>,
) -> Result<impl IntoResponse, AppError> {
    let user = get_user_from_token(&state, &auth).await?;
    let event = event_repo::get_event_by_id(&state.db_pool, id)
        .await?
        .ok_or(AppError::EventNotFound)?;

    if event.created_by != user.user_id {
        return Err(AppError::EventNotFound);
    }

    match payload.status.as_str() {
        "active" | "archived" => {},
        _ => return Err(AppError::BadRequest("Status must be 'active' or 'archived'".into())),
    }

    event_repo::update_event_status(&state.db_pool, id, &payload.status).await?;
    Ok(StatusCode::OK)   // Пустое тело
}

// POST /events/{id}/avatar
pub async fn upload_event_avatar_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    Path(id): Path<Uuid>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, AppError> {
    let user = get_user_from_token(&state, &auth).await?;
    let event = event_repo::get_event_by_id(&state.db_pool, id)
        .await?
        .ok_or(AppError::EventNotFound)?;

    if event.created_by != user.user_id {
        return Err(AppError::EventNotFound);
    }

    while let Ok(Some(field)) = multipart.next_field().await {
        if field.name() != Some("avatar") {
            continue;
        }

        let data = field.bytes().await.map_err(|_| AppError::BadRequest("Failed to read file".into()))?;
        if data.len() > 5 * 1024 * 1024 {
            return Err(AppError::BadRequest("File too large (max 5 MB)".into()));
        }

        // Конвертируем в JPEG и уменьшаем до разумного размера
        let img = image::load_from_memory(&data)
            .map_err(|_| AppError::BadRequest("Invalid image".into()))?;
        let (w, h) = img.dimensions();
        let new_w = if w > 500 { 500 } else { w };
        let new_h = if h > 500 { 500 } else { h };
        let thumbnail = img.thumbnail(new_w, new_h);
        let mut jpeg_bytes = Vec::new();
        thumbnail.write_to(&mut std::io::Cursor::new(&mut jpeg_bytes), image::ImageOutputFormat::Jpeg(90))
            .map_err(|_| AppError::Internal("Failed to process image".into()))?;

        let dir = PathBuf::from(EVENT_AVATAR_DIR);
        fs::create_dir_all(&dir).await.map_err(|e| AppError::Internal(e.to_string()))?;
        let file_path = dir.join(format!("{}.jpg", id));
        fs::write(&file_path, &jpeg_bytes).await.map_err(|e| AppError::Internal(e.to_string()))?;

        event_repo::set_avatar_uploaded(&state.db_pool, id, true).await?;

        return Ok(Json(serde_json::json!({"success": true})));
    }

    Err(AppError::BadRequest("No avatar field in form".into()))
}

// GET /event-avatars/{eventId}.jpg
pub async fn get_event_avatar_handler(
    Path(filename): Path<String>,
    headers: HeaderMap,
) -> impl IntoResponse {
    // Проверяем, что имя файла имеет формат UUID.jpg
    let event_id_str = filename.strip_suffix(".jpg")
        .ok_or((StatusCode::BAD_REQUEST, "Invalid file extension"))?;
    let event_id = Uuid::parse_str(event_id_str)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid UUID"))?;

    let file_path = PathBuf::from(EVENT_AVATAR_DIR).join(format!("{}.jpg", event_id));
    match fs::read(&file_path).await {
        Ok(data) => {
            // Вычисляем ETag (SHA-256 хэш)
            let hash = Sha256::digest(&data);
            let etag = format!("\"{}\"", hex::encode(hash));
            if let Some(if_none_match) = headers.get(header::IF_NONE_MATCH) {
                if if_none_match.to_str().unwrap_or("") == etag {
                    return (StatusCode::NOT_MODIFIED, HeaderMap::new()).into_response();
                }
            }

            let mut response_headers = HeaderMap::new();
            response_headers.insert(header::CONTENT_TYPE, "image/jpeg".parse().unwrap());
            response_headers.insert(header::ETAG, etag.parse().unwrap());
            response_headers.insert(
                header::CACHE_CONTROL,
                "max-age=3600, must-revalidate".parse().unwrap(),
            );
            (StatusCode::OK, response_headers, data).into_response()
        }
        Err(_) => (StatusCode::NOT_FOUND, "Avatar not found").into_response(),
    }
}
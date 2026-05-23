use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use axum_extra::TypedHeader;
use headers::{Authorization, authorization::Bearer};
use std::sync::Arc;
use crate::{
    AppState, AppError,
    data_base::{
        user_db::find_user_by_token,
        event_db::*,
    },
    models::*,
    permissions::*,
    structs::Events,
};
use chrono::{DateTime, Utc};

// POST /events
pub async fn create_event_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    Json(payload): Json<CreateEventRequest>,
) -> Result<impl IntoResponse, AppError> {
    let user = find_user_by_token(&state.db_pool, auth.token()).await?
        .ok_or(AppError::UserNotFound)?;

    let start_date = payload.start_date_time
        .map(|s| s.parse::<DateTime<Utc>>().map_err(|_| AppError::BadRequest("Invalid start date".into())))
        .transpose()?;
    let end_date = payload.end_date_time
        .map(|s| s.parse::<DateTime<Utc>>().map_err(|_| AppError::BadRequest("Invalid end date".into())))
        .transpose()?;

    let event_id = create_event(
        &state.db_pool,
        &payload.title,
        payload.description.as_deref(),
        start_date,
        end_date,
        payload.color,
    ).await?;

    // Добавляем создателя как владельца
    add_member(&state.db_pool, user.user_id, event_id, EventPermissions::OWNER, 2).await?;

    let event = get_event_by_id(&state.db_pool, event_id).await?;

    let response = CreateEventResponse {
        id: event.event_id.to_string(),
        title: event.event_name,
        description: event.description_event,
        location: Some("test".to_string()), // TODO: заменить на реальное местоположение
        start_date_time: event.start_date.map(|dt| dt.to_rfc3339()),
        end_date_time: event.end_date.map(|dt| dt.to_rfc3339()),
        color: event.color,
        created_by: user.user_id.to_string(),
        created_at: event.created_at.to_rfc3339(),
        status: event.is_active.to_string(),
    };

    Ok((StatusCode::CREATED, Json(response)))
}

// GET /events
pub async fn list_events_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
) -> Result<impl IntoResponse, AppError> {
    let user = find_user_by_token(&state.db_pool, auth.token()).await?
        .ok_or(AppError::UserNotFound)?;

    let events = get_user_events(&state.db_pool, user.user_id, 100, 0).await?;
    Ok((StatusCode::OK, Json(json!({ "events": events }))))
}

// GET /events/:event_id
pub async fn get_event_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    Path(event_id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    let user = find_user_by_token(&state.db_pool, auth.token()).await?
        .ok_or(AppError::UserNotFound)?;

    let event = get_event_by_id(&state.db_pool, event_id).await?;

    if !is_user_in_event(&state.db_pool, user.user_id, event_id).await? {
        return Err(AppError::UserNotInEvent("You are not a member".into()));
    }

    let members = get_event_members(&state.db_pool, event_id).await?;
    let permissions = get_user_permissions(&state.db_pool, event_id, user.user_id).await?;
    let invite_url = create_event_token(&state.db_pool, event_id, 24).await.ok();

    Ok((StatusCode::OK, Json(GetEventResponse {
        event,
        invite_url,
        members,
        permissions: permissions.get_bits().to_string(),
    })))
}

// PUT /events/:event_id
pub async fn update_event_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    Path(event_id): Path<i64>,
    Json(payload): Json<UpdateEventRequest>,
) -> Result<impl IntoResponse, AppError> {
    let user = find_user_by_token(&state.db_pool, auth.token()).await?
        .ok_or(AppError::UserNotFound)?;
    let event = get_event_by_id(&state.db_pool, event_id).await?;

    check_user_permissions(&state.db_pool, &event, &user, EventPermissions::EDIT_EVENT).await?;

    let start_date = payload.start_date_time
        .map(|s| s.parse::<DateTime<Utc>>().map_err(|_| AppError::BadRequest("Invalid start date".into())))
        .transpose()?;
    let end_date = payload.end_date_time
        .map(|s| s.parse::<DateTime<Utc>>().map_err(|_| AppError::BadRequest("Invalid end date".into())))
        .transpose()?;

    update_event(
        &state.db_pool,
        event_id,
        payload.title.as_deref(),
        payload.description.as_deref(),
        start_date,
        end_date,
        payload.is_active,
    ).await?;

    if let Some(status_id) = payload.status_id {
        update_event_status(&state.db_pool, event_id, status_id).await?;
    }

    Ok((StatusCode::NO_CONTENT, Json(json!({ "success": true }))))
}

// DELETE /events/:event_id
pub async fn delete_event_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    Path(event_id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    let user = find_user_by_token(&state.db_pool, auth.token()).await?
        .ok_or(AppError::UserNotFound)?;
    let event = get_event_by_id(&state.db_pool, event_id).await?;

    check_user_permissions(&state.db_pool, &event, &user, EventPermissions::OWNER).await?;

    // Мягкое удаление или полное? Здесь – деактивируем событие
    sqlx::query!(
        "UPDATE events SET is_active = false WHERE event_id = $1",
        event_id
    )
    .execute(&state.db_pool)
    .await?;

    Ok((StatusCode::NO_CONTENT, Json(json!({ "success": true }))))
}

// POST /events/:event_id/join
pub async fn join_event_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    Path(event_id): Path<i64>,
    Json(payload): Json<JoinEventRequest>,
) -> Result<impl IntoResponse, AppError> {
    let user = find_user_by_token(&state.db_pool, auth.token()).await?
        .ok_or(AppError::UserNotFound)?;

    let event_id_from_token = get_event_id_by_token(&state.db_pool, &payload.invite_token).await?
        .ok_or(AppError::BadRequest("Invalid or expired invite token".into()))?;

    if event_id_from_token != event_id {
        return Err(AppError::BadRequest("Token does not match event".into()));
    }

    // Добавляем участника с базовыми правами MEMBER
    add_member(&state.db_pool, user.user_id, event_id, EventPermissions::MEMBER, 2).await?;

    Ok((StatusCode::NO_CONTENT, Json(json!({ "success": true }))))
}

// POST /events/:event_id/members
pub async fn add_member_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    Path(event_id): Path<i64>,
    Json(payload): Json<AddMemberRequest>,
) -> Result<impl IntoResponse, AppError> {
    let user = find_user_by_token(&state.db_pool, auth.token()).await?
        .ok_or(AppError::UserNotFound)?;
    let event = get_event_by_id(&state.db_pool, event_id).await?;

    check_user_permissions(&state.db_pool, &event, &user, EventPermissions::INVITE).await?;

    add_member(&state.db_pool, payload.user_id, event_id, payload.permissions, payload.status_id).await?;

    Ok((StatusCode::NO_CONTENT, Json(json!({ "success": true }))))
}

// DELETE /events/:event_id/members/:user_id
pub async fn remove_member_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    Path((event_id, user_id)): Path<(i64, i64)>,
) -> Result<impl IntoResponse, AppError> {
    let current_user = find_user_by_token(&state.db_pool, auth.token()).await?
        .ok_or(AppError::UserNotFound)?;
    let event = get_event_by_id(&state.db_pool, event_id).await?;

    // Разрешаем удалять себя или если есть права DELETE_MEMBER
    if current_user.user_id != user_id {
        check_user_permissions(&state.db_pool, &event, &current_user, EventPermissions::DELETE_MEMBER).await?;
    } else {
        // Не даём удалить владельца (OWNER)
        let perms = get_user_permissions(&state.db_pool, event_id, user_id).await?;
        if perms.check_permission(EventPermissions::OWNER) {
            return Err(AppError::BadRequest("Cannot remove the owner".into()));
        }
    }

    remove_member(&state.db_pool, user_id, event_id).await?;
    Ok((StatusCode::NO_CONTENT, Json(json!({ "success": true }))))
}

// PUT /events/:event_id/members/:user_id/permissions
pub async fn update_member_permissions_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    Path((event_id, user_id)): Path<(i64, i64)>,
    Json(payload): Json<UpdateMemberPermissionsRequest>,
) -> Result<impl IntoResponse, AppError> {
    let current_user = find_user_by_token(&state.db_pool, auth.token()).await?
        .ok_or(AppError::UserNotFound)?;
    let event = get_event_by_id(&state.db_pool, event_id).await?;

    check_user_permissions(&state.db_pool, &event, &current_user, EventPermissions::UPDATE_PERMISSIONS).await?;

    update_user_permissions(&state.db_pool, event_id, user_id, payload.new_permissions).await?;

    Ok((StatusCode::NO_CONTENT, Json(json!({ "success": true }))))
}
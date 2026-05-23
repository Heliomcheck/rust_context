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
    data_base::user_db::find_user_by_token,
    data_base::event_db::{get_event_by_id, is_user_in_event},
    models::*,
    permissions::*,
    plainning_modules::item::*,
};

// POST /events/:event_id/items
pub async fn create_item_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    Path(event_id): Path<i64>,
    Json(payload): Json<CreateItemRequest>,
) -> Result<impl IntoResponse, AppError> {
    let user = find_user_by_token(&state.db_pool, auth.token()).await?
        .ok_or(AppError::UserNotFound)?;
    let event = get_event_by_id(&state.db_pool, event_id).await?;

    if !is_user_in_event(&state.db_pool, user.user_id, event_id).await? {
        return Err(AppError::UserNotInEvent("Not a member".into()));
    }
    // Требуем право CREATE_MODULE (или можно создать отдельное)
    check_user_permissions(&state.db_pool, &event, &user, EventPermissions::CREATE_MODULE).await?;

    let item = create_item(&state.db_pool, event_id, &payload.content, user.user_id).await?;
    Ok((StatusCode::CREATED, Json(ItemResponse { item })))
}

// GET /events/:event_id/items
pub async fn list_items_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    Path(event_id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    let user = find_user_by_token(&state.db_pool, auth.token()).await?
        .ok_or(AppError::UserNotFound)?;

    if !is_user_in_event(&state.db_pool, user.user_id, event_id).await? {
        return Err(AppError::UserNotInEvent("Not a member".into()));
    }

    let items = get_items(&state.db_pool, event_id).await?;
    Ok((StatusCode::OK, Json(json!({ "items": items }))))
}

// PUT /events/:event_id/items/:item_id
pub async fn update_item_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    Path((event_id, item_id)): Path<(i64, i64)>,
    Json(payload): Json<UpdateItemRequest>,
) -> Result<impl IntoResponse, AppError> {
    let user = find_user_by_token(&state.db_pool, auth.token()).await?
        .ok_or(AppError::UserNotFound)?;
    let event = get_event_by_id(&state.db_pool, event_id).await?;

    if !is_user_in_event(&state.db_pool, user.user_id, event_id).await? {
        return Err(AppError::UserNotInEvent("Not a member".into()));
    }
    check_user_permissions(&state.db_pool, &event, &user, EventPermissions::UPDATE_MODULE).await?;

    let updated = update_item(&state.db_pool, item_id, payload.content.as_deref(), payload.done).await?;
    if !updated {
        return Err(AppError::BadRequest("Item not found".into()));
    }
    Ok((StatusCode::NO_CONTENT, Json(json!({ "success": true }))))
}

// DELETE /events/:event_id/items/:item_id
pub async fn delete_item_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    Path((event_id, item_id)): Path<(i64, i64)>,
) -> Result<impl IntoResponse, AppError> {
    let user = find_user_by_token(&state.db_pool, auth.token()).await?
        .ok_or(AppError::UserNotFound)?;
    let event = get_event_by_id(&state.db_pool, event_id).await?;

    if !is_user_in_event(&state.db_pool, user.user_id, event_id).await? {
        return Err(AppError::UserNotInEvent("Not a member".into()));
    }
    check_user_permissions(&state.db_pool, &event, &user, EventPermissions::DELETE_MODULE).await?;

    let deleted = delete_item(&state.db_pool, item_id).await?;
    if !deleted {
        return Err(AppError::BadRequest("Item not found".into()));
    }
    Ok((StatusCode::NO_CONTENT, Json(json!({ "success": true }))))
}
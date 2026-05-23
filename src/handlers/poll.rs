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
    plainning_modules::poll::*,
};

// POST /events/:event_id/polls
pub async fn create_poll_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    Path(event_id): Path<i64>,
    Json(payload): Json<CreatePollRequest>,
) -> Result<impl IntoResponse, AppError> {
    let user = find_user_by_token(&state.db_pool, auth.token()).await?
        .ok_or(AppError::UserNotFound)?;
    let event = get_event_by_id(&state.db_pool, event_id).await?;

    if !is_user_in_event(&state.db_pool, user.user_id, event_id).await? {
        return Err(AppError::UserNotInEvent("Not a member".into()));
    }
    check_user_permissions(&state.db_pool, &event, &user, EventPermissions::CREATE_MODULE).await?;

    let poll_id = create_poll(
        &state.db_pool,
        event_id,
        payload.question,
        user.user_id,
        payload.options,
        payload.more_than_one_vote,
    ).await?;

    Ok((StatusCode::CREATED, Json(json!({ "poll_id": poll_id }))))
}

// GET /events/:event_id/polls
pub async fn list_polls_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    Path(event_id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    let user = find_user_by_token(&state.db_pool, auth.token()).await?
        .ok_or(AppError::UserNotFound)?;
    let event = get_event_by_id(&state.db_pool, event_id).await?;

    if !is_user_in_event(&state.db_pool, user.user_id, event_id).await? {
        return Err(AppError::UserNotInEvent("Not a member".into()));
    }

    let polls = get_event_polls(&state.db_pool, event_id).await?;
    Ok((StatusCode::OK, Json(json!({ "polls": polls }))))
}

// GET /events/:event_id/polls/:poll_id
pub async fn get_poll_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    Path((event_id, poll_id)): Path<(i64, i64)>,
) -> Result<impl IntoResponse, AppError> {
    let user = find_user_by_token(&state.db_pool, auth.token()).await?
        .ok_or(AppError::UserNotFound)?;

    if !is_user_in_event(&state.db_pool, user.user_id, event_id).await? {
        return Err(AppError::UserNotInEvent("Not a member".into()));
    }

    let details = get_poll_details(&state.db_pool, poll_id).await?;
    Ok((StatusCode::OK, Json(details)))
}

// PUT /events/:event_id/polls/:poll_id
pub async fn update_poll_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    Path((event_id, poll_id)): Path<(i64, i64)>,
    Json(payload): Json<UpdatePollRequest>,
) -> Result<impl IntoResponse, AppError> {
    let user = find_user_by_token(&state.db_pool, auth.token()).await?
        .ok_or(AppError::UserNotFound)?;
    let event = get_event_by_id(&state.db_pool, event_id).await?;

    if !is_user_in_event(&state.db_pool, user.user_id, event_id).await? {
        return Err(AppError::UserNotInEvent("Not a member".into()));
    }
    check_user_permissions(&state.db_pool, &event, &user, EventPermissions::UPDATE_MODULE).await?;

    edit_pool_question(&state.db_pool, poll_id, payload.question).await?;

    Ok((StatusCode::NO_CONTENT, Json(json!({ "success": true }))))
}

// DELETE /events/:event_id/polls/:poll_id
pub async fn delete_poll_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    Path((event_id, poll_id)): Path<(i64, i64)>,
) -> Result<impl IntoResponse, AppError> {
    let user = find_user_by_token(&state.db_pool, auth.token()).await?
        .ok_or(AppError::UserNotFound)?;
    let event = get_event_by_id(&state.db_pool, event_id).await?;

    if !is_user_in_event(&state.db_pool, user.user_id, event_id).await? {
        return Err(AppError::UserNotInEvent("Not a member".into()));
    }
    check_user_permissions(&state.db_pool, &event, &user, EventPermissions::DELETE_MODULE).await?;

    delete_poll(&state.db_pool, poll_id).await?;
    Ok((StatusCode::NO_CONTENT, Json(json!({ "success": true }))))
}

// POST /events/:event_id/polls/:poll_id/vote
pub async fn vote_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    Path((event_id, poll_id)): Path<(i64, i64)>,
    Json(payload): Json<VoteRequest>,
) -> Result<impl IntoResponse, AppError> {
    let user = find_user_by_token(&state.db_pool, auth.token()).await?
        .ok_or(AppError::UserNotFound)?;

    if !is_user_in_event(&state.db_pool, user.user_id, event_id).await? {
        return Err(AppError::UserNotInEvent("Not a member".into()));
    }

    let max_options = get_count_of_options(&state.db_pool, poll_id).await?;
    if payload.option_ids.len() as i64 > max_options {
        return Err(AppError::BadRequest("Too many options selected".into()));
    }

    vote_on_poll(&state.db_pool, poll_id, user.user_id, payload.option_ids).await?;
    Ok((StatusCode::NO_CONTENT, Json(json!({ "success": true }))))
}
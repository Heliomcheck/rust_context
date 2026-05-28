use axum::{
    response::IntoResponse,
    extract::Query,
    extract::Path,
    extract::State,
    Json,
    http::StatusCode,
};
use std::{
    sync::Arc,
    result::Result
};
use serde_json::json;
use axum_extra::TypedHeader;
use headers::{
    Authorization, 
    authorization::Bearer
};
use crate::{
    data_base::{
        event_db::*, 
        plainning_modules::poll_db::*
    }, 
    errors::AppError, 
    handlers::user::get_user_for_handler_from_token, 
    models::*, 
    permissions::*, 
    structs::*
};



#[utoipa::path(
    post,
    path = "/events/{event_id}/planning/poll",
    tag = "Modules",
    security(
        ("bearerAuth" = [])
    ),
    request_body = CreatePollRequest,
    responses(
        (status = 201, description = "Poll created", body = PollResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 403, description = "User doesn't have permission to invite or not in event", body = ErrorResponse),
        (status = 404, description = "User or event not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn create_poll_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    Path(event_id): Path<i64>,
    Json(payload): Json<CreatePollRequest>,
) -> Result<impl IntoResponse, AppError> {
    let user = get_user_for_handler_from_token(&state.db_pool, &auth.token()).await?;

    let event = get_event_by_id(&state.db_pool, event_id).await?;

    let is_member = check_user_in_event(&state.db_pool, event.event_id, user.user_id).await?;
    if !is_member {
        return Err(AppError::UserNotInEvent("User not in event".to_string()));
    }
    match check_user_permissions(&state.db_pool, &event, &user, EventPermissions::OWNER).await {
        Ok(true) => {},
        Ok(false) => return Err(AppError::UserNotInEvent("User doesn't have permission to update permissions".to_string())),
        Err(e) => return Err(e),
    };

    let poll_id = create_poll(
        &state.db_pool,
        event_id,
        payload.title,
        user.user_id,
        payload.options,
        payload.multiple_choice
    ).await?;

    let poll = get_poll_by_id(&state.db_pool, poll_id).await?;

    Ok((StatusCode::CREATED, Json(json!({"success": true }))))
}

#[utoipa::path(
    put,
    path = "/events/{event_id}/planning/poll/{poll_id}",
    tag = "Modules",
    security(
        ("bearerAuth" = [])
    ),
    request_body = UpdatePollRequest,
    responses(
        (status = 204, description = "Poll updated", body = SuccessResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 403, description = "User doesn't have permission to invite or not in event", body = ErrorResponse),
        (status = 404, description = "User or event not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn update_poll_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    query: Query<EventModule>,
    Json(payload): Json<UpdatePollRequest>
) -> Result<impl IntoResponse, AppError> {
    let user = get_user_for_handler_from_token(&state.db_pool, &auth.token()).await?;

    let event = get_event_by_id(&state.db_pool, query.event_id).await?;

    let is_member = check_user_in_event(&state.db_pool, event.event_id, user.user_id).await?;

    if !is_member {
        return Err(AppError::UserNotInEvent("User not in event".to_string()));
    }

    match check_user_permissions(&state.db_pool, &event, &user, EventPermissions::OWNER).await {
        Ok(true) => {},
        Ok(false) => return Err(AppError::UserNotInEvent("User doesn't have permission to update permissions".to_string())),
        Err(e) => return Err(e),
    };

    let updated = edit_pool_question(&state.db_pool, query.module_id, payload.question).await?;
    if !updated {
        return Err(AppError::BadRequest("Poll not found".to_string()));
    }
    Ok((StatusCode::NO_CONTENT, Json(json!({"success": true}))))
}

#[utoipa::path(
    delete,
    path = "/events/{event_id}/planning/poll/{poll_id}",
    tag = "Modules",
    security(
        ("bearerAuth" = [])
    ),
    request_body = DeletePollRequest,
    responses(
        (status = 204, description = "Poll deleted", body = SuccessResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 403, description = "User doesn't have permission to invite or not in event", body = ErrorResponse),
        (status = 404, description = "User or event not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn delete_poll_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    query: Query<EventModule>,
    //Json(payload): Json<DeletePollRequest>,
) -> Result<impl IntoResponse, AppError> {
    let user = get_user_for_handler_from_token(&state.db_pool, &auth.token()).await?;

    let event = get_event_by_id(&state.db_pool, query.event_id).await?;

    let is_member = check_user_in_event(&state.db_pool, event.event_id, user.user_id).await?;
    if !is_member {
        return Err(AppError::UserNotInEvent("User not in event".to_string()));
    }
    let _ = match check_user_permissions(&state.db_pool, &event, &user, EventPermissions::OWNER).await {
        Ok(true) => true,
        Ok(false) => return Err(AppError::UserNotInEvent("User doesn't have permission to update permissions".to_string())),
        Err(e) => return Err(e),
    };

    let deleted = delete_poll(&state.db_pool, query.event_id).await?;
    if !deleted {
        return Err(AppError::BadRequest("Poll not found".to_string()));
    }
    Ok((StatusCode::NO_CONTENT, Json(json!({"success": true}))))
}

#[utoipa::path(
    post,
    path = "/events/{event_id}/planning/poll/{poll_id}/vote",
    tag = "Modules",
    security(
        ("bearerAuth" = [])
    ),
    request_body = VotePollRequest,
    responses(
        (status = 204, description = "Poll voted successfully", body = SuccessResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 403, description = "User doesn't have permission to invite or not in event", body = ErrorResponse),
        (status = 404, description = "User or event not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn vote_poll_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    query: Query<EventModule>,
    Json(payload): Json<VotePollRequest>,
) -> Result<impl IntoResponse, AppError> {
    let user = get_user_for_handler_from_token(&state.db_pool, &auth.token()).await?;

    let event = get_event_by_id(&state.db_pool, query.event_id).await?;

    let is_member = check_user_in_event(&state.db_pool, event.event_id, user.user_id).await?;
    if !is_member {
        return Err(AppError::UserNotInEvent("User not in event".to_string()));
    }

    match vote_on_poll(&state.db_pool, query.module_id, user.user_id, payload.option_indexes).await {
        Ok(true) => {},
        Ok(false) => return Err(AppError::BadRequest("Poll or options not found".to_string())),
        Err(e) => return Err(AppError::Internal(format!("Failed to vote on poll: {}", e))),
    };

    Ok((StatusCode::NO_CONTENT, Json(json!({"success": true}))))
}
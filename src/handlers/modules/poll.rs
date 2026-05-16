use axum::response::IntoResponse;
use axum::extract::State;
use std::sync::Arc;
use axum::Json;
use axum::http::StatusCode;
use serde_json::json;
use std::result::Result;
use axum_extra::TypedHeader;
use headers::{Authorization, authorization::Bearer};
use crate::{data_base::user_db::*};

use crate::structs::*;

use crate::{
    models::*,
    errors::AppError,
    data_base::{
        event_db::*,
        plainning_modules::poll_db::*,
    },
    permissions::*,
};


#[utoipa::path(
    post,
    path = "/modules/poll/create_poll",
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
    Json(payload): Json<CreatePollRequest>,
) -> Result<impl IntoResponse, AppError> {
    let user = match find_user_by_token(&state.db_pool, auth.token()).await? {
        Some(u) => u,
        None => return Err(AppError::UserNotFound),
    };
    let event = get_event_by_id(&state.db_pool, payload.event_id).await?;

    let is_member = check_user_in_event(&state.db_pool, event.event_id, user.user_id).await?;
    if !is_member {
        return Err(AppError::UserNotInEvent("User not in event".to_string()));
    }
    let _ = match check_user_permissions(&state.db_pool, &event, &user, EventPermissions::CREATE_MODULE).await {
        Ok(true) => true,
        Ok(false) => return Err(AppError::UserNotInEvent("User doesn't have permission to update permissions".to_string())),
        Err(e) => return Err(e),
    };
    // let max_allowed = get_count_of_options(&state.db_pool, payload.event_id).await?;

    // if (payload.options.len() as i64) > max_allowed {
    //     return Err(AppError::BadRequest("To many options".to_string()));
    // }

    let poll_id = create_poll(
        &state.db_pool,
        payload.event_id,
        payload.question,
        user.user_id,
        payload.options,
        payload.more_than_one_vote
    ).await?;


    Ok((StatusCode::CREATED, Json(json!({"poll_id": PollResponse { poll_id }}))))
}

#[utoipa::path(
    post,
    path = "/modules/poll/update_poll",
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
    Json(payload): Json<UpdatePollRequest>,
) -> Result<impl IntoResponse, AppError> {
    let user = match find_user_by_token(&state.db_pool, auth.token()).await? {
        Some(u) => u,
        None => return Err(AppError::UserNotFound),
    };
    let event = get_event_by_id(&state.db_pool, payload.event_id).await?;

    let is_member = check_user_in_event(&state.db_pool, event.event_id, user.user_id).await?;
    if !is_member {
        return Err(AppError::UserNotInEvent("User not in event".to_string()));
    }
    let _ = match check_user_permissions(&state.db_pool, &event, &user, EventPermissions::UPDATE_MODULE).await {
        Ok(true) => true,
        Ok(false) => return Err(AppError::UserNotInEvent("User doesn't have permission to update permissions".to_string())),
        Err(e) => return Err(e),
    };

    let updated = edit_pool_question(&state.db_pool, payload.poll_id, payload.question).await?;
    if !updated {
        return Err(AppError::BadRequest("Poll not found".to_string()));
    }
    Ok((StatusCode::NO_CONTENT, Json(json!({"success": true}))))
}

#[utoipa::path(
    post,
    path = "/modules/poll/delete_poll",
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
    Json(payload): Json<DeletePollRequest>,
) -> Result<impl IntoResponse, AppError> {
    let user = match find_user_by_token(&state.db_pool, auth.token()).await? {
        Some(u) => u,
        None => return Err(AppError::UserNotFound),
    };
    let event = get_event_by_id(&state.db_pool, payload.event_id).await?;

    let is_member = check_user_in_event(&state.db_pool, event.event_id, user.user_id).await?;
    if !is_member {
        return Err(AppError::UserNotInEvent("User not in event".to_string()));
    }
    let _ = match check_user_permissions(&state.db_pool, &event, &user, EventPermissions::UPDATE_MODULE).await {
        Ok(true) => true,
        Ok(false) => return Err(AppError::UserNotInEvent("User doesn't have permission to update permissions".to_string())),
        Err(e) => return Err(e),
    };

    let deleted = delete_poll(&state.db_pool, payload.event_id).await?;
    if !deleted {
        return Err(AppError::BadRequest("Poll not found".to_string()));
    }
    Ok((StatusCode::NO_CONTENT, Json(json!({"success": true}))))
}

#[utoipa::path(
    post,
    path = "/modules/poll/vote_poll",
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
    Json(payload): Json<VotePollRequest>,
) -> Result<impl IntoResponse, AppError> {

    Ok((StatusCode::NO_CONTENT, Json(json!({"success": true}))))
}
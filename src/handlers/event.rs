use axum_macros::debug_handler;
use chrono::{DateTime, Utc};
use tokio::{net::TcpListener, sync::broadcast};
use axum::{Router, extract::ws::{WebSocket, WebSocketUpgrade}, response::IntoResponse, routing::{self, trace}
        };
use axum::extract::State;
use std::sync::Arc;
use axum::Json;
use axum::http::StatusCode;
use serde_json::json;
use std::result::Result;
use axum_extra::TypedHeader;
use headers::{Authorization, authorization::Bearer};
use crate::{data_base::user_db::*, secrets::{generator, token}};
use tracing::*;

use crate::{data_base::user_db::{create_token, create_user_db, validate_token}, models::CheckUsernameRequest, secrets::token::*, structs::*};
use crate::mail::send_mail_verif_code;

use crate::{
    models::*,
    errors::AppError,
    data_base::{
        event_db::*,
        plainning_modules::poll_db::*,
    },
    permissions::*,
};

use crate::test_utils::*;

// Event handlers

#[utoipa::path(
    post,
    path = "/events",
    tag = "Event",
    security(
        ("bearerAuth" = [])
    ),
    request_body = CreateEventRequest,
    responses(
        (status = 201, description = "Event created", body = CreateEventResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 404, description = "User or event not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn create_event_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    Json(payload): Json<CreateEventRequest>,
) -> Result<impl IntoResponse, AppError> {
    let user = find_user_by_token(&state.db_pool, auth.token()).await?
        .ok_or(AppError::UserNotFound)?;

    let start_date = match payload.start_date_time {
        Some(ref s) => {
            let dt = s.parse::<DateTime<Utc>>().map_err(|_| AppError::BadRequest("Invalid start date format. Use ISO 8601".to_string()))?;
            Some(dt)
        }
        None => None,
    };
    let end_date = match payload.end_date_time{
        Some(ref s) => {
            let dt = s.parse::<DateTime<Utc>>().map_err(|_| AppError::BadRequest("Invalid end date format. Use ISO 8601".to_string()))?;
            Some(dt)
        }
        None => None,
    };

    let event_id = create_event(
        &state.db_pool,
        &payload.title,
        payload.description.as_deref(),
        start_date,
        end_date,
        payload.color, 
    ).await?;

    let _ = add_member(&state.db_pool, user.user_id, event_id, EventPermissions::OWNER, 2).await?;

    let event = get_event_by_id(&state.db_pool, event_id).await?;
    
    let created_by = check_user_permissions(&state.db_pool, &event, &user, EventPermissions::OWNER).await?;

    let event_response = CreateEventResponse {
        id: event.event_id.to_string(),
        title: event.event_name,
        description: event.description_event,
        location: Some("test".to_string()),
        start_date_time: event.start_date.map(|dt| dt.to_rfc3339()),
        end_date_time: event.end_date.map(|dt| dt.to_rfc3339()),
        color: event.color,
        created_by: created_by.to_string(), 
        created_at: event.created_at.to_rfc3339(),
        status: event.is_active.to_string()
    };

    Ok((StatusCode::CREATED, Json(event_response)))
}

#[utoipa::path(
    get,
    path = "/events_detailed",
    tag = "Event",
    security(
        ("bearerAuth" = [])
    ),
    responses(
        (status = 200, description = "Event details retrieved", body = GetEventsResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 404, description = "User or event not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn get_user_events_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>
) -> Result<impl IntoResponse, AppError> {
    let user = match find_user_by_token(&state.db_pool, auth.token()).await? {
        Some(u) => u,
        None => return Err(AppError::UserNotFound),
    };

    let events = get_user_events(&state.db_pool, user.user_id, 10, 0).await?;

    Ok((StatusCode::OK, Json(json!({"events": events}))))
}

#[utoipa::path(
    get,
    path = "/events_detailed",
    tag = "Event",
    security(
        ("bearerAuth" = [])
    ),
    request_body = GetEventRequest,
    responses(
        (status = 200, description = "Event details retrieved", body = GetEventDetailedResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 404, description = "User or event not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn get_detailed_event_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    Json(payload): Json<GetEventRequest>,
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

    let invite_url = "invite_url".to_string();// create_event_token(&state.db_pool, event.event_id).await?;

    let members = get_users_in_event(&state.db_pool, event.event_id).await?;

    // let _ = match check_user_permissions(&state.db_pool, &event, &user, EventPermissions::MEMBER).await {
    //     Ok(true) => true,
    //     Ok(false) => return Err(AppError::UserNotInEvent("User doesn't have permission to invite".to_string())),
    //     Err(e) => return Err(e),
    // }; refactor

    let permissions = get_user_permissions(&state.db_pool, event.event_id, user.user_id).await?;
    Ok((StatusCode::OK, Json(json!(GetEventDetailedResponse {
        event: event, 
        invite_url: Some(invite_url), 
        members: members,
        permissions: permissions.get_bits().to_string()
    }))))
}

// pub async fn get_event_modules_handler(

//     State(state): State<Arc<AppState>>,
//     auth: TypedHeader<Authorization<Bearer>>,
//     Json(payload): Json<InviteUserToEventRequest>,
// ) -> Result<impl IntoResponse, AppError> {
//     let user = match find_user_by_token(&state.db_pool, auth.token()).await? {
//         Some(u) => u,
//         None => return Err(AppError::UserNotFound),
//     };
//     let event = get_event_by_id(&state.db_pool, payload.event_id).await?;

//     let is_member = check_user_in_event(&state.db_pool, event.event_id, user.user_id).await?;
//     if !is_member {
//         return Err(AppError::UserNotInEvent("User not in event".to_string()));
//     }

//     let _ = match check_user_permissions(&state.db_pool, &event, &user, EventPermissions::INVITE).await {
//         Ok(true) => true,
//         Ok(false) => return Err(AppError::UserNotInEvent("User doesn't have permission to invite".to_string())),
//         Err(e) => return Err(e),
//     };
//     let modules = get_event_modules(&state.db_pool, event.event_id).await?;

//     Ok((StatusCode::OK, Json(json!({"modules": event.modules}))))
// }

#[utoipa::path(
    post,
    path = "/events/add_user",
    tag = "Event",
    security(
        ("bearerAuth" = [])
    ),
    request_body = InviteUserToEventRequest,
    responses(
        (status = 204, description = "User added to event", body = SuccessResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 403, description = "User doesn't have permission to invite or not in event", body = ErrorResponse),
        (status = 404, description = "User or event not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn add_user_to_event_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    Json(payload): Json<InviteUserToEventRequest>,
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

    let _ = match check_user_permissions(&state.db_pool, &event, &user, EventPermissions::INVITE).await {
        Ok(true) => true,
        Ok(false) => return Err(AppError::UserNotInEvent("User doesn't have permission to invite".to_string())),
        Err(e) => return Err(e),
    };
    let _ = add_member(&state.db_pool, user.user_id, event.event_id, payload.permissions, 2).await?;

    Ok((StatusCode::NO_CONTENT, Json(json!({"success": true}))))
}

#[utoipa::path(
    post,
    path = "/events/delete_user",
    tag = "Event",
    security(
        ("bearerAuth" = [])
    ),
    request_body = GetEventRequest,
    responses(
        (status = 204, description = "User deleted from event", body = SuccessResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 403, description = "User doesn't have permission to invite or not in event", body = ErrorResponse),
        (status = 404, description = "User or event not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn delete_user_from_event_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    Json(payload): Json<GetEventRequest>,
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

    let _ = match check_user_permissions(&state.db_pool, &event, &user, EventPermissions::DELETE_MEMBER).await {
        Ok(true) => true,
        Ok(false) => return Err(AppError::UserNotInEvent("User doesn't have permission to delete member".to_string())),
        Err(e) => return Err(e),
    };
    // chech user for deleting
    let user_id_for_deleting = match find_user_by_token(&state.db_pool, auth.token()).await? {
        Some(u) => u,
        None => return Err(AppError::UserNotFound),
    };

    let is_member = check_user_in_event(&state.db_pool, event.event_id, user_id_for_deleting.user_id).await?;
    if !is_member {
        return Err(AppError::UserNotInEvent("User not in event".to_string()));
    }
    let _ = remove_member(&state.db_pool, user_id_for_deleting.user_id, event.event_id).await?;

    Ok((StatusCode::NO_CONTENT, Json(json!({"success": true}))))
}

#[utoipa::path(
    post,
    path = "/events/permissions",
    tag = "Event",
    security(
        ("bearerAuth" = [])
    ),
    request_body = UpdateUserPermissionsRequest,
    responses(
        (status = 204, description = "Updated user permissions", body = SuccessResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 403, description = "User doesn't have permission to invite or not in event", body = ErrorResponse),
        (status = 404, description = "User or event not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn update_user_permissions_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    Json(payload): Json<UpdateUserPermissionsRequest>,
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
    let _ = match check_user_permissions(&state.db_pool, &event, &user, EventPermissions::UPDATE_PERMISSIONS).await {
        Ok(true) => true,
        Ok(false) => return Err(AppError::UserNotInEvent("User doesn't have permission to update permissions".to_string())),
        Err(e) => return Err(e),
    };

    let _ = update_user_permissions(&state.db_pool, event.event_id, user.user_id, payload.new_permissions).await?;

    Ok((StatusCode::NO_CONTENT, Json(json!({"success": true}))))
}

#[utoipa::path(
    post,
    path = "/events/join",
    tag = "Event",
    security(
        ("bearerAuth" = [])
    ),
    request_body = JoinEventRequest,
    responses(
        (status = 204, description = "Join in event successfully", body = SuccessResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 403, description = "User doesn't have permission to invite or not in event", body = ErrorResponse),
        (status = 404, description = "User or event not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn event_join_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    Json(payload): Json<JoinEventRequest>,
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
    let _ = match check_user_permissions(&state.db_pool, &event, &user, EventPermissions::UPDATE_PERMISSIONS).await {
        Ok(true) => true,
        Ok(false) => return Err(AppError::UserNotInEvent("User doesn't have permission to update permissions".to_string())),
        Err(e) => return Err(e),
    };


    Ok((StatusCode::NO_CONTENT, Json(json!({"success": true}))))
}

// Plainning module handlers


pub async fn get_event_polls_handler(
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
    let _ = match check_user_permissions(&state.db_pool, &event, &user, EventPermissions::UPDATE_MODULE).await {
        Ok(true) => true,
        Ok(false) => return Err(AppError::UserNotInEvent("User doesn't have permission to update permissions".to_string())),
        Err(e) => return Err(e),
    };
    let polls = get_event_polls(&state.db_pool, event.event_id).await?;

    Ok((StatusCode::CREATED, Json(json!({"polls": polls}))))
}

// #[debug_handler]
// pub async fn test_handler(
//     State(state): State<Arc<AppState>>,
//     auth: TypedHeader<Authorization<Bearer>>,
//     Json(payload): Json<CreateEventRequest>,
// ) -> Result<impl IntoResponse, AppError> {
//     Ok((StatusCode::CREATED, Json(json!({"status": "ok"}))))
// }
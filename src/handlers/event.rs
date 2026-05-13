use axum_macros::debug_handler;
use chrono::{DateTime, Utc};
use tokio::{net::TcpListener, sync::broadcast};
use axum::{Router, extract::ws::{WebSocket, WebSocketUpgrade}, response::IntoResponse, routing::{self, trace}
        };
use axum::extract::State;
use std::sync::Arc;
use tokio::sync::Mutex;
use axum::Json;
use validator::Validate;
use axum::http::StatusCode;
use serde_json::json;
use axum::body::Body;
use axum::http::Request;
use tower::util::ServiceExt;
use std::collections::HashMap;
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
    data_base::event_db::*,
    plainning_modules::poll::*,
    permissions::*,
};

use crate::test_utils::*;

// Event handlers

pub async fn create_event_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    Json(payload): Json<CreateEventRequest>,
) -> Result<impl IntoResponse, AppError> {
    let user = find_user_by_token(&state.db_pool, auth.token()).await?
        .ok_or(AppError::UserNotFound)?;

    let start_date = match payload.startDateTime {
        Some(ref s) => {
            let dt = s.parse::<DateTime<Utc>>().map_err(|_| AppError::BadRequest("Invalid start date format. Use ISO 8601".to_string()))?;
            Some(dt)
        }
        None => None,
    };
    let end_date = match payload.endDateTime {
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

    let _ = add_member(&state.db_pool, user.user_id, event_id, 1, 2).await?;

    let event = get_event_by_id(&state.db_pool, event_id).await?;
    
    let created_by = get_event_owner_id(&state.db_pool, event_id).await?;

    let event_response = CreateEventResponse {
        id: event.event_id.to_string(),
        title: event.event_name,
        description: event.description_event,
        location: Some("test".to_string()),
        startDateTime: event.start_date.map(|dt| dt.to_rfc3339()),
        endDateTime: event.end_date.map(|dt| dt.to_rfc3339()),
        color: event.color,
        createdBy: created_by.to_string(), 
        createdAt: event.created_at.to_rfc3339(),
        status: event.is_active.to_string()
    };

    Ok((StatusCode::CREATED, Json(event_response)))
}

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

pub async fn get_detailed_event_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    Json(payload): Json<GetEventResponse>,
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

    let users = get_users_in_event(&state.db_pool, event.event_id).await?;

    let user_permissions = check_user_permissions(&state.db_pool, &event, &user, 0).await?;// number is a index of permissions

    Ok((StatusCode::OK, Json(json!({
        "event": event, 
        "invite_url": invite_url, 
        "users": users, 
        "user_permission": user_permissions
    }))))
}

// Plainning module handlers

pub async fn create_poll_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    Json(payload): Json<CreatePollRequest>,
) -> Result<impl IntoResponse, AppError> {
    let user = match find_user_by_token(&state.db_pool, auth.token()).await? {
        Some(u) => u,
        None => return Err(AppError::UserNotFound),
    };

    let _ = get_event_by_id(&state.db_pool, payload.event_id).await?;
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


    Ok((StatusCode::CREATED, Json(json!({"poll_id": poll_id}))))
}

pub async fn get_event_polls_handler(
  State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    Json(payload): Json<CreatePollRequest>,
) -> Result<impl IntoResponse, AppError> {

    let _ = match find_user_by_token(&state.db_pool, auth.token()).await? {
        Some(u) => u,
        None => return Err(AppError::UserNotFound),
    };

    let event = get_event_by_id(&state.db_pool, payload.event_id).await?;

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
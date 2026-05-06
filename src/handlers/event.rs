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
use sqlx::PgPool;
use crate::{data_base::user_db::*, secrets::generator};
use tracing::*;

use crate::{data_base::user_db::{create_token, create_user_db, validate_token}, models::CheckUsernameRequest, secrets::token::*, structs::*};
use crate::mail::send_mail_verif_code;

use crate::{
    models::*,
    user_store::*,
    secrets::verification::VerificationStore,
    errors::AppError,
    data_base::event_db::*,
    data_base::user_db::*,
};

use std::str::*;

use chrono::format::ParseError;

use crate::test_utils::*;

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

#[debug_handler]
pub async fn test_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    Json(payload): Json<CreateEventRequest>,
) -> Result<impl IntoResponse, AppError> {
    Ok((StatusCode::CREATED, Json(json!({"status": "ok"}))))
}
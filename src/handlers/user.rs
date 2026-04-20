use futures_util::future::ok;
use tokio::{net::TcpListener, sync::broadcast};
use anyhow::{Context}; 
use axum::{Router, extract::ws::{WebSocket, WebSocketUpgrade}, response::IntoResponse, routing::{self, trace}
        };
use axum_macros::debug_handler;
use axum::extract::State;
use std::sync::Arc;
use tokio::sync::Mutex;
use axum::Json;
use validator::Validate;
use axum::http::StatusCode;
use serde_json::json;
//use axum_auth::{Json, TypedHeader, headers::{Authorization, authorization::Bearer}};
use axum_extra::TypedHeader;
use headers::{Authorization, authorization::Bearer};
use sqlx::PgPool;
use tracing::*;

use crate::{data_base::user_db::find_user_by_token, models::CheckUsernameRequest, secrets::token::{self, TokenStore}, structs::*};

use crate::{
    models::*,
    user::*,
    secrets::verification::VerificationStore,
    data_base::user_db::*
};

pub async fn user_edit_handler(
        auth: TypedHeader<Authorization<Bearer>>,
        State(state): State<Arc<AppState>>,
        Json(payload): Json<EditUserRequest>,
    ) -> impl IntoResponse {
    if let Err(errors) = payload.validate() {
        return validation_errors_to_response(errors);
    }

    let token = auth.token();
    
    let user = match find_user_by_token(&state.db_pool, token).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            return (StatusCode::UNAUTHORIZED, Json(json!({ "success": false, "error": "User not found" })));
        }
        Err(e) => {
            tracing::error!("Token validation error: {}", e);
            return (StatusCode::UNAUTHORIZED, Json(json!({ "success": false, "error": "Invalid session" })));
        }
    };
    
    if let Err(e) = edit_user_db(
        &state.db_pool,
        user.id,
        payload.username.as_deref(),
        payload.email.as_deref(),
        payload.display_name.as_deref(),
        payload.birthday.as_deref(),
        payload.avatar_url.as_deref()
    ).await {
        tracing::error!("Failed to update user: {}", e);
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "error": "Failed to update user" })));
    }
    // update user data un UserStore in future
    
    (StatusCode::OK, Json(json!({ "success": true })))
}

pub async fn get_user_data_handler(
    auth: TypedHeader<Authorization<Bearer>>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let token = auth.token();
    
    // 1. Находим пользователя по токену
    let user = match find_user_by_token(&state.db_pool, token).await {
        Ok(Some(u)) => u,
        Ok(None) => {
            return (StatusCode::UNAUTHORIZED, Json(json!({ "error": "Invalid or expired token" })));
        }
        Err(e) => {
            error!("Failed to find user by token: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Database error" })));
        }
    };
    
    // 2. Формируем ответ
    let response = UserDataResponse {
        id: user.id,
        username: user.username.clone(),  // ← clone()
        email: user.email.clone(),        // ← clone()
        name: user.name.clone(),          // ← clone()
        birthday: user.birthday.clone(),
        avatar_url: user.avatar_url.clone()
    };
    
    (StatusCode::OK, Json(json!({
        //"id": user.id,
        "username": user.username,
        "email": user.email,
        "display_name": user.name,
        "birthday": user.birthday,
        //"avatar_url": user.avatar_url,
        //"created_at": user.created_at,
    })))
}
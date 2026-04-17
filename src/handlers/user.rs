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

use crate::{models::CheckUsernameRequest, structs::*, secrets::token::{self, TokenStore}};

use crate::{
    models::*,
    user::*,
    secrets::verification::VerificationStore
};

pub async fn user_edit_handler(
        State(state): State<Arc<AppState>>,
        Json(payload): Json<EditUserRequest>,
    ) -> impl IntoResponse {
        if let Err(errors) = payload.validate() {
            return validation_errors_to_response(errors);
        }

        let token = payload.token.clone();
        let user_store = state.user_store.lock().await;
        
        let user_id = match user_store.get_session(&token) { // check session
            Some(session) => session.user_id,
            None => return (StatusCode::UNAUTHORIZED, Json(json!({"success": false, "error": "Invalid session"}))),
        };
        
        let mut user = match user_store.get_user_by_id(user_id) { // get user
            Some(u) => u.clone(),
            None => return (StatusCode::NOT_FOUND, Json(json!({"success": false, "error": "User not found"}))),
        };
        
        user.edit(payload);
        (StatusCode::OK, Json(json!({"success": true})))
}
use futures_util::future::ok;
use tokio::{net::TcpListener, sync::broadcast};
use anyhow::{Context, Result}; 
use axum::{Router, extract::ws::{WebSocket, WebSocketUpgrade}, response::IntoResponse, routing::{self, trace}
        };
use axum_macros::debug_handler;
use axum::extract::State;
use std::sync::Arc;
use tokio::sync::Mutex;
use axum::Json;
use validator::{Validate, ValidationError};
use axum::http::StatusCode;
use serde_json::json;
use std::collections::HashMap;

use crate::{structs::*, token::{self, TokenStore}};
use crate::context::*;
use crate::mail::send_mail_verif_code;

use crate::{
    models::{RegisterRequest, TokenVerifyRequest, VerifyCodeRequest, CodeRequest, validation_errors_to_response},
    user::UserStore
};

pub async fn register_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<RegisterRequest>
) -> impl IntoResponse {
    if let Err(errors) = payload.validate() {
        return validation_errors_to_response(errors);
    }
    
    if state.user_store.lock().await.check_username(&payload.username.as_str()) {
        return (StatusCode::BAD_REQUEST, Json(json!({ "error": "Username is already taken" })));
    }

    if Some(state.user_store.lock().await.get_user_by_email(&payload.email.as_str())).is_some() {
        return (StatusCode::BAD_REQUEST, Json(json!({ "error": "Email is already registered" })));
    }

    let token = TokenStore::new(30);
    let token_str = token.token.clone();
    let mut user_store = state.user_store.lock().await;
    match user_store.add_user(
        payload.username.clone(),
        payload.email.clone(),
        payload.birthday.clone(),
        payload.name.clone(),
        payload.avatar_url.clone(),
        Some(HashMap::from([(token_str.clone(), token)]))
    ) {
        Ok(_) => {
            return (StatusCode::CREATED, Json(json!({"token" : token_str})));
        }
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": format!("Failed to create user: {e}" )})));
        }
    }
}

pub async fn request_code_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CodeRequest>
) -> impl IntoResponse {
    if let Err(errors) = payload.validate() {
        return validation_errors_to_response(errors);
    }

    match send_mail_verif_code(&payload.email, state).await {
        Ok(()) =>
            (StatusCode::CREATED, Json(json!({"success": true}))),
        Err(e) => {
            print!("{e}");
            (StatusCode::INTERNAL_SERVER_ERROR, 
                Json(json!({"success": false, "error": format!("Failed to send verification code: {e}")})))
            }
    }
}

pub async fn verify_code_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<VerifyCodeRequest>
) -> impl IntoResponse {
    if let Err(errors) = payload.validate() {
        return validation_errors_to_response(errors);
    }

    if state.verification_store.lock().await.verify(&payload.email, &payload.code) {
        if let Some(_) = state.user_store.lock().await.get_user_by_email(&payload.email) {
            return (StatusCode::OK, Json(json!({ "isNewUser": false, "token" : "" })));
        } else {
            return (StatusCode::OK, Json(json!({ "isNewUser": true, "tempToken" : ""  })));
        }
    }
    (StatusCode::BAD_GATEWAY, Json(json!({ "error": "Verification failed" })))
}

pub async fn token_validate_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<TokenVerifyRequest>
) -> impl IntoResponse {
    if let Err(errors) = payload.validate() {
        return validation_errors_to_response(errors);
    }
    match state.user_store.lock().await.is_valid_token(&payload.token) {
        true => (StatusCode::OK, Json(json!({"success": true}))),
        false => (StatusCode::UNAUTHORIZED, Json(json!({"success": false, "error": format!("Token validation failed")})))
    }
}

pub async fn logout_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<TokenVerifyRequest>
) -> impl IntoResponse {
    if let Err(errors) = payload.validate() {
        return validation_errors_to_response(errors);
    }
    match state.user_store.lock().await.is_valid_token(&payload.token) {
        true => {
            let _ = state.user_store.lock().await.delete_session(&payload.token);
            return (StatusCode::OK, Json(json!({"success": true})));
        },
        false => (StatusCode::UNAUTHORIZED, Json(json!({"success": false, "error": format!("Logout failed")})))
    }
}

pub async fn username_check_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CodeRequest>
) -> impl IntoResponse {
    if let Err(errors) = payload.validate() {
        return validation_errors_to_response(errors);
    }
    let exists = state.user_store.lock().await.check_username(&payload.email.as_str());
    (StatusCode::OK, Json(json!({"available": !exists})))
}
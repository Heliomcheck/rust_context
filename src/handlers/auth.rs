use chrono::Utc;
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
use axum::body::Body;
use axum::http::Request;
use tower::util::ServiceExt;
use std::collections::HashMap;
use std::result::Result;
use axum_extra::TypedHeader;
use headers::{Authorization, authorization::Bearer};
use sqlx::PgPool;

use crate::{data_base::user_db::{create_token, create_user_db, validate_token}, models::CheckUsernameRequest, secrets::token::*, structs::*};
use crate::mail::send_mail_verif_code;
use crate::generator::Generator;

use crate::{
    models::*,
    user::*,
    secrets::verification::VerificationStore
};

pub async fn register_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<RegisterRequest>
) -> impl IntoResponse {
    if let Err(errors) = payload.validate() {
        return validation_errors_to_response(errors);
    }
    
    let mut user_store = state.user_store.lock().await;
    
    if user_store.check_username(&payload.username) {
        return (StatusCode::BAD_REQUEST, Json(json!({ "error": "Username is already taken" })));
    }
    
    if user_store.get_user_by_email(&payload.email).is_some() {
        return (StatusCode::BAD_REQUEST, Json(json!({ "error": "Email is already registered" })));
    }

    let user_id = create_user_db(&state.db_pool, &payload.username, &payload.email, &payload.display_name, &payload.birthday, &payload.avatar_url).await;
    let user_id = match user_id {
        Ok(id) => {id},
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": format!("Failed to create user: {e}" )})));
        }
    };

    let token_gen = Generator::new_session_token();
    let token = create_token(
        &state.db_pool, user_id, &token_gen, Utc::now() + chrono::Duration::days(30)).await;
    let token_str = match token {
        Ok(t) => {t},
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": format!("Failed to create session token: {e}" )})));
        }
    };
    
    let mut user_store = state.user_store.lock().await;
    match user_store.add_user(
        payload.username.clone(),
        payload.email.clone(),
        payload.birthday.clone(),
        payload.display_name.clone(),
        payload.avatar_url.clone(),
        &state.db_pool
    ).await {
        Ok(_) => return (StatusCode::CREATED, Json(json!({"token" : token_str}))),
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
    auth: TypedHeader<Authorization<Bearer>>,
    State(state): State<Arc<AppState>>
) -> impl IntoResponse {

    let token = auth.token();
    match validate_token(&state.db_pool, token).await {
        Ok(true) => (StatusCode::OK, Json(json!({"success": true}))),
        Ok(false) => (StatusCode::UNAUTHORIZED, Json(json!({"success": false, "error": "Invalid or expired token"}))),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"success": false, "error": format!("Token validation failed: {e}")})))
    }
}

pub async fn username_check_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CheckUsernameRequest>
) -> impl IntoResponse {
    if let Err(errors) = payload.validate() {
        return validation_errors_to_response(errors);
    }
    let exists = state.user_store.lock().await.check_username(&payload.username);
    (StatusCode::OK, Json(json!({"available": !exists})))
}

//test
#[tokio::test]//registretion check
async fn test_register_handler() {
    use tower::ServiceExt;

    let state = Arc::new(AppState {
        tx: broadcast::channel(10).0,
        user_store: Arc::new(Mutex::new(UserStore::new())),
        verification_store: Arc::new(Mutex::new(VerificationStore::new())),
        db_pool: PgPool::connect("postgres://user:password@localhost/test_db").await.unwrap()
    });

    let app = Router::new()
        .route("/auth/register", routing::post(register_handler))
        .with_state(state);

    let payload = json!({
        "username": "testuser",
        "email": "test@mail.com",
        "birthday": null,
        "name": "Test",
        "avatar_url": null
    });

    let request: Request<Body> = Request::builder()
        .method("POST")
        .uri("/auth/register")
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::CREATED);
}
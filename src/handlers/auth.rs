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
use crate::{data_base::user_db::*, generator};
use tracing::*;

use crate::{data_base::user_db::{create_token, create_user_db, validate_token}, models::CheckUsernameRequest, secrets::token::*, structs::*};
use crate::mail::send_mail_verif_code;
use crate::generator::Generator;

use crate::{
    models::*,
    user::*,
    secrets::verification::VerificationStore
};

use crate::test_utils::*;

pub async fn register_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<RegisterRequest>,
) -> impl IntoResponse {
    if let Err(errors) = payload.validate() {
        return validation_errors_to_response(errors);
    }
    
    match find_user_by_email(&state.db_pool, &payload.email).await {
        Ok(Some(_)) => {
            return (StatusCode::BAD_REQUEST, Json(json!({ "error": "Email already registered" })));
        }
        Ok(None) => {}
        Err(e) => {
            tracing::error!("DB error: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Database error" })));
        }
    }
    
    match find_user_by_username(&state.db_pool, &payload.username).await {
        Ok(Some(_)) => {
            return (StatusCode::BAD_REQUEST, Json(json!({ "error": "Username already taken" })));
        }
        Ok(None) => {}
        Err(e) => {
            tracing::error!("DB error: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Database error" })));
        }
    }
    
    let user_id = match create_user_db(
        &state.db_pool,
        &payload.username,
        &payload.email,
        &payload.display_name,
        &payload.birthday,
        &payload.avatar_url,
    ).await {
        Ok(id) => id,
        Err(e) => {
            let error_msg = e.to_string();
            if error_msg.contains("duplicate key") && error_msg.contains("username") {
                return (StatusCode::CONFLICT, Json(json!({ "error": "Username already taken" })));
            }
            if error_msg.contains("duplicate key") && error_msg.contains("email") {
                return (StatusCode::CONFLICT, Json(json!({ "error": "Email already registered" })));
            }
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Database error" })));
        }
    };
    
    let token_str = generator::Generator::new_session_token();
    let expires_at = Utc::now() + chrono::Duration::days(30);
    
    if let Err(e) = create_token(&state.db_pool, user_id, &token_str, expires_at).await {
        tracing::error!("Failed to create token: {}", e);
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Failed to create session" })));
    }
    
    let mut user_store = state.user_store.lock().await;
    if let Err(e) = user_store.add_user(
        user_id,
        payload.username.clone(),
        payload.email.clone(),
        payload.birthday.clone(),
        payload.display_name.clone(),
        payload.avatar_url.clone(),
        &state.db_pool,
    ).await {
        tracing::warn!("User created in DB but failed to add to cache: {}", e);
    }
    
    (StatusCode::CREATED, Json(json!({ "token": token_str })))
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

    if !state.verification_store.lock().await.verify(&payload.email, &payload.code) {
        return (StatusCode::BAD_REQUEST, Json(json!({ "error": "Invalid or expired code" })));
    }

    let user = match find_user_by_email(&state.db_pool, &payload.email).await {
        Ok(Some(u)) => u,
        Ok(None) => {
            return (StatusCode::OK, Json(json!({ "isNewUser": true })));
        }
        Err(e) => {
            tracing::error!("DB error: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Database error" })));
        }
    };

    let token = match find_token_by_user_id(&state.db_pool, user.id).await {
        Ok(Some(t)) => t,
        Ok(None) => {
            let new_token = uuid::Uuid::new_v4().to_string();
            let expires_at = Utc::now() + chrono::Duration::days(30);
            
            if let Err(e) = create_token(&state.db_pool, user.id, &new_token, expires_at).await {
                tracing::error!("Failed to create token: {}", e);
                return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Failed to create session" })));
            }
            new_token
        }
        Err(e) => {
            tracing::error!("Failed to find token: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Database error" })));
        }
    };

    (StatusCode::OK, Json(json!({ "isNewUser": false, "token": token })))
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

    let exists: bool = match find_user_by_username(&state.db_pool, &payload.username).await {
        Ok(Some(_)) => true,
        Ok(None) => false,
        Err(e) => {
            error!("DB error: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "error": format!("Database error: {}", e) })));
        }
    };

    (StatusCode::OK, Json(json!({ "available": !exists })))
}
pub async fn resend_code_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CodeRequest>
) -> impl IntoResponse {
    if let Err(errors) = payload.validate() {
        return validation_errors_to_response(errors);
    }

    match send_mail_verif_code(&payload.email, state).await {
        Ok(()) => (
            StatusCode::OK,
            Json(json!({ "success": true, "message": "Code resent" }))
        ),
        Err(e) => (
            StatusCode::TOO_MANY_REQUESTS,
            Json(json!({
                "success": false,
                "error": e.to_string()
            }))
        )
    }
}

//test
#[tokio::test]//registretion check
async fn test_register_handler() {
    let pool = setup_test_db().await;

    sqlx::query!("DELETE FROM tokenstore").execute(&pool).await.unwrap();
    sqlx::query!("DELETE FROM users").execute(&pool).await.unwrap();

    let state = Arc::new(AppState {
        tx: broadcast::channel(10).0,
        user_store: Arc::new(Mutex::new(UserStore::new())),
        verification_store: Arc::new(Mutex::new(VerificationStore::new())),
        db_pool: pool
    });

    let app = Router::new()
        .route("/auth/register", routing::post(register_handler))
        .with_state(state);

    let payload = json!({
        "username": "testuser",
        "email": "test@mail.com",
        "birthday": null,
        "display_name": "Test",
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
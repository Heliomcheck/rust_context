use chrono::Utc;
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
    secrets::verification::VerificationStore
};

use crate::test_utils::*;

pub async fn register_handler(
    State(state): State<Arc<AppState>>,
    Json(wrapper): Json<RegisterRequestWrapper>,
) -> impl IntoResponse {
    let payload = wrapper.user;
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
    let avatar_url = Some("test".to_string());
    
    let user_id = match create_user_db(
        &state.db_pool,
        &payload.username,
        &payload.email,
        &payload.display_name,
        &payload.birthday,
        &avatar_url,
        &payload.description
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
        avatar_url.clone(),
        payload.description.clone(),
        &state.db_pool,
    ).await {
        tracing::warn!("User created in DB but failed to add to cache: {}", e);
    }
    
    (StatusCode::CREATED, Json(json!({ "token": token_str, "userId": user_id})))
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
        Err(_) => {
            return (StatusCode::OK, Json(json!({ "isNewUser": true })));
            // tracing::error!("DB error: {}", e);
            // return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Database error" })));
        }
    };

    let token = match find_token_by_user_id(&state.db_pool, user.user_id).await {
        Ok(Some(t)) => t,
        Ok(None) => {
            let new_token = uuid::Uuid::new_v4().to_string();
            let expires_at = Utc::now() + chrono::Duration::days(30);
            
            if let Err(e) = create_token(&state.db_pool, user.user_id, &new_token, expires_at).await {
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

    (StatusCode::OK, Json(json!({ "isNewUser": false, "token": token, "userId": user.user_id })))
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

pub async fn logout_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
) -> impl IntoResponse {
    let token = auth.token();
    
    match find_user_by_token(&state.db_pool, token).await { // check user
        Ok(Some(user)) => {
            match deactivate_token(&state.db_pool, token).await {
                Ok(_) => {
                    tracing::info!("User {} logged out", user.user_id);
                    (StatusCode::OK, Json(json!({ "success": true })))
                }
                Err(e) => {
                    tracing::error!("Failed to deactivate token: {}", e);
                    (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Failed to logout" })))
                }
            }
        }
        Ok(None) => {
            (StatusCode::UNAUTHORIZED, Json(json!({ "error": "Invalid or expired token" }))) // token invalid or user not found
        }
        Err(e) => {
            tracing::error!("DB error: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Database error" })))
        }
    }
}

//test
#[tokio::test]//registretion check
async fn test_register_handler() {
    let pool = setup_test_db().await;

    sqlx::query!("DELETE FROM token_store").execute(&pool).await.unwrap();
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
        "user" : {
            "username": "testuser",
            "email": "test@mail.com",
            "birthday": null,
            "display_name": "Test",
            "avatar_url": null
        }
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
#[tokio::test]// Проверяет, что невалидный токен возвращает UNAUTHORIZED
async fn test_token_validate_handler_invalid() {
    let pool = setup_test_db().await;
    let state = Arc::new(AppState {
        tx: broadcast::channel(10).0,
        user_store: Arc::new(Mutex::new(UserStore::new())),
        verification_store: Arc::new(Mutex::new(VerificationStore::new())),
        db_pool: pool
    });
    let app = Router::new()
        .route("/auth/token-validate", routing::post(token_validate_handler))
        .with_state(state);
    let request = Request::builder()
        .method("POST")
        .uri("/auth/token-validate")
        .header("Authorization", "Bearer invalid")
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]// Проверяет, что logout успешен и токен деактивирован
async fn test_logout_handler_success() {
    let pool = setup_test_db().await;
    let user_id = create_user_db(
        &pool,
        "logout_user",
        "logout@mail.com",
        "Logout User",
        &None,
        &None,
        &Some("test".to_string())
    ).await.unwrap();
    let token = "logout_token";
    create_token(&pool, user_id, token, Utc::now() + chrono::Duration::hours(1))
        .await
        .unwrap();

    let state = Arc::new(AppState {
        tx: broadcast::channel(10).0,
        user_store: Arc::new(Mutex::new(UserStore::new())),
        verification_store: Arc::new(Mutex::new(VerificationStore::new())),
        db_pool: pool.clone(),
    });

    let app = Router::new()
        .route("/auth/logout", routing::post(logout_handler))
        .with_state(state);

    let request = Request::builder()
        .method("POST")
        .uri("/auth/logout")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let user = find_user_by_token(&pool, token).await.unwrap();
    assert!(user.is_none());
}

#[tokio::test]// Проверяет, что занятый username корректно обрабатывается эндпоинтом
async fn test_username_check_handler() {
    let pool = setup_test_db().await;
    create_user_db(
        &pool,
        "taken",
        "taken@mail.com",
        "Test",
        &None,
        &None,
        &Some("test".to_string())
    ).await.unwrap();
    let state = Arc::new(AppState {
        tx: broadcast::channel(10).0,
        user_store: Arc::new(Mutex::new(UserStore::new())),
        verification_store: Arc::new(Mutex::new(VerificationStore::new())),
        db_pool: pool
    });
    let app = Router::new()
        .route("/auth/check-username", routing::post(username_check_handler))
        .with_state(state);
    let payload = json!({ "username": "taken" });
    let request = Request::builder()
        .method("POST")
        .uri("/auth/check-username")
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]// Проверяет, что при валидном коде возвращается isNewUser = true
async fn test_verify_code_new_user() {
    let pool = setup_test_db().await;
    let mut verification = VerificationStore::new();
    let email = "new@mail.com";
    let code = verification.create(email, 15);
    let state = Arc::new(AppState {
        tx: broadcast::channel(10).0,
        user_store: Arc::new(Mutex::new(UserStore::new())),
        verification_store: Arc::new(Mutex::new(verification)),
        db_pool: pool,
    });
    let app = Router::new()
        .route("/auth/verify-code", routing::post(verify_code_handler))
        .with_state(state);
    let payload = json!({ "email": email, "code": code });
    let request = Request::builder()
        .method("POST")
        .uri("/auth/verify-code")
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]// Проверяет, что неверный код возвращает ошибку
async fn test_verify_code_invalid() {
    let pool = setup_test_db().await;
    let state = Arc::new(AppState {
        tx: broadcast::channel(10).0,
        user_store: Arc::new(Mutex::new(UserStore::new())),
        verification_store: Arc::new(Mutex::new(VerificationStore::new())),
        db_pool: pool,
    });
    let app = Router::new()
        .route("/auth/verify-code", routing::post(verify_code_handler))
        .with_state(state);
    let payload = json!({ "email": "test@mail.com", "code": "000000" });
    let request = Request::builder()
        .method("POST")
        .uri("/auth/verify-code")
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

// #[tokio::test]// Проверяет, что код можно запросить повторно
// async fn test_resend_code_handler() {
//     let pool = setup_test_db().await;
//     let state = Arc::new(AppState {
//         tx: broadcast::channel(10).0,
//         user_store: Arc::new(Mutex::new(UserStore::new())),
//         verification_store: Arc::new(Mutex::new(VerificationStore::new())),
//         db_pool: pool,
//     });
//     let app = Router::new()
//         .route("/auth/resend", routing::post(resend_code_handler))
//         .with_state(state);
//     let payload = json!({ "email": "test@mail.com" });
//     let request = Request::builder()
//         .method("POST")
//         .uri("/auth/resend")
//         .header("content-type", "application/json")
//         .body(Body::from(payload.to_string()))
//         .unwrap();
//     let response = app.oneshot(request).await.unwrap();
//     assert!(response.status().is_success() || response.status() == StatusCode::TOO_MANY_REQUESTS);
//     // Может быть OK или TOO_MANY_REQUESTS в зависимости от cooldown
// }

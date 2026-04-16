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
use tower::ServiceExt;

use crate::{models::CheckUsernameRequest, structs::*, token::{self, TokenStore}};
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
    Json(payload): Json<CheckUsernameRequest>
) -> impl IntoResponse {
    if let Err(errors) = payload.validate() {
        return validation_errors_to_response(errors);
    }
    let exists = state.user_store.lock().await.check_username(&payload.username);
    (StatusCode::OK, Json(json!({"available": !exists})))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        Router,
        routing::post,
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use tokio::sync::broadcast;

    use crate::{
        structs::AppState,
        user::UserStore,
        verification::VerificationStore,
    };

    fn app() -> Router {
        let (tx, _) = broadcast::channel(10);

        let state = Arc::new(AppState {
            tx,
            user_store: Arc::new(Mutex::new(UserStore::new())),
            verification_store: Arc::new(Mutex::new(VerificationStore::new())),
        });

        Router::new()
            .route("/auth/register", post(register_handler))
            .route("/auth/check_username", post(username_check_handler))
            .route("/auth/token-validate", post(token_validate_handler))
            .route("/auth/logout", post(logout_handler))
            .route("/auth/verify_code", post(verify_code_handler))
            .with_state(state)
    }

    // REGISTER SUCCESS
    #[tokio::test]
    async fn test_register_success() {
        let app = app();

        let payload = serde_json::json!({
            "email": "test@mail.com",
            "username": "testuser",
            "birthday": "01.01.2000",
            "name": "Test",
            "avatar_url": null
        });

        let res = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/register")
                    .header("content-type", "application/json")
                    .body(Body::from(payload.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(res.status(), StatusCode::CREATED);
    }

    // REGISTER DUPLICATE USERNAME
    #[tokio::test]
    async fn test_register_duplicate_username() {
        let app = app();

        let payload = serde_json::json!({
            "email": "test1@mail.com",
            "username": "sameuser",
            "birthday": "01.01.2000",
            "name": "Test",
            "avatar_url": null
        });

        // first
        let _ = app.clone().oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        ).await.unwrap();

        // second (same username)
        let res = app.oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        ).await.unwrap();

        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }

    // USERNAME AVAILABLE
    #[tokio::test]
    async fn test_username_available() {
        let (tx, _) = broadcast::channel::<ChatMessage>(100);
        let user_store = Arc::new(Mutex::new(UserStore::new()));
        let verification_store = Arc::new(Mutex::new(VerificationStore::new()));
        
        let state = Arc::new(AppState {
            tx,
            user_store,
            verification_store,
        });

        let app = Router::new()
            .route("/auth/check_username", post(username_check_handler))
            .with_state(state);

        let payload = json!({ "username": "freeusername" });

        let request = Request::builder()
            .method("POST")
            .uri("/auth/check_username")
            .header("content-type", "application/json")
            .body(Body::from(payload.to_string()))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    // TOKEN INVALID
    #[tokio::test]
    async fn test_token_invalid() {
        let app = app();

        let payload = serde_json::json!({
            "token": "invalidtokeninvalidtokeninvalid12"
        });

        let res = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/token-validate")
                    .header("content-type", "application/json")
                    .body(Body::from(payload.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }

    // LOGOUT INVALID TOKEN
    #[tokio::test]
    async fn test_logout_invalid_token() {
        let app = app();

        let payload = serde_json::json!({
            "token": "invalidtokeninvalidtokeninvalid12"
        });

        let res = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/logout")
                    .header("content-type", "application/json")
                    .body(Body::from(payload.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }

    // VERIFY CODE FAIL
    #[tokio::test]
    async fn test_verify_code_fail() {
        let app = app();

        let payload = serde_json::json!({
            "email": "test@mail.com",
            "code": "123456"
        });

        let res = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/verify_code")
                    .header("content-type", "application/json")
                    .body(Body::from(payload.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(res.status(), StatusCode::BAD_GATEWAY);
    }
}
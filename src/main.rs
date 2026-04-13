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
use axum::body::Body;
use axum::http::Request;
use tower::util::ServiceExt;

mod context;
mod structs;


pub(crate) mod mail;
pub(crate) mod user;
pub(crate) mod generator;
pub(crate) mod verification;
pub(crate) mod models;

use structs::*;
use context::*;
use mail::send_mail_verif_code;

use crate::{
    models::RegisterRequest,
    models::Verify_code, 
    user::UserStore, 
    models::validation_errors_to_response
};

async fn health_handler() -> &'static str {
    "OK"
}


#[axum_macros::debug_handler]
async fn sign_up_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<RegisterRequest>
) -> impl IntoResponse {
    if let Err(errors) = payload.validate() {
        return validation_errors_to_response(errors);
    }
    
    if state.user_store.lock().await.check_username(&payload.username.as_str()) {
        eprintln!("Username is already taken");
        return (StatusCode::BAD_REQUEST, Json(json!({ "error": "Username is already taken" })));
    }

    if Some(state.user_store.lock().await.get_user_by_email(&payload.email.as_str())).is_some() {
        eprintln!("Email is already registered");
        return (StatusCode::BAD_REQUEST, Json(json!({ "error": "Email is already registered" })));
    }
    let mut user_store = state.user_store.lock().await;
    match user_store.add_user(
        payload.username.clone(),
        payload.email.clone(),
        payload.birthday.clone(),
        payload.name.clone(),
        payload.avatar_url.clone()
    ) {
        Ok(user_id) => {
            println!("User created with ID: {}", user_id);
        }
        Err(e) => {
            eprintln!("Failed to create user: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Failed to create user" })));
        }
    }
    (StatusCode::CREATED, Json(json!({ "status": "ok" })))
}

async fn request_code_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<RegisterRequest>
) -> impl IntoResponse {
    if let Err(errors) = payload.validate() {
        return validation_errors_to_response(errors);
    }

    match send_mail_verif_code(&payload.email, state).await {
        Ok(()) =>
            (StatusCode::CREATED, Json(json!({"success": true}))),
        Err(e) => 
            (StatusCode::INTERNAL_SERVER_ERROR, 
                Json(json!({"success": false, "error": "Failed to send verification code: {e}"})))
    }
}

async fn verify_code_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<Verify_code>
) -> impl IntoResponse {
    if let Err(errors) = payload.validate() {
        return validation_errors_to_response(errors);
    }

    if state.verification_codes.lock().await.codes.contains_key(&payload.email) {
        if state.verification_codes.lock().await.codes.get(&payload.email) == Some(&payload.code.parse::<u32>().unwrap_or(0)) {
            // Code is valid, check if user exists
            let user_exists = state.user_store.lock().await.get_user_by_email(&payload.email).is_some();
            if user_exists {
                // User exists, return login response
                return (StatusCode::OK, Json(json!({ "token": "user_token", "is_new_user": false })));
            } else {
                // User does not exist, return registration response
                return (StatusCode::OK, Json(json!({ "temp_token": "temp_token", "is_new_user": true })));
            }
        } else {
            // Code is invalid
            return (StatusCode::BAD_REQUEST, Json(json!({ "error": "Invalid verification code" })));
        }
    }
    (StatusCode::OK, Json(json!({ "status": "verified" })))
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let args: Vec<String> = std::env::args().collect();

    let (tx, _rx) = broadcast::channel::<ChatMessage>(100);
    let user_store = Arc::new(Mutex::new(UserStore::new()));
    let verification_codes = Arc::new(Mutex::new(mail::VerificationCode ::new()));

    let state = Arc::new(AppState {tx, user_store, verification_codes});

    let app = Router::new()
        .route("/auth/request-code", routing::post(request_code_handler))
        .route("/auth/register", routing::post(sign_up_handler))

        .route("/chat", routing::get(websocket_handler))
        .route("/health", routing::get(health_handler)) // delete in future
        .route("/login", routing::get(sign_up_handler))
        .with_state(state);

    // POST /auth/request-code {email: "test.example.com"} -> {"success": true} or {"success":false, error: "email is invalid"}
    // POST /auth/verify-code {email: "test.example.com", code: "123456"} -> register {temp_token: "", is_new_user: true} or login {token: "", is_new_user: false} or {error: "code is invalid"}
    // POST /auth/register {user: {email: "test.example.com", display_name: "display_name", birthday: "2000-01-01", "username": "test"}, temp_token: ""} -> if data.valid -> {token: ""} else {error: "reason"}
    // POST /auth/token-validate {token: ""} -> {success: true} or {success: false, error: "reason"}
    // POST /auth/logout {} -> userstore.logout(token)
    // POST /auth/check_username {"username": "test"} -> {"available": true} or {"available": false}
    // POST /user/avatar 
    // GET /avatars/{user_id}.(jpg, png, jpeg)
    
    let listner = TcpListener::bind(args[1].as_str()).await
        .context("Can't bind to address")?;

    println!("Server was start");

    axum::serve(listner, app).await
        .context("Server is false")?;

    
    Ok(())
}

#[axum_macros::debug_handler]
async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_websocket(socket, state))
}
//test
#[tokio::test]//endpoint check(OK?)
async fn test_health_handler() {
    use axum::{Router, routing::get};
    use axum::body::Body;
    use axum::http::Request;
    use tower::util::ServiceExt;
    async fn health_handler() -> &'static str {"OK"}
    let app = Router::new().route("/health", get(health_handler));
    let request = Request::builder()
        .uri("/health")
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), axum::http::StatusCode::OK);
}
#[tokio::test]//registretion check
async fn test_sign_up_handler() {
    use tower::ServiceExt;

    let state = Arc::new(AppState {
        tx: broadcast::channel(10).0,
        user_store: Arc::new(Mutex::new(UserStore::new())),
        verification_codes: Arc::new(Mutex::new(mail::VerificationCode::new())),
    });

    let app = Router::new()
        .route("/auth/register", routing::post(sign_up_handler))
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
use tokio::{net::TcpListener, sync::broadcast};
use anyhow::{Context, Result}; 
use axum::{Router, extract::ws::{WebSocket, WebSocketUpgrade}, response::IntoResponse, routing::{self, trace}
        };
use axum_macros::debug_handler;
use axum::extract::State;
use std::sync::Arc;
use tokio::sync::Mutex;
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
pub(crate) mod token;
pub(crate) mod handlers;

use structs::*;
use context::*;

use crate::{
    models::{RegisterRequest, TokenVerifyRequest, VerifyCodeRequest, CodeRequest, validation_errors_to_response},
    user::UserStore,
    handlers::auth::*
};

async fn health_handler() -> &'static str {
    "OK"
}


#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let args: Vec<String> = std::env::args().collect();

    let (tx, _rx) = broadcast::channel::<ChatMessage>(100);
    let user_store = Arc::new(Mutex::new(UserStore::new()));
    let verification_store = Arc::new(Mutex::new(verification::VerificationStore::new()));

    let state = Arc::new(AppState {tx, user_store, verification_store});

    let app = Router::new()
        .route("/auth/request-code", routing::post(request_code_handler))
        .route("/auth/verify-code", routing::post(verify_code_handler))
        .route("/auth/register", routing::post(register_handler))
        .route("/auth/token-validate", routing::post(token_validate_handler))
        .route("/auth/logout", routing::post(logout_handler)) 
        .route("/auth/check-username", routing::post(username_check_handler))

        .route("/chat", routing::get(websocket_handler))
        .route("/health", routing::get(health_handler)) // delete in future
        //.route("/login", routing::get(sign_up_handler))
        .with_state(state);

    // OK POST /auth/request-code {email: "test.example.com"} -> {"success": true} or {"success":false, error: "reason"}
    // OK POST /auth/verify-code {email: "test.example.com", code: "123456"} -> {temp_token: "", is_new_user: true} or {token: "", is_new_user: false} or {error: "Verification failed"}
    // OK POST /auth/register {user: {email: "test.example.com", display_name: "display_name", birthday: "2000-01-01", "username": "test"}, temp_token: ""} -> if data.valid -> {token: ""} else {error: "reason"}
    // OK POST /auth/token-validate {token: ""} -> {success: true} or {success: false, error: "reason"}
    // OK POST /auth/logout {"token": ""} -> {success: true} or {success: false, error: "reason"}
    // OK POST /auth/check_username {"username": "test"} -> {"available": true} or {"available": false}
    
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
// #[tokio::test]//endpoint check(OK?)
// async fn test_health_handler() {
//     use axum::{Router, routing::get};
//     use axum::body::Body;
//     use axum::http::Request;
//     use tower::util::ServiceExt;
//     async fn health_handler() -> &'static str {"OK"}
//     let app = Router::new().route("/health", get(health_handler));
//     let request = Request::builder()
//         .uri("/health")
//         .body(Body::empty())
//         .unwrap();
//     let response = app.oneshot(request).await.unwrap();
//     assert_eq!(response.status(), axum::http::StatusCode::OK);
// }
// #[tokio::test]//registretion check
// async fn test_sign_up_handler() {
//     use tower::ServiceExt;

//     let state = Arc::new(AppState {
//         tx: broadcast::channel(10).0,
//         user_store: Arc::new(Mutex::new(UserStore::new())),
//         verification_codes: Arc::new(Mutex::new(mail::VerificationCode::new())),
//     });

//     let app = Router::new()
//         .route("/auth/register", routing::post(sign_up_handler))
//         .with_state(state);

//     let payload = json!({
//         "username": "testuser",
//         "email": "test@mail.com",
//         "birthday": null,
//         "name": "Test",
//         "avatar_url": null
//     });

//     let request: Request<Body> = Request::builder()
//         .method("POST")
//         .uri("/auth/register")
//         .header("content-type", "application/json")
//         .body(Body::from(payload.to_string()))
//         .unwrap();

//     let response = app.oneshot(request).await.unwrap();

//     assert_eq!(response.status(), axum::http::StatusCode::CREATED);
// }
use tokio::{net::TcpListener, sync::broadcast};
use anyhow::{Context, Result}; 
use axum::{Router, extract::ws::{WebSocket, WebSocketUpgrade}, response::IntoResponse, routing::{self, trace}
        };
use axum_macros::debug_handler;
use axum::extract::State;
use std::sync::Arc;
use tokio::sync::Mutex;

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
        .route("/auth/verify_code", routing::post(verify_code_handler))
        .route("/auth/register", routing::post(register_handler))
        .route("/auth/token-validate", routing::post(token_validate_handler))
        .route("/auth/logout", routing::post(logout_handler)) 
        .route("/auth/check_username", routing::post(username_check_handler))

        .route("/chat", routing::get(websocket_handler))
        .route("/health", routing::get(health_handler)) // delete in future
        //.route("/login", routing::get(sign_up_handler))
        .with_state(state);

    // OK POST /auth/request-code {email: "test.example.com"} -> {"success": true} or {"success":false, error: "email is invalid"}
    // OK POST /auth/verify-code {email: "test.example.com", code: "123456"} -> register {temp_token: "", is_new_user: true} or login {token: "", is_new_user: false} or {error: "code is invalid"}
    // OK POST /auth/register {user: {email: "test.example.com", display_name: "display_name", birthday: "2000-01-01", "username": "test"}, temp_token: ""} -> if data.valid -> {token: ""} else {error: "reason"}
    // OK POST /auth/token-validate {token: ""} -> {success: true} or {success: false, error: "reason"}
    // OK POST /auth/logout {} -> userstore.logout(token)
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
use tokio::{net::TcpListener, sync::broadcast};
use anyhow::{Context, Result}; 
use axum::{Router, extract::ws::{WebSocketUpgrade}, response::IntoResponse, routing::{self}
        };
use axum::extract::State;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing_appender::{non_blocking, rolling};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

mod context;
mod structs;


pub(crate) mod mail;
pub(crate) mod user;
pub(crate) mod generator;
pub(crate) mod secrets;
pub(crate) mod models;
pub(crate) mod handlers;
pub(crate) mod data_base;
pub(crate) mod test_utils;

use structs::*;
use context::*;

use crate::{
    user::UserStore,
    handlers::auth::*,
    handlers::user::*,
    secrets::token::TokenStore,
    secrets::verification::VerificationStore,
    data_base::user_db::create_pool
};

async fn health_handler() -> &'static str {
    "OK"
}


#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    dotenvy::dotenv().ok();
    let args: Vec<String> = std::env::args().collect();

    let database_url = std::env::var("DATABASE_URL")
        .context("DATABASE_URL not set")?;

    let file_appender = rolling::daily("logs", "app.log");
    let (non_blocking, _guard) = non_blocking(file_appender);
    
    tracing_subscriber::registry()
        .with(fmt::layer().with_writer(std::io::stdout))
        .with(fmt::layer().with_writer(non_blocking)) 
        .with(EnvFilter::from_default_env())
        .init();

    let (tx, _rx) = broadcast::channel::<ChatMessage>(100);
    let user_store = Arc::new(Mutex::new(UserStore::new()));
    let verification_store = Arc::new(Mutex::new(VerificationStore::new()));
    let db_pool = create_pool(&database_url.as_str()).await?;

    let state = Arc::new(AppState {tx, user_store, verification_store, db_pool});

    let app = Router::new()
        .route("/auth/request-code", routing::post(request_code_handler))
        .route("/auth/verify-code", routing::post(verify_code_handler))
        .route("/auth/register", routing::post(register_handler))
        .route("/auth/resend-code", routing::post(resend_code_handler))
        .route("/auth/token-validate", routing::post(token_validate_handler))
        //.route("/auth/logout", routing::post(logout_handler)) 
        .route("/auth/check-username", routing::post(username_check_handler))

        .route("/user/edit", routing::post(user_edit_handler))
        //.route("/user/stay-online", routing::post(stay_online_handler))

        .route("/chat", routing::get(websocket_handler))
        .route("/health", routing::get(health_handler)) // delete in future
        //.route("/login", routing::get(sign_up_handler))
        .with_state(state);

    // OK POST /auth/request-code {email: "test.example.com"} -> {"success": true} or {"success":false, error: "reason"}
    // OK POST /auth/verify-code {email: "test.example.com", code: "123456"} -> {is_new_user: true} or {token: "", is_new_user: false} or {error: "Verification failed"}
    // OK POST /auth/register {user: {email: "test.example.com", display_name: "display_name", birthday: "2000-01-01", "username": "test"}} -> if data.valid -> {token: ""} else {error: "reason"}
    // OK POST /auth/token-validate {token: ""} -> {success: true} or {success: false, error: "reason"}
    // OK POST /auth/logout {"token": ""} -> {success: true} or {success: false, error: "reason"}
    // OK POST /auth/check_username {"username": "test"} -> {"available": true} or {"available": false}
    // POST /user/edit {token: "", user: {email: "test.example.com", display_name: "display_name", birthday: "2000-01-01", "username": "test"}} -> if data.valid -> {success: true} else {error: "reason"}
    // GET /user/get-data {token: ""} -> {user: {email: "test.example.com", display_name: "display_name", birthday: "2000-01-01", "username": "test"}} or {error: "reason"}

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
use tokio::{net::TcpListener, sync::broadcast};
use anyhow::{Context, Result}; 
use axum::{Router, extract::ws::{WebSocketUpgrade}, response::IntoResponse, routing::{self}
        };
use axum::extract::State;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing_appender::{non_blocking, rolling};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};
use utoipa_swagger_ui::SwaggerUi;
use utoipa::OpenApi;

mod context;


pub(crate) mod mail;
pub(crate) mod user_store;
pub(crate) mod secrets;
pub(crate) mod models;
pub(crate) mod handlers;
pub(crate) mod data_base;
pub(crate) mod test_utils;
pub(crate) mod errors;

pub(crate) mod structs;
pub(crate) mod plainning_modules;
pub(crate) mod permissions;
pub(crate) mod api_doc;

use structs::*;
use context::*;

use crate::{
    user_store::*,
    handlers::auth::*,
    handlers::user::*,
    handlers::event::*,
    secrets::token::TokenStore,
    secrets::verification::VerificationStore,
    data_base::user_db::create_pool,
    api_doc::ApiDoc
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

    let _guard = setup_logging();

    tracing::info!("Server started");

    let (tx, _rx) = broadcast::channel::<ChatMessage>(100);
    let user_store = Arc::new(Mutex::new(UserStore::new()));
    let verification_store = Arc::new(Mutex::new(VerificationStore::new()));
    let db_pool = create_pool(&database_url.as_str()).await?;

    let state = Arc::new(AppState {tx, user_store, verification_store, db_pool});

    {
        let mut store = state.user_store.lock().await;
        store.load_from_db(&state.db_pool).await.context("Error of loading db in cache")?;
    }

    let app = Router::new()
    .merge(SwaggerUi::new("/swagger_ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
    // Auth
    .route("/auth/request_code", post(request_code_handler))
    .route("/auth/resend_code", post(resend_code_handler))
    .route("/auth/verify_code", post(verify_code_handler))
    .route("/auth/register", post(register_handler))
    .route("/auth/token_validate", post(token_validate_handler))
    .route("/auth/logout", post(logout_handler))
    .route("/auth/check_username", post(username_check_handler))
    // User
    .route("/user/me", get(get_user_data_handler).put(user_edit_handler))
    .route("/user/me/avatar", post(upload_avatar_handler))
    .route("/avatars/{user_id}", get(get_avatar_handler))
    // Events
    .route("/events", get(list_events_handler).post(create_event_handler))
    .route("/events/{event_id}",
        get(get_event_handler)
        .put(update_event_handler)
        .delete(delete_event_handler))
    .route("/events/{event_id}/join", post(join_event_handler))
    .route("/events/{event_id}/members", post(add_member_handler))
    .route("/events/{event_id}/members/{user_id}",
        delete(remove_member_handler))
    .route("/events/{event_id}/members/{user_id}/permissions",
        put(update_member_permissions_handler))
    // Polls
    .route("/events/{event_id}/polls",
        get(list_polls_handler).post(create_poll_handler))
    .route("/events/{event_id}/polls/{poll_id}",
        get(get_poll_handler)
        .put(update_poll_handler)
        .delete(delete_poll_handler))
    .route("/events/{event_id}/polls/{poll_id}/vote", post(vote_handler))
    // Items
    .route("/events/{event_id}/items",
        get(list_items_handler).post(create_item_handler))
    .route("/events/{event_id}/items/{item_id}",
        put(update_item_handler).delete(delete_item_handler))
    // Chat и health
    .route("/chat", get(websocket_handler))
    .route("/health", get(health_handler))
    .with_state(state);

    // OK POST /auth/request_code {email: "test.example.com"} -> {"success": true} or {"success":false, error: "reason"}
    // OK POST /auth/resend_code {email: "test.example.com"} -> {"success": true} or {"success":false, error: "reason"}
    // OK POST /auth/verify_code {email: "test.example.com", code: "123456"} -> {is_new_user: true} or {token: "", is_new_user: false} or {error: "Verification failed"}
    // OK POST /auth/register {user: {email: "test.example.com", display_name: "display_name", birthday: "2000-01-01", "username": "test"}} -> if data.valid -> {token: ""} else {error: "reason"}
    // OK POST /auth/token_validate {token: ""} -> {success: true} or {success: false, error: "reason"}
    // OK POST /auth/logout {"token": ""} -> {success: true} or {success: false, error: "reason"}
    // OK POST /auth/check_username {"username": "test"} -> {"available": true} or {"available": false}
    // OK POST /user/edit {token: "", user: {email: "test.example.com", display_name: "display_name", birthday: "2000-01-01", "username": "test"}} -> if data.valid -> {success: true} else {error: "reason"}
    // OK GET /user/get-data {token: ""} -> {user: {email: "test.example.com", display_name: "display_name", birthday: "2000-01-01", "username": "test"}} or {error: "reason"}
    // OK POST /user/avatar {token: ""} and avatar -> {"avatar_url" : "avatar_url"}
    // OK GET /avatars/:avatar_uuid.jpg {"token": "token"} -> avatar through multipart


    // OK POST /events {token: "", event: {title: "Event title", description: "Event description", startDateTime: "2024-01-01T12:00:00Z", endDateTime: "2024-01-01T14:00:00Z", color: "blue"}} -> {event_id: ""} or {error: "reason"}
    // OK GET /events/{event_id} {token: ""} -> {event: {id: 1, title: "Event title", description: "Event description", startDateTime: "2024-01-01T12:00:00Z", endDateTime: "2024-01-01T14:00:00Z", event_token: ""}} or {error: "reason"}
    // OK POST /events/create_poll {token: "", event_id: 1, question: "Question?", options: ["Option 1", "Option 2"], more_than_one_vote: false} -> {poll_id: 1} or {error: "reason"}
    // OK GET /events/{event_id}/polls {token: ""} -> {polls: [{poll_id: 1, question: "Question?", options: [{id: 1, text: "Option 1", votes: 10}]}]} or {error: "reason"}
    // OK POST /events/add_member {event_token: "", user_id: "", permissions: ""} -> {success: "true"} or {error: "reason"}
    //
    // user_id добавить как отправку пользователю
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

fn setup_logging() -> tracing_appender::non_blocking::WorkerGuard {
    let file_appender = rolling::daily("logs", "app.log");
    let (non_blocking, guard) = non_blocking(file_appender);
    
    tracing_subscriber::registry()
        .with(fmt::layer().with_writer(std::io::stdout))
        .with(fmt::layer().with_writer(non_blocking))
        .with(EnvFilter::from_default_env())
        .init();
    
    guard
}
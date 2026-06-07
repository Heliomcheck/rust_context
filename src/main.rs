use tokio::{
    net::TcpListener,
    sync::broadcast
};
use anyhow::{
    Context,
    Result
};
use axum::{
    Router,
    extract::ws::WebSocketUpgrade,
    response::IntoResponse,
    routing
};
use axum::extract::State;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing_appender::{
    non_blocking,
    rolling
};
use tracing_subscriber::{
    fmt,
    prelude::*,
    EnvFilter
};
use utoipa_swagger_ui::SwaggerUi;
use utoipa::OpenApi;


mod context;

pub(crate) mod mail;
pub(crate) mod secrets;
pub(crate) mod models;
pub(crate) mod handlers;
pub(crate) mod data_base;
pub(crate) mod test_utils;
pub(crate) mod errors;

pub(crate) mod structs;
pub(crate) mod permissions;
pub(crate) mod api_doc;
pub(crate) mod config;

use structs::*;
use context::*;

use crate::{
    handlers::auth::*,
    handlers::user::*,
    handlers::event::*,
    secrets::verification::VerificationStore,
    data_base::user_db::create_pool,
    api_doc::ApiDoc,
    handlers::modules::poll::*,
    test_utils::health_handler,
    handlers::modules::item::*,
    handlers::modules::task::*,
    config::*,
};

use crate::handlers::album::{
    create_album_handler,
    get_albums_handler,
    get_album_handler,
    upload_photo_handler,
    get_photo_handler,
    delete_album_handler,
    delete_photo_handler,
};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    dotenvy::dotenv().ok();
    let config = Config::from_env();
    
    let args: Vec<String> = std::env::args().collect();

    let _guard = setup_logging();

    tracing::info!("Server started");

    let (tx, _rx) = broadcast::channel::<ChatMessage>(100);
    let verification_store = Arc::new(Mutex::new(VerificationStore::new()));
    let db_pool = create_pool(&config.database_url.as_str()).await?;

    sqlx::migrate!().run(&db_pool).await?;

    let state = Arc::new(AppState {tx, verification_store, db_pool, config});

    let app = create_app(state).await;
    
    let server_ip = std::env::var("SERVER_IP").unwrap_or_else(|_| "0.0.0.0".into());
    let server_port = std::env::var("SERVER_PORT").unwrap_or_else(|_| "8080".into());
    let address = format!("{}:{}", server_ip, server_port);
    let listner = TcpListener::bind(&address).await
        .context("Can't bind to address")?;

    println!("Server was start");

    axum::serve(listner, app).await
        .context("Server is false")?;
    
    Ok(())
}


async fn create_app(state: Arc<AppState>) -> Router {
    let app =Router::new()
        .merge(SwaggerUi::new("/swagger_ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .route("/auth/request_code", routing::post(request_code_handler))
        .route("/auth/verify_code", routing::post(verify_code_handler))
        .route("/auth/resend_code", routing::post(resend_code_handler)) //delete in future
        .route("/auth/register", routing::post(register_handler))
        .route("/auth/token_validate", routing::post(token_validate_handler))
        .route("/auth/logout", routing::post(logout_handler))
        .route("/auth/check_username", routing::post(username_check_handler))

        .route("/user/edit", routing::post(update_user_data_handler))
        .route("/user/get_data", routing::get(get_user_data_handler)) // user_id
        .route("/user/avatar", routing::post(upload_avatar_handler))

        //.route("/chat", routing::get(websocket_handler))
        .route("/health", routing::get(health_handler)) // delete in future

        .route("/avatars/{file_name}", routing::get(get_avatar_handler))

        .route("/events", routing::post(create_event_handler))
        .route("/events/{event_id}", routing::get(get_detailed_event_handler))
        .route("/events/{event_id}", routing::put(update_event_handler))
        .route("/events/{event_id}/status", routing::patch(update_event_status_handler))
        .route("/events/{event_id}", routing::delete(delete_event_handler))

        .route("/events", routing::get(get_user_events_handler)) // query required // status = ""/limit = 10/offset = 10
        .route("/events/join", routing::post(event_join_handler))
        .route("/events/{event_id}/members/{user_id}", routing::post(delete_user_from_event_handler))
        .route("/events/{event_id}/members/{user_id}", routing::put(update_user_permissions_handler))

        .route("/events/{event_id}/avatar", routing::post(upload_event_avatar_handler))
        .route("/event-avatars/{event_id}", routing::get(get_event_avatar_handler))
        .route("/events/{event_id}/avatar", routing::delete(delete_event_avatar_handler))

        .route("/events/{event_id}/planning", routing::get(get_modules_handler))

        // Планирование: опросы
        .route("/events/{event_id}/planning/poll", routing::post(create_poll_handler))
        .route("/events/{event_id}/planning/poll/{module_id}/vote", routing::patch(vote_poll_handler))
        .route("/events/{event_id}/planning/poll/{module_id}", routing::delete(delete_poll_handler))
        .route("/events/{event_id}/planning/poll/{module_id}", routing::put(update_poll_handler))

        // Планирование: списки вещей
        .route("/events/{event_id}/planning/item_list", routing::post(create_item_list_handler))
        .route("/events/{event_id}/planning/item_list/{module_id}", routing::patch(update_item_list_handler))
        .route("/events/{event_id}/planning/item_list/{module_id}/items/{item_id}/assign", routing::patch(assign_item_handler))
        .route("/events/{event_id}/planning/item_list/{module_id}", routing::delete(delete_item_list_handler))

        // Планирование: списки задач
        .route("/events/{event_id}/planning/task_list", routing::post(create_task_list_handler))
        .route("/events/{event_id}/planning/task_list/{module_id}", routing::patch(update_task_list_handler))
        .route("/events/{event_id}/planning/task_list/{module_id}/tasks/{task_id}/assign", routing::patch(assign_task_handler))
        .route("/events/{event_id}/planning/task_list/{module_id}/tasks/{task_id}/complete", routing::patch(complete_task_handler))
        .route("/events/{event_id}/planning/task_list/{module_id}", routing::delete(delete_task_list_handler))

        // Альбомы
        .route("/events/{event_id}/albums", routing::post(create_album_handler).get(get_albums_handler))
        .route("/events/{event_id}/albums/{album_id}", routing::get(get_album_handler).delete(delete_album_handler))
        .route("/events/{event_id}/albums/{album_id}/photos", routing::post(upload_photo_handler))
        .route("/events/{event_id}/albums/{album_id}/photos/{photo_id}", routing::get(get_photo_handler).delete(delete_photo_handler))

        .with_state(state);
    app
}

#[allow(dead_code)]
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
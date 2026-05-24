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
pub(crate) mod user_store;
pub(crate) mod secrets;
pub(crate) mod models;
pub(crate) mod handlers;
pub(crate) mod data_base;
pub(crate) mod test_utils;
pub(crate) mod errors;

pub(crate) mod structs;
pub(crate) mod permissions;
pub(crate) mod api_doc;

use structs::*;
use context::*;

use crate::{
    user_store::*,
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
};


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
        .route("/event/{event_id}", routing::get(get_detailed_event_handler))
        .route("/events/{event_id}", routing::put(update_event_handler))
        .route("/events/{event_id}/status", routing::patch(update_event_status_handler))
        .route("/events/{event_id}", routing::delete(delete_event_handler))
        
        .route("/events/", routing::get(get_user_events_handler)) // query required
        //.route("/events/{event_id}/avatar", routing::post(upload_event_avatar_handler)) // status = ""/limit = 10/offset = 10
        .route("/events/{event_id}/join", routing::post(event_join_handler))
        .route("/events/{event_id}/members/{user_id}", routing::post(delete_user_from_event_handler))
        .route("/events/{event_id}/members/{user_id}", routing::put(update_user_permissions_handler))

        .route("/events/{eventId}/planning", routing::get(get_modules_handler))

        .route("/events/{event_id}/planning/poll", routing::post(create_poll_handler))
        .route("/events/{event_id}/planning/poll/{poll_id}", routing::post(vote_poll_handler))
        .route("/events/{event_id}/planning/poll/{poll_id}", routing::delete(delete_poll_handler))
        .route("/events/{event_id}/planning/poll/{poll_id}", routing::put(update_poll_handler))

        .route("/events/{event_id}/planning/items", routing::post(create_item_list_handler))
        .route("/events/{event_id}/planning/items/{module_id}", routing::patch(update_item_list_handler))
        .route("/events/{event_id}/planning/items/{module_id}/items/{item_id}/assign", routing::post(assign_item_handler))
        .route("/events/{event_id}/planning/items/{module_id}", routing::delete(delete_item_list_handler))

        .route("/events/{event_id}/planning/tasks", routing::post(create_task_list_handler))
        .route("/events/{event_id}/planning/tasks/{module_id}", routing::patch(update_task_list_handler))
        .route("/events/{event_id}/planning/tasks/{module_id}/items/{task_id}/assign", routing::post(assign_task_handler))
        .route("/events/{event_id}/planning/tasks/{module_id}/items/{task_id}/complete", routing::post(complete_task_handler))
        .route("/events/{event_id}/planning/tasks/{module_id}", routing::delete(delete_task_list_handler))

        // .route("/events/create_item", routing::post(create_item_handler))
        // .route("/events/update_item", routing::post(update_item_handler))
        // .route("/events/create_task", routing::post(create_task_handler))
        // .route("/events/update_task", routing::post(update_task_handler))
        .with_state(state);
    
    let listner = TcpListener::bind(args[1].as_str()).await
        .context("Can't bind to address")?;

    println!("Server was start");

    axum::serve(listner, app).await
        .context("Server is false")?;

    
    Ok(())
}

#[axum_macros::debug_handler]
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
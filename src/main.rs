use tokio::{net::TcpListener, sync::broadcast};
use tokio::io::AsyncReadExt;
use anyhow::{Context, Result}; 
use futures_util::{SinkExt, StreamExt};
use axum::{extract::ws::{WebSocket, WebSocketUpgrade},
        Router, routing::get, 
        extract::Path, 
        response::IntoResponse};
use std::sync::Arc;

mod context;
mod structs;

use structs::*;
use context::*;

async fn health_handler() -> &'static str {
    "OK"
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let args: Vec<String> = std::env::args().collect();

    let (tx, _rx) = broadcast::channel::<ChatMessage>(100);

    let state = Arc::new(AppState {tx});

    let app = Router::new()
        .route("/chat", get(websocket_handler))
        .route("/health", get(health_handler))
        .with_state(state);
    
    let listner = TcpListener::bind(args[1].as_str()).await
        .context("Can't bind to address")?;

    println!("Server was start");

    axum::serve(listner, app).await
        .context("Server is false")?;

    
    Ok(())
}

async fn websocket_handler(
    ws: WebSocketUpgrade,
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_websocket(socket, state))
}
use tokio::{net::TcpListener, sync::broadcast};
use anyhow::{Context, Result}; 
use axum::{extract::ws::{WebSocketUpgrade},
        Router, routing::get,
        response::IntoResponse};
use std::sync::Arc;

mod context;
mod structs;

pub(crate) mod mail;
pub(crate) mod user;
pub(crate) mod generator;

use structs::*;
use context::*;
use mail::send_mail_verif_code;

async fn health_handler() -> &'static str {
    "OK"
}

async fn send_code_handler() -> &'static str {
    match send_mail_verif_code("heliom.check@gmail.com").await {
        Ok(_) => "OK",
        Err(e) => {
            eprintln!("Failed to send email: {}", e);
            "ERROR"
        }
    }
}

async fn sign_in_up_handler() -> &'static str {

    "OK"
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let args: Vec<String> = std::env::args().collect();

    let (tx, _rx) = broadcast::channel::<ChatMessage>(100);

    let state = Arc::new(AppState {tx});

    let app = Router::new()
        .route("/chat", get(websocket_handler))
        .route("/health", get(health_handler)) // delete in future
        .route("/code", get(send_code_handler))
        .route("/login", get(sign_in_up_handler))
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
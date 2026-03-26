use axum::{extract::ws::{WebSocket, WebSocketUpgrade},
        Router, routing::get, 
        extract::Path, 
        response::IntoResponse};
use std::sync::Arc;
use futures_util::StreamExt;
use futures_util::SinkExt;

use crate::{ChatMessage, AppState};

pub async fn handle_websocket (socket: WebSocket, state: Arc<AppState>) {
    println!("New connection");

    let (mut sender, mut receiver) = socket.split();

    let mut rx = state.tx.subscribe();

    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if let Ok(json) = serde_json::to_string(&msg) {
                if sender.send(json.into()).await.is_err() {
                }
            }
        }
    });

    let tx = state.tx.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if let Ok(chat_msg) = serde_json::from_str::<ChatMessage>(msg.to_text().unwrap_or("")) {
                println!("📨 Получено: {}: {}", chat_msg.username, chat_msg.text);
                let _ = tx.send(chat_msg);
            }
        }
    });

        tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    };

    println!("Client disconnect");
}
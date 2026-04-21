use futures_util::future::ok;
use tokio::{net::TcpListener, sync::broadcast};
use anyhow::{Context}; 
use axum::{Router, extract::ws::{WebSocket, WebSocketUpgrade}, response::IntoResponse, routing::{self, trace}
        };
use axum_macros::debug_handler;
use axum::extract::{State};
use std::sync::Arc;
use tokio::sync::Mutex;
use axum::Json;
use validator::Validate;
use axum::http::StatusCode;
use serde_json::json;
//use axum_auth::{Json, TypedHeader, headers::{Authorization, authorization::Bearer}};
use axum_extra::TypedHeader;
use headers::{Authorization, authorization::Bearer};
use sqlx::PgPool;
use tracing::*;
use axum::body::Body;
use tokio::fs;
use axum_extra::extract::multipart::Multipart;
use axum::extract::Path;
use std::path::PathBuf;

use crate::{data_base::user_db::find_user_by_token, generator, models::CheckUsernameRequest, secrets::token::{self, TokenStore}, structs::*};

use crate::{
    models::*,
    user::*,
    secrets::verification::VerificationStore,
    data_base::user_db::*
};

pub async fn user_edit_handler(
        auth: TypedHeader<Authorization<Bearer>>,
        State(state): State<Arc<AppState>>,
        Json(payload): Json<EditUserRequest>,
    ) -> impl IntoResponse {
    if let Err(errors) = payload.validate() {
        return validation_errors_to_response(errors);
    }

    let token = auth.token();
    
    let user = match find_user_by_token(&state.db_pool, token).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            return (StatusCode::UNAUTHORIZED, Json(json!({ "success": false, "error": "User not found" })));
        }
        Err(e) => {
            tracing::error!("Token validation error: {}", e);
            return (StatusCode::UNAUTHORIZED, Json(json!({ "success": false, "error": "Invalid session" })));
        }
    };
    
    if let Err(e) = edit_user_db(
        &state.db_pool,
        user.id,
        payload.username.as_deref(),
        payload.email.as_deref(),
        payload.display_name.as_deref(),
        payload.birthday.as_deref(),
        payload.avatar_url.as_deref()
    ).await {
        tracing::error!("Failed to update user: {}", e);
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "error": "Failed to update user" })));
    }
    // update user data un UserStore in future
    
    (StatusCode::OK, Json(json!({ "success": true })))
}

pub async fn get_user_data_handler(
    auth: TypedHeader<Authorization<Bearer>>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let token = auth.token();
    
    // 1. Находим пользователя по токену
    let user = match find_user_by_token(&state.db_pool, token).await {
        Ok(Some(u)) => u,
        Ok(None) => {
            return (StatusCode::UNAUTHORIZED, Json(json!({ "error": "Invalid or expired token" })));
        }
        Err(e) => {
            error!("Failed to find user by token: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Database error" })));
        }
    };
    
    // 2. Формируем ответ
    let response = UserDataResponse {
        id: user.id,
        username: user.username.clone(),  // ← clone()
        email: user.email.clone(),        // ← clone()
        name: user.name.clone(),          // ← clone()
        birthday: user.birthday.clone()
    };
    
    (StatusCode::OK, Json(json!({"user":{
        "username": user.username,
        "email": user.email,
        "display_name": user.name,
        "birthday": user.birthday
    }})))
}

const UPLOAD_DIR: &str = "uploads/avatars";

pub async fn upload_avatar_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let token = auth.token();
    
    let user = match find_user_by_token(&state.db_pool, token).await { // find user
        Ok(Some(u)) => u,
        Ok(None) => return (StatusCode::UNAUTHORIZED, Json(json!({ "error": "Invalid token" }))),
        Err(e) => {
            error!("DB error: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Database error" })));
        }
    };
    
    while let Ok(Some(field)) = multipart.next_field().await { // avatar download
        if field.name() != Some("avatar") {
            continue;
        }
        
        let file_name = match field.file_name() {
            Some(name) => name.to_string(),
            None => continue,
        };
        
        let ext = std::path::Path::new(&file_name)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("jpg");
        
        let data = match field.bytes().await {
            Ok(d) => d,
            Err(e) => {
                error!("Failed to read file: {}", e);
                return (StatusCode::BAD_REQUEST, Json(json!({ "error": "Failed to read file" })));
            }
        };
        
        let new_name = format!("{}.{}", generator::Generator::new_session_token(), ext); // generate name avatar
        let save_path = PathBuf::from(UPLOAD_DIR).join(&new_name); // sate in path
        
        if let Err(e) = fs::create_dir_all(UPLOAD_DIR).await {
            error!("Failed to create upload dir: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Internal error" })));
        }
        
        if let Err(e) = fs::write(&save_path, data).await {
            error!("Failed to save file: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Failed to save file" })));
        }
        
        let avatar_url = format!("/avatars/{}", new_name);
        if let Err(e) = update_user_avatar(&state.db_pool, user.id, &avatar_url).await {
            error!("Failed to update avatar URL: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Failed to update avatar" })));
        }
        
        return (StatusCode::OK, Json(json!({"success" : true, "avatar_url": avatar_url })));
    }
    
    (StatusCode::BAD_REQUEST, Json(json!({ "error": "No file provided" })))
}

pub async fn get_avatar_handler(
    Path(file_name): Path<String>,
    auth: TypedHeader<Authorization<Bearer>>,
    State(state): State<Arc<AppState>>
) -> impl IntoResponse {
    let token = auth.token();
    
    let user = match find_user_by_token(&state.db_pool, token).await { // check token
        Ok(Some(u)) => u,
        Ok(None) => {
            return (StatusCode::UNAUTHORIZED, "Invalid or expired token").into_response();
        }
        Err(e) => {
            error!("DB error: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response();
        }
    };
    
    let expected_avatar = format!("/avatars/{}", file_name);
    
    match user.avatar_url {
        Some(url) if url == expected_avatar => { // user owner avatar
        }
        Some(_) => {
            return (StatusCode::FORBIDDEN, "You can only access your own avatar").into_response();
        }
        None => {
            return (StatusCode::NOT_FOUND, "Avatar not found").into_response();
        }
    }
    
    let path = PathBuf::from(UPLOAD_DIR).join(&file_name); // upload file
    
    if !path.exists() {
        return (StatusCode::NOT_FOUND, "File not found").into_response();
    }
    
    let mime = mime_guess::from_path(&path).first_or_octet_stream();
    
    match fs::read(&path).await {
        Ok(data) => (
            StatusCode::OK,
            [(axum::http::header::CONTENT_TYPE, mime.to_string())],
            data,
        ).into_response(),
        Err(e) => {
            error!("Failed to read file: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to read file").into_response()
        }
    }
}
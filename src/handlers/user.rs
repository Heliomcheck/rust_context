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
use crate::test_utils::setup_test_db;
use tower::ServiceExt;
use axum::http::{Request, Response};
use chrono::Utc;
use axum::http::header::{self, HeaderMap};

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
            tracing::error!("User not found");
            return (StatusCode::UNAUTHORIZED, Json(json!({ "success": false, "error": "User not found" })));
        }
        Err(e) => {
            tracing::error!("Token validation error: {}", e);
            return (StatusCode::UNAUTHORIZED, Json(json!({ "success": false, "error": "Invalid session" })));
        }
    };
    
    if let Err(e) = edit_user_db(
        &state.db_pool,
        user.user_id,
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
        id: user.user_id,
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
    
    let user = match find_user_by_token(&state.db_pool, token).await {
        Ok(Some(u)) => u,
        Ok(None) => return (StatusCode::UNAUTHORIZED, Json(json!({ "error": "Invalid token" }))),
        Err(e) => {
            error!("DB error: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Database error" })));
        }
    };
    
    while let Ok(Some(field)) = multipart.next_field().await {
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
        
        let user_dir = PathBuf::from(UPLOAD_DIR).join(format!("user_{}", user.user_id)); // create user dir
        
        if let Err(e) = fs::create_dir_all(&user_dir).await {
            error!("Failed to create user dir: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Internal error" })));
        }
        
        let old_avatar_path = user_dir.join("avatar.*"); // delete old avatar
        if let Ok(mut entries) = fs::read_dir(&user_dir).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                let name = entry.file_name();
                if name.to_string_lossy().starts_with("avatar.") {
                    let _ = fs::remove_file(entry.path()).await;
                }
            }
        }
        
        let new_name = format!("avatar.{}", ext); // save new avatar
        let save_path = user_dir.join(&new_name);
        
        if let Err(e) = fs::write(&save_path, data).await {
            error!("Failed to save file: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Failed to save file" })));
        }
        
        // Обновляем avatar_url в БД
        let avatar_url = format!("/user/avatar");
        if let Err(e) = update_user_avatar(&state.db_pool, user.user_id, &avatar_url).await {
            error!("Failed to update avatar URL: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Failed to update avatar" })));
        }
        
        return (StatusCode::OK, Json(json!({ "success": true, "avatar_url": avatar_url })));
    }
    
    (StatusCode::BAD_REQUEST, Json(json!({ "error": "No file provided" })))
}

pub async fn get_avatar_handler(
    headers: HeaderMap,
    Path(user_id): Path<i64>,
) -> impl IntoResponse {
    let current_etag = compute_avatar_etag(user_id).await.unwrap_or("\"\"".to_string()); // compute etag

    if let Some(if_none_match) = headers.get(header::IF_NONE_MATCH) { // check 
        if if_none_match.to_str().unwrap_or("") == current_etag {
            return StatusCode::NOT_MODIFIED.into_response();
        }
    }

    let user_dir = PathBuf::from(UPLOAD_DIR).join(format!("user_{}", user_id));
    
    if !user_dir.exists() {
        return (StatusCode::NOT_FOUND, "Avatar not found").into_response();
    }
    
    let mut avatar_path = None; // find avatar.*
    if let Ok(mut entries) = fs::read_dir(&user_dir).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            let name = entry.file_name();
            if name.to_string_lossy().starts_with("avatar.") {
                avatar_path = Some(entry.path());
                break;
            }
        }
    }
    
    let path = match avatar_path {
        Some(p) => p,
        None => return (StatusCode::NOT_FOUND, "Avatar not found").into_response(),
    };
    
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

pub async fn compute_avatar_etag(user_id: i64) -> Result<String, std::io::Error> {
    let user_dir = PathBuf::from(UPLOAD_DIR).join(format!("user_{}", user_id));
    
    let mut avatar_path = None;
    if let Ok(mut entries) = fs::read_dir(&user_dir).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            let name = entry.file_name();
            if name.to_string_lossy().starts_with("avatar.") {
                avatar_path = Some(entry.path());
                break;
            }
        }
    }
    
    let path = match avatar_path {
        Some(p) => p,
        None => return Ok("\"\"".to_string()),
    };
    
    let data = fs::read(&path).await?;
    let hash = blake3::hash(&data);
    Ok(format!("\"{}\"", hash.to_hex()))
}

#[tokio::test]// Проверяет, что запрос без валидного токена возвращает UNAUTHORIZED
async fn test_get_user_data_handler_unauthorized() {
    let pool = setup_test_db().await;
    let state = Arc::new(AppState {
        tx: broadcast::channel(10).0,
        user_store: Arc::new(Mutex::new(UserStore::new())),
        verification_store: Arc::new(Mutex::new(VerificationStore::new())),
        db_pool: pool
    });
    let app = Router::new()
        .route("/user/get-data", routing::get(get_user_data_handler))
        .with_state(state);
    let request = Request::builder()
        .method("GET")
        .uri("/user/get-data")
        .header("Authorization", "Bearer invalid")
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
#[tokio::test]// Проверяет, что редактирование пользователя без валидного токена запрещено
async fn test_user_edit_handler_unauthorized() {
    let pool = setup_test_db().await;
    let state = Arc::new(AppState {
        tx: broadcast::channel(10).0,
        user_store: Arc::new(Mutex::new(UserStore::new())),
        verification_store: Arc::new(Mutex::new(VerificationStore::new())),
        db_pool: pool
    });
    let app = Router::new()
        .route("/user/edit", routing::post(user_edit_handler))
        .with_state(state);
    let payload = json!({
        "username": "newname"
    });
    let request = Request::builder()
        .method("POST")
        .uri("/user/edit")
        .header("Authorization", "Bearer invalid")
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]// Проверяет успешное получение данных пользователя по токену
async fn test_get_user_data_success() {
    let pool = setup_test_db().await;
    let user_id = create_user_db(
        &pool,
        "user1",
        "user1@mail.com",
        "User",
        &None,
        &None,
        &Some("test".to_string())
    ).await.unwrap();
    let token = "valid_token";
    create_token(
        &pool,
        user_id,
        token,
        Utc::now() + chrono::Duration::hours(1),
    ).await.unwrap();
    let state = Arc::new(AppState {
        tx: broadcast::channel(10).0,
        user_store: Arc::new(Mutex::new(UserStore::new())),
        verification_store: Arc::new(Mutex::new(VerificationStore::new())),
        db_pool: pool,
    });
    let app = Router::new()
        .route("/user/get-data", routing::get(get_user_data_handler))
        .with_state(state);
    let request = Request::builder()
        .method("GET")
        .uri("/user/get-data")
        .header("Authorization", format!("Bearer {}", token))
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}
use axum::{
    Json, 
    http::StatusCode,
    http::header::{self, HeaderMap},
    extract::{Path,State},
    response::IntoResponse
};
use validator::Validate;
use serde_json::json;
use headers::{
    Authorization, 
    authorization::Bearer
};
use sqlx::PgPool;
use tracing::*;
use tokio::fs;
use axum_extra::{
    extract::multipart::Multipart, 
    TypedHeader
};
use std::{
    sync::Arc, 
    path::PathBuf
};

use crate::{
    models::*,
    data_base::user_db::*,
    errors::AppError,
    structs::*,
};


#[utoipa::path(
    post,
    path = "/user/edit",
    tag = "User",
    request_body = EditUserRequest,
    security(
        ("bearerAuth" = [])
    ),
    responses(
        (status = 200, description = "User updated successfully", body = SuccessResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 401, description = "Unauthorized - invalid or expired token", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn update_user_data_handler(
        auth: TypedHeader<Authorization<Bearer>>,
        State(state): State<Arc<AppState>>,
        Json(payload): Json<EditUserRequest>,
    ) -> Result<impl IntoResponse, AppError> {
    if let Err(errors) = payload.validate() {
        return Err(validation_errors_to_response(errors));
    }
    
    let user = get_user_for_handler_from_token(&state.db_pool, &auth.token()).await?;
    
    if let Err(e) = edit_user_db(
        &state.db_pool,
        user.user_id,
        payload.username.as_deref(),
        payload.display_name.as_deref(),
        payload.birthday.as_deref(),
        payload.description.as_deref()
    ).await {
        tracing::error!("Failed to update user: {}", e);
        (AppError::Internal("Failed to update user".to_string()));
    }
    // update user data un UserStore in future
    
    Ok((StatusCode::OK, Json(SuccessResponse{success: true})))
}

#[utoipa::path(
    get,
    path = "/user/get_data",
    tag = "User",
    security(
        ("bearerAuth" = [])
    ),
    responses(
        (status = 200, description = "Get user data successfully", body = GetUserDataResponseWrapper),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 401, description = "Unauthorized - invalid or expired token", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn get_user_data_handler(
    auth: TypedHeader<Authorization<Bearer>>,
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, AppError> {
    let user = get_user_for_handler_from_token(&state.db_pool, &auth.token()).await?;
    
    let response = UserDataResponse {
        user_id: user.user_id,
        username: user.username.clone(),
        email: user.email.clone(),
        display_name: Some(user.display_name.clone()),
        birthday: user.birthday.clone(),
        description: user.description_profile
    };
    
    Ok((StatusCode::OK, Json(json!({"user": response}))))
}

const UPLOAD_DIR: &str = "uploads/avatars";
#[utoipa::path(
    post,
    path = "/user/avatar",
    tag = "User",
    request_body(
        content_type = "multipart/form-data",
        description = "Avatar file to upload",
    ),
    security(
        ("bearerAuth" = [])
    ),
    responses(
        (status = 200, description = "Avatar uploaded successfully", body = SuccessResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 401, description = "Unauthorized - invalid or expired token", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn upload_avatar_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, AppError> {
    let user = get_user_for_handler_from_token(&state.db_pool, &auth.token()).await?;
    
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
                return Err(AppError::BadRequest("Failed to read file".to_string()));
            }
        };
        
        let user_dir = PathBuf::from(UPLOAD_DIR).join(format!("user_{}", user.user_id)); // create user dir
        
        if let Err(e) = fs::create_dir_all(&user_dir).await {
            error!("Failed to create user dir: {}", e);
            return Err(AppError::Internal("Failed to create user dir".to_string()));
        }
        
        let _ = user_dir.join("avatar.*"); // delete old avatar
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
            return Err(AppError::Internal("Failed to save file".to_string()));
        }
        
        let avatar_url = format!("/user/avatar");
        if let Err(e) = update_user_avatar(&state.db_pool, user.user_id, &avatar_url).await {
            error!("Failed to update avatar URL: {}", e);
            return Err(AppError::Internal("Failed to update avatar".to_string()));
        }
        
        return Ok((StatusCode::OK, Json(SuccessResponse{success: true})));
    }
    
    return Err(AppError::BadRequest("No file provided".to_string()));
}

#[utoipa::path(
    get,
    path = "/avatars/{user_id}",
    tag = "Avatar",
    security(
        ("bearerAuth" = [])
    ),
    params(
        ("user_id" = i64, Path, description = "User ID")
    ),
    responses(
        (status = 200, description = "Get avatar successfully"),
        (status = 304, description = "Not modified - avatar not changed"),
        (status = 401, description = "Unauthorized - invalid or expired token", body = ErrorResponse),
        (status = 404, description = "Avatar not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn get_avatar_handler(
    headers: HeaderMap,
    Path(user_id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    let current_etag = compute_avatar_etag(user_id).await?;

    // Проверяем стандартный заголовок If-None-Match
    if let Some(if_none_match) = headers.get(header::IF_NONE_MATCH) {
        if if_none_match.to_str().unwrap_or("") == current_etag {
            return Ok(StatusCode::NOT_MODIFIED.into_response());
        }
    }

    let user_dir = PathBuf::from(UPLOAD_DIR).join(format!("user_{}", user_id));
    
    if !user_dir.exists() {
        return Err(AppError::NotFound("Avatar not found".to_string()));
    }
    
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
        None => return Err(AppError::NotFound("Avatar not found".to_string())),
    };
    
    let mime = mime_guess::from_path(&path).first_or_octet_stream();
    let data = fs::read(&path).await.map_err(|e| {
        error!("Failed to read file: {}", e);
        AppError::Internal("Failed to read file".to_string())
    })?;
    
    // Создаём response с заголовками
    let mut response = (StatusCode::OK, data).into_response();
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        mime.to_string().parse().unwrap(),
    );
    response.headers_mut().insert(
        header::ETAG,
        current_etag.parse().unwrap(),
    );
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        "public, max-age=3600".parse().unwrap(),
    );
    
    Ok(response)
}

pub async fn compute_avatar_etag(user_id: i64) -> Result<String, AppError> {
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
    
    let data = fs::read(&path).await.map_err(|e| {
        error!("Failed to read avatar file: {}", e);
        AppError::Internal("Failed to read file".to_string())
    })?;
    
    let hash = blake3::hash(&data);
    Ok(format!("\"{}\"", hash.to_hex()))
}

pub async fn get_user_for_handler_from_token(pool: &PgPool, token: &str) -> Result<User, AppError> {
    let user = match find_user_by_token(pool, token).await {
        Ok(Some(user)) => {
            update_last_online(pool, user.user_id).await?;
            user
        },
        Ok(None) => {
            tracing::error!("User not found");
            return Err(AppError::InvalidToken);
        }
        Err(e) => {
            tracing::error!("Token validation error: {}", e);
            return Err(AppError::InvalidToken);
        }
    };
    Ok(user)
}

pub async fn get_user_for_handler_from_id(pool: &PgPool, user_id: i64) -> Result<User, AppError> {
    let user = match find_user_by_id(pool, user_id).await {
        Ok(Some(user)) => {
            update_last_online(pool, user.user_id).await?;
            user
        },
        Ok(None) => {
            tracing::error!("User not found");
            return Err(AppError::InvalidToken);
        }
        Err(e) => {
            tracing::error!("Token validation error: {}", e);
            return Err(AppError::InvalidToken);
        }
    };
    Ok(user)
}

#[cfg(test)]
mod tests {
    use tokio::sync::{
        broadcast,
        Mutex
    }; 
    use axum::{
        Router, 
        routing,
        body::Body,
        http::Request,
        http::StatusCode
    };
    use std::sync::Arc;
    use serde_json::json;
    use tower::ServiceExt;
    use chrono::Utc;

    use crate::{
        user_store::*,
        secrets::verification::VerificationStore,
        data_base::user_db::*,
        test_utils::setup_test_db,
        structs::*,
        *
    };

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
            .route("/user/edit", routing::post(update_user_data_handler))
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
}
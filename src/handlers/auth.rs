use chrono::Utc;
use axum::{
    response::IntoResponse,
    extract::State,
    Json,
    http::StatusCode,
};
use std::{
    sync::Arc,
    result::Result
};
use validator::Validate;
use serde_json::json;
use axum_extra::TypedHeader;
use headers::{
    Authorization, 
    authorization::Bearer
};
use tracing::*;

use crate::{
    data_base::user_db::*, 
    models::CheckUsernameRequest, 
    secrets::generator,
    structs::*,
    models::*,
    errors::AppError,
    mail::send_mail_verif_code
};

#[utoipa::path(
    post,
    path = "/auth/register",
    tag = "Auth",
    request_body = RegisterRequestWrapper,
    responses(
        (status = 201, description = "User registered", body = RegisterResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 404, description = "User or event not found", body = ErrorResponse),
        (status = 409, description = "Conflict - email or username already exists", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn register_handler(
    State(state): State<Arc<AppState>>,
    Json(wrapper): Json<RegisterRequestWrapper>,
) -> Result<impl IntoResponse, AppError> {
    let payload = wrapper.user;
    if let Err(errors) = payload.validate() {
        return Err(validation_errors_to_response(errors));
    }
    
    match find_user_by_email(&state.db_pool, &payload.email).await {
        Ok(Some(_)) => {
            return Err(AppError::Conflict);
        }
        Ok(None) => {}
        Err(e) => {
            tracing::error!("DB error: {}", e);
            return Err(AppError::Internal("Database error find user by email".to_string()));
        }
    }
    
    match find_user_by_username(&state.db_pool, &payload.username).await {
        Ok(Some(_)) => {
            return Err(AppError::Conflict);
        }
        Ok(None) => {}
        Err(e) => {
            tracing::error!("DB error: {}", e);
            return Err(AppError::Internal("Database error find user by username".to_string()));
        }
    }
    
    let user_id = match create_user_db(
        &state.db_pool,
        &payload.username,
        &payload.email,
        &payload.display_name,
        &payload.birthday,
        &payload.description
    ).await {
        Ok(id) => id,
        Err(e) => {
            let error_msg = e.to_string();
            if error_msg.contains("duplicate key") && error_msg.contains("username") {
                return Err(AppError::Conflict);
            }
            if error_msg.contains("duplicate key") && error_msg.contains("email") {
                return Err(AppError::Conflict);
            }
            return Err(AppError::Internal("Database error for create user".to_string()));
        }
    };

    let ttl_user_token = std::env::var("TTL_USER_TOKEN")
        .ok()
        .and_then(|s|s.parse::<i64>().ok())
        .unwrap_or(30);
    
    let token = generator::Generator::new_session_token();
    let expires_at = Utc::now() + chrono::Duration::days(ttl_user_token);
    
    if let Err(e) = create_token(&state.db_pool, user_id, &token, expires_at).await {
        tracing::error!("Failed to create token: {}", e);
        return Err(AppError::Internal("Failed to create session".to_string()));
    }
    
    // let mut user_store = state.user_store.lock().await;
    // if let Err(e) = user_store.add_user(
    //     user_id,
    //     payload.username.clone(),
    //     payload.email.clone(),
    //     payload.birthday.clone(),
    //     payload.display_name.clone(),
    //     payload.description.clone(),
    //     &state.db_pool,
    // ).await {
    //     tracing::warn!("User created in DB but failed to add to cache: {}", e);
    // }
    
    Ok((StatusCode::CREATED, Json(json!(RegisterResponse {token, user_id}))))
}

#[utoipa::path(
    post,
    path = "/auth/request_code",
    tag = "Auth",
    request_body = CodeRequest,
    responses(
        (status = 201, description = "Code sent", body = SuccessResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn request_code_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CodeRequest>
) -> Result<impl IntoResponse, AppError> {
    if let Err(errors) = payload.validate() {
        return Err(validation_errors_to_response(errors));
    }

    match send_mail_verif_code(&payload.email, state).await {
        Ok(()) =>
            Ok((StatusCode::CREATED, Json(json!({"success": true})))),
        Err(e) => {
            print!("{e}");
            Err(AppError::Internal("Failed to send verification code".to_string()))
        }
    }
}

#[utoipa::path(
    post,
    path = "/auth/verify_code",
    tag = "Auth",
    request_body = VerifyCodeRequest,
    responses(
        (status = 200, description = "User verified(old user)", body = NewUserVerifyResponse),
        (status = 201, description = "User verified(new user)", body = SuccessResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn verify_code_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<VerifyCodeRequest>
) -> Result<impl IntoResponse, AppError> {
    if let Err(errors) = payload.validate() {
        return Err(validation_errors_to_response(errors));
    }

    if !state.verification_store.lock().await.verify(&payload.email, &payload.code) {
        return Err(AppError::BadRequest("Invalid or expired code".to_string()));
    }

    let user = match find_user_by_email(&state.db_pool, &payload.email).await {
        Ok(Some(u)) => u,
        Ok(None) => {
            return Ok((StatusCode::OK, Json(json!({ "is_new_user": true }))));
        }
        Err(_) => {
            tracing::error!("DB error (verify_code_handler)");
            return Err(AppError::Internal("Database error find user by email".to_string()));
        }
    };

    let token = match find_token_by_user_id(&state.db_pool, user.user_id).await {
        Ok(Some(t)) => t,
        Ok(None) => {
            let ttl_user_token = std::env::var("TTL_USER_TOKEN")
                .ok()
                .and_then(|s|s.parse::<i64>().ok())
                .unwrap_or(30);
            let new_token = uuid::Uuid::new_v4().to_string();
            let expires_at = Utc::now() + chrono::Duration::days(ttl_user_token);
            
            if let Err(e) = create_token(&state.db_pool, user.user_id, &new_token, expires_at).await {
                tracing::error!("Failed to create token: {}", e);
                return Err(AppError::Internal("Failed to create session".to_string()));
            }
            new_token
        }
        Err(e) => {
            tracing::error!("Failed to find token: {}", e);
            return Err(AppError::Internal("Database error find user by token".to_string()));
        }
    };

    Ok((StatusCode::OK, Json(json!(NewUserVerifyResponse{ 
        is_new_user: false, 
        token, 
        user_id: user.user_id 
    }))))
}

#[utoipa::path(
    post,
    path = "/auth/token_validate",
    tag = "Auth",
    security(
        ("bearerAuth" = [])
    ),
    responses(
        (status = 200, description = "Token is valid", body = SuccessResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn token_validate_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
) -> Result<impl IntoResponse, AppError> {

    let token = auth.token();
    match validate_token(&state.db_pool, token).await {
        Ok(true) => Ok((StatusCode::OK, Json(json!({"success": true})))),
        Ok(false) => Err(AppError::InvalidToken),
        Err(e) => Err(AppError::Internal(format!("Token validation failed: {e}")))
    }
}

#[utoipa::path(
    post,
    path = "/auth/check_username",
    tag = "Auth",
    request_body = CheckUsernameRequest,
    responses(
        (status = 200, description = "Username is available", body = SuccessResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn username_check_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CheckUsernameRequest>
) -> Result<impl IntoResponse, AppError> {
    if let Err(errors) = payload.validate() {
        return Err(validation_errors_to_response(errors));
    }

    let exists: bool = match find_user_by_username(&state.db_pool, &payload.username).await {
        Ok(Some(_)) => true,
        Ok(None) => false,
        Err(e) => {
            error!("DB error: {}", e);
            return Err(AppError::Internal(format!("Database error: {}", e)));
        }
    };

    Ok((StatusCode::OK, Json(json!({ "success": !exists }))))
}

#[utoipa::path(
    post,
    path = "/auth/resend_code",
    tag = "Auth",
    request_body = CodeRequest,
    responses(
        (status = 200, description = "Code resent successfully", body = SuccessResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 429, description = "Too many requests - rate limit exceeded", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn resend_code_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CodeRequest>
) -> Result<impl IntoResponse, AppError> {
    if let Err(errors) = payload.validate() {
        return Err(validation_errors_to_response(errors));
    }

    match send_mail_verif_code(&payload.email, state).await {
        Ok(()) => Ok((StatusCode::OK, Json(json!({ "success": true})))),
        Err(e) => Err(AppError::Internal(e.to_string()))
    }
}

#[utoipa::path(
    post,
    path = "/auth/logout",
    tag = "Auth",
    security(
        ("bearerAuth" = [])
    ),
    responses(
        (status = 200, description = "Logged out successfully", body = SuccessResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 401, description = "Unauthorized - invalid or expired token", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn logout_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
) -> Result<impl IntoResponse, AppError> {
    let token = auth.token();
    
    match find_user_by_token(&state.db_pool, token).await { // check user
        Ok(Some(user)) => {
            match deactivate_token(&state.db_pool, token).await {
                Ok(_) => {
                    tracing::info!("User {} logged out", user.user_id);
                    Ok((StatusCode::OK, Json(json!({"success": true}))))
                }
                Err(e) => {
                    tracing::error!("Failed to deactivate token: {}", e);
                    Err(AppError::Internal("Failed to logout".to_string()))
                }
            }
        }
        Ok(None) => {
            Err(AppError::InvalidToken)
        }
        Err(e) => {
            tracing::error!("DB error: {}", e);
            Err(AppError::Internal("Database error".to_string()))
        }
    }
}

//test
#[cfg(test)]
mod tests {
    use chrono::Utc;
    use axum::{Router, routing};
    use tokio::sync::Mutex;
    use axum::http::StatusCode;
    use serde_json::json;
    use axum::body::Body;
    use axum::http::Request;
    use tower::util::ServiceExt;

    use crate::{
        data_base::user_db::*,
        structs::*,
        secrets::verification::VerificationStore,
        *
    };
    use crate::test_utils::*;

    #[tokio::test]
    async fn test_register_handler() -> anyhow::Result<()> {
        let pool = setup_test_db().await;

        sqlx::query!("DELETE FROM token_store").execute(&pool).await?;
        sqlx::query!("DELETE FROM users").execute(&pool).await?;

        let state = Arc::new(AppState {
            tx: broadcast::channel(10).0,
            verification_store: Arc::new(Mutex::new(VerificationStore::new())),
            db_pool: pool,
            config: Config::from_env()
        });

        let app = Router::new()
            .route("/auth/register", routing::post(register_handler))
            .with_state(state);

        let payload = json!({
            "user" : {
                "username": "testuser",
                "email": "test@mail.com",
                "birthday": null,
                "display_name": "Test",
                "description": null
            }
        });

        let request: Request<Body> = Request::builder()
            .method("POST")
            .uri("/auth/register")
            .header("content-type", "application/json")
            .body(Body::from(payload.to_string()))?;

        let response = app.oneshot(request).await?;
        assert_eq!(response.status(), axum::http::StatusCode::CREATED);
        Ok(())
    }

    #[tokio::test]
    async fn test_token_validate_handler_invalid() -> anyhow::Result<()> {
        let pool = setup_test_db().await;
        let state = Arc::new(AppState {
            tx: broadcast::channel(10).0,
            //user_store: Arc::new(Mutex::new(UserStore::new())),
            verification_store: Arc::new(Mutex::new(VerificationStore::new())),
            db_pool: pool,
            config: Config::from_env()
        });
        let app = Router::new()
            .route("/auth/token-validate", routing::post(token_validate_handler))
            .with_state(state);
        let request = Request::builder()
            .method("POST")
            .uri("/auth/token-validate")
            .header("Authorization", "Bearer invalid")
            .body(Body::empty())?;
        let response = app.oneshot(request).await?;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        Ok(())
    }

    #[tokio::test]
    async fn test_logout_handler_success() -> anyhow::Result<()> {
        let pool = setup_test_db().await;
        let user_id = create_user_db(
            &pool,
            "logout_user",
            "logout@mail.com",
            "Logout User",
            &None,
            &None,
        ).await?;
        let token = "logout_token";
        create_token(&pool, user_id, token, Utc::now() + chrono::Duration::hours(1)).await?;

        let state = Arc::new(AppState {
            tx: broadcast::channel(10).0,
            //user_store: Arc::new(Mutex::new(UserStore::new())),
            verification_store: Arc::new(Mutex::new(VerificationStore::new())),
            db_pool: pool.clone(),
            config: Config::from_env()
        });

        let app = Router::new()
            .route("/auth/logout", routing::post(logout_handler))
            .with_state(state);

        let request = Request::builder()
            .method("POST")
            .uri("/auth/logout")
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())?;

        let response = app.oneshot(request).await?;
        assert_eq!(response.status(), StatusCode::OK);

        let user = find_user_by_token(&pool, token).await?;
        assert!(user.is_none());
        Ok(())
    }

    #[tokio::test]
    async fn test_username_check_handler() -> anyhow::Result<()> {
        let pool = setup_test_db().await;
        create_user_db(
            &pool,
            "taken",
            "taken@mail.com",
            "Test",
            &None,
            &None,
        ).await?;
        let state = Arc::new(AppState {
            tx: broadcast::channel(10).0,
            verification_store: Arc::new(Mutex::new(VerificationStore::new())),
            db_pool: pool,
            config: Config::from_env()
        });
        let app = Router::new()
            .route("/auth/check-username", routing::post(username_check_handler))
            .with_state(state);
        let payload = json!({ "username": "taken" });
        let request = Request::builder()
            .method("POST")
            .uri("/auth/check-username")
            .header("content-type", "application/json")
            .body(Body::from(payload.to_string()))?;
        let response = app.oneshot(request).await?;
        assert_eq!(response.status(), StatusCode::OK);
        Ok(())
    }

    #[tokio::test]
    async fn test_verify_code_new_user() -> anyhow::Result<()> {
        let pool = setup_test_db().await;

        let email = "new@mail.com";
        let mut verification = VerificationStore::new();
        let code = verification.create(email, 60);

        let user_id = create_user_db(
            &pool,
            "newuser",
            email,
            "New User",
            &None,
            &None,
        ).await?;

        let token = "test_token";
        let expires_at = Utc::now() + chrono::Duration::days(30);
        create_token(&pool, user_id, token, expires_at).await?;

        let state = Arc::new(AppState {
            tx: broadcast::channel(10).0,
            verification_store: Arc::new(Mutex::new(verification)),
            db_pool: pool,
            config: Config::from_env()
        });

        let app = Router::new()
            .route("/auth/verify-code", routing::post(verify_code_handler))
            .with_state(state);

        let payload = json!({
            "email": email,
            "code": code
        });

        let request = Request::builder()
            .method("POST")
            .uri("/auth/verify-code")
            .header("content-type", "application/json")
            .body(Body::from(payload.to_string()))?;

        let response = app.oneshot(request).await?;
        assert_eq!(response.status(), StatusCode::OK);
        Ok(())
    }

    #[tokio::test]
    async fn test_verify_code_invalid() -> anyhow::Result<()> {
        let pool = setup_test_db().await;
        let state = Arc::new(AppState {
            tx: broadcast::channel(10).0,
            verification_store: Arc::new(Mutex::new(VerificationStore::new())),
            db_pool: pool,
            config: Config::from_env()
        });
        let app = Router::new()
            .route("/auth/verify-code", routing::post(verify_code_handler))
            .with_state(state);
        let payload = json!({ "email": "test@mail.com", "code": "000000" });
        let request = Request::builder()
            .method("POST")
            .uri("/auth/verify-code")
            .header("content-type", "application/json")
            .body(Body::from(payload.to_string()))?;
        let response = app.oneshot(request).await?;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        Ok(())
    }
}
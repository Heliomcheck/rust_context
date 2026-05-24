use thiserror::Error;
use axum::{response::IntoResponse, http::StatusCode, Json};
use serde_json::json;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Database error")]
    DbError(#[from] sqlx::Error),
    #[error("Event not found")]
    EventNotFound,
    #[error("Already exists")]
    Conflict,
    #[error("Invalid or expired token")]
    InvalidToken,
    #[error("Internal error: {0}")]
    Internal(String),
    #[error("Invalid request")]
    BadRequest(String),
    #[error("User is not member")]
    UserNotInEvent(String),
    #[error("Not found")]
    NotFound(String),
    #[error("Unauthorized")]
    Unauthorized,
    #[error("Forbidden: {0}")]
    Forbidden(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, msg) = match self {
            AppError::EventNotFound => (StatusCode::NOT_FOUND, "Event not found".to_string()),
            AppError::Conflict => (StatusCode::CONFLICT, "Already exists".to_string()),
            AppError::InvalidToken => (StatusCode::UNAUTHORIZED, "Invalid token".to_string()),
            AppError::DbError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string()),
            AppError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::UserNotInEvent(msg) => (StatusCode::FORBIDDEN, msg),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized".to_string()),
            AppError::Forbidden(msg) => (StatusCode::FORBIDDEN, msg),
        };
        (status, Json(json!({ "error": msg }))).into_response()
    }
}
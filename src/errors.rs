use thiserror::Error;
use axum::{response::IntoResponse, http::StatusCode, Json};
use serde_json::json;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Database error")]
    DbError(#[from] sqlx::Error),
    #[error("Event not found")]
    EventNotFound,
    #[error("Participant not found")]
    ParticipantNotFound,
    #[error("User not found")]
    UserNotFound,
    #[error("Already exists")]
    Conflict,
    #[error("Invalid or expired token")]
    InvalidToken,
    #[error("Internal error: {0}")]
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, msg) = match self {
            AppError::EventNotFound => (StatusCode::NOT_FOUND, "Event not found"),
            AppError::ParticipantNotFound => (StatusCode::NOT_FOUND, "Participant not found"),
            AppError::UserNotFound => (StatusCode::NOT_FOUND, "User not found"),
            AppError::Conflict => (StatusCode::CONFLICT, "Already exists"),
            AppError::InvalidToken => (StatusCode::UNAUTHORIZED, "Invalid token"),
            AppError::DbError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Database error"),
            AppError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, &msg),
        };
        (status, Json(json!({ "error": msg }))).into_response()
    }
}
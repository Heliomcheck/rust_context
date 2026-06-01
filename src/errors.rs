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
//TEST

#[cfg(test)]
mod tests {
    use super::*;
    use axum::response::IntoResponse;
    use http_body_util::BodyExt;

    #[tokio::test]
    async fn test_all_error_responses() {
        let test_cases = vec![
            (AppError::EventNotFound, StatusCode::NOT_FOUND, "Event not found"),
            (AppError::Conflict, StatusCode::CONFLICT, "Already exists"),
            (AppError::InvalidToken, StatusCode::UNAUTHORIZED, "Invalid token"),
            (AppError::DbError(sqlx::Error::WorkerCrashed), StatusCode::INTERNAL_SERVER_ERROR, "Database error"),
            (AppError::Internal("custom".into()), StatusCode::INTERNAL_SERVER_ERROR, "custom"),
            (AppError::BadRequest("bad".into()), StatusCode::BAD_REQUEST, "bad"),
            (AppError::UserNotInEvent("no".into()), StatusCode::FORBIDDEN, "no"),
            (AppError::NotFound("missing".into()), StatusCode::NOT_FOUND, "missing"),
            (AppError::Unauthorized, StatusCode::UNAUTHORIZED, "Unauthorized"),
            (AppError::Forbidden("denied".into()), StatusCode::FORBIDDEN, "denied"),
        ];

        for (error, expected_status, expected_message) in test_cases {
            let response = error.into_response();
            assert_eq!(response.status(), expected_status, "Status mismatch for {:?}", expected_status);
            let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
            let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
            assert_eq!(body["error"].as_str().unwrap(), expected_message);
        }
    }
}
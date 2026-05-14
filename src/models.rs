use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use validator::{self, Validate, ValidationErrors};
use axum::http::StatusCode;
use axum::Json;
use chrono::{DateTime, Utc};
use uuid::Uuid;

pub fn validation_errors_to_response(errors: ValidationErrors) -> (StatusCode, Json<Value>) {
    let mut error_map = serde_json::Map::new();

    for (field, field_errors) in errors.field_errors() {
        let messages: Vec<String> = field_errors
            .iter()
            .filter_map(|err| err.message.as_ref().map(|msg| msg.to_string()))
            .collect();

        if !messages.is_empty() {
            error_map.insert(field.to_string(), json!(messages));
        }
    }

    (StatusCode::BAD_REQUEST, Json(json!({ "errors": error_map })))
}

#[derive(Deserialize)]
pub struct RegisterRequestWrapper {
    pub user: RegisterRequest,
}

#[derive(Deserialize, Serialize, Validate)]
pub struct RegisterRequest {
    #[validate(email(
        message = "Email format invalid"))]
    pub email: String,
    #[validate(length(min = 5, max = 30,
        message = "Username length must be more than 4 characters"))]
    pub username: String,
    #[validate(length(min = 10, max = 10, 
        message = "Birthday format must be xx-xx-xxxx"))]
    pub birthday: Option<String>,
    #[validate(length(min = 1, max = 100,
        message = "Display name cannot be empty"))]
    pub display_name: String,
    #[validate(length(max = 100, 
        message = "Description length can't be more than 100 characters"))]
    pub description: Option<String>
}

#[derive(Deserialize, Serialize, Validate)]
pub struct CodeRequest {
    #[validate(email(
        message = "Email format invalid"))]
    pub email: String
}

#[derive(Deserialize, Serialize, Validate)]
pub struct VerifyCodeRequest {
    #[validate(email(
        message = "Email format invalid"))]
    pub email: String,
    #[validate(length(min = 6, max = 6, 
        message = "Code must be 6 digits"))]
    pub code: String
}

#[derive(Debug, Deserialize, Validate)]
pub struct CheckUsernameRequest {
    #[validate(length(min = 3, message = "Username must be at least 3 characters"))]
    pub username: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct EditUserRequest {
    #[validate(length(min = 5, max = 30,
        message = "Username length must be more than 4 characters"))]
    pub username: Option<String>,
    #[validate(email(
        message = "Email format invalid"))]
    pub email: Option<String>,
    #[validate(length(min = 10, max = 10,
        message = "Birthday format must be xx-xx-xxxx"))]
    pub birthday: Option<String>,
    #[validate(length(min = 1, max = 100,
        message = "Display name cannot be empty"))]
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    #[validate(length(max = 100, 
        message = "Description length can't be more than 100 characters"))]
    pub descripion: Option<String>
}

#[derive(Debug, Serialize)]
pub struct UserDataResponse {
    pub id: i64,
    pub username: String,
    pub email: String,
    pub name: String,
    pub birthday: Option<String>
}

#[derive(Debug, Deserialize)]
pub struct CreateEventRequest {
    pub title: String,
    pub description: Option<String>,
    pub startDateTime: Option<String>,   // ISO строка
    pub endDateTime: Option<String>,
    pub color: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateEventRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub startDateTime: Option<String>,
    pub endDateTime: Option<String>,
    pub color: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ChangeEventStatusRequest {
    pub status: String, // "active" или "archived"
}

#[derive(Debug, Serialize)]
pub struct EventResponse {
    pub id: i64,
    pub title: String,
    pub description: Option<String>,
    pub startDateTime: Option<String>,
    pub endDateTime: Option<String>,
    pub color: Option<String>,
    #[serde(rename = "createdBy")]
    pub created_by: String,   // user_id как строка
    pub createdAt: String,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct EventListResponse {
    pub items: Vec<EventResponse>,
    pub total: i64,
}

pub struct NewEvent {
    pub title: String,
    pub description: Option<String>,
    pub start_date_time: Option<DateTime<Utc>>,
    pub end_date_time: Option<DateTime<Utc>>,
    pub color: Option<String>,
    pub created_by: i64,
}

#[derive(Debug, Deserialize)]
pub struct EventsQuery {
    pub status: String,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}
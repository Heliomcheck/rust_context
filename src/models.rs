use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use validator::{self, Validate, ValidationErrors};
use axum::{http::StatusCode};
use axum::Json;
use utoipa::{ToSchema};

use crate::{
    structs::*,
};

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

#[derive(Deserialize, ToSchema)]
pub struct RegisterRequestWrapper {
    pub user: RegisterRequest,
}

#[derive(Deserialize, Serialize, Validate, ToSchema)]
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

#[derive(Deserialize, Serialize, Validate, ToSchema)]
pub struct RegisterResponse {
    pub token: String,
    pub user_id: i64
}

#[derive(Deserialize, Serialize, Validate, ToSchema)]
pub struct NewUserVerifyResponse {
    pub is_new_user: bool, 
    pub token: String, 
    pub user_id: i64 
}

#[derive(Deserialize, Serialize, Validate, ToSchema)]
pub struct CodeRequest {
    #[validate(email(
        message = "Email format invalid"))]
    pub email: String
}

#[derive(Deserialize, Serialize, Validate, ToSchema)]
pub struct VerifyCodeRequest {
    #[validate(email(
        message = "Email format invalid"))]
    pub email: String,
    #[validate(length(min = 6, max = 6, 
        message = "Code must be 6 digits"))]
    pub code: String
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CheckUsernameRequest {
    #[validate(length(min = 3, message = "Username must be at least 3 characters"))]
    pub username: String,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
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

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct GetUserDataResponseWrapper {
    pub user: UserDataResponse,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct UserDataResponse {
    pub id: i64,
    pub username: String,
    pub email: String,
    pub name: String,
    pub birthday: Option<String>
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateEventRequest {
    pub title: String,
    pub location: Option<String>,
    pub description: Option<String>,
    pub start_date_time: Option<String>,
    pub end_date_time: Option<String>,
    pub color: String
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateEventResponse {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub start_date_time: Option<String>, 
    pub end_date_time: Option<String>,
    pub color: String, 
    pub created_by: String,
    pub created_at: String,
    pub status: String
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct GetEventRequest {
    pub event_id: i64,
    pub user_id: i64
} 

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct GetEventDetailedResponse {
    pub event: Events,
    pub invite_url: Option<String>,
    pub members: Vec<EventParticipant>,
    pub permissions: String
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct GetEventsResponse {
    pub events: Vec<Events>
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct JoinEventRequest {
    pub event_id: i64,
    pub invite_token: String,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct UpdateUserPermissionsRequest {
    pub event_id: i64,
    pub user_id: i64,
    pub new_permissions: i32
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct InviteUserToEventRequest {
    pub event_id: i64,
    pub user_id: i64,
    pub permissions: i32
}


#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct SuccessResponse {
    pub success: bool
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct ErrorResponse {
    pub error: String
}    

// plaining modules

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct CreatePollRequest {
    pub event_id: i64,
    pub question: String,
    pub options: Vec<String>,
    pub more_than_one_vote: bool
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct UpdatePollRequest {
    pub event_id: i64,
    pub poll_id: i64,
    pub question: String,
}

#[derive(Debug, Deserialize, Serialize, Validate, ToSchema)]
pub struct VotePollRequest {
    pub event_id: i64,
    pub poll_id: i64,
    #[validate(length(min = 1, message = "At least one option"))]
    pub option_indexes: Vec<i64>,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct DeletePollRequest {
    pub event_id: i64,
    pub poll_id: i64,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct PollResponse {
    pub poll_id: i64
}

#[derive(Debug, Deserialize, Serialize, Validate, ToSchema)]
pub struct CreateItemListRequest { 
    pub event_id: i64,
    pub title: String,
    #[validate(length(min = 1, message = "Items cannot be empty"))]
    pub items: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct ItemListResponse {
    pub item_list_id: i64
}

#[derive(Debug, Deserialize, Serialize, Validate, ToSchema)]
pub struct UpdateItemsListRequest {
    pub event_id: i64,
    pub item_list_id: i64,
    pub add: Option<Vec<String>>,
    pub remove: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct AssignItemRequest {
    pub event_id: i64,
    pub item_list_id: i64,
    pub assign: bool,   // true - забронировать, false - отказаться
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct DeleteItemListRequest {
    pub event_id: i64,
    pub item_list_id: i64,
}

#[derive(Debug, Deserialize, Serialize, Validate, ToSchema)]
pub struct CreateTaskListRequest {
    pub event_id: i64,
    pub title: String,
    #[validate(length(min = 1, message = "Tasks cannot be empty"))]
    pub tasks: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Validate, ToSchema)]
pub struct CreateTaskListResponse {
    pub task_list_id: i64
}

#[derive(Debug, Deserialize, Serialize, Validate, ToSchema)]
pub struct DeleteTaskListResponse {
    pub task_list_id: i64,
    pub event_id: i64
}

#[derive(Debug, Deserialize, Serialize, Validate, ToSchema)]
pub struct UpdateTaskListRequest {
    pub event_id: i64,
    pub task_list_id: i64,
    pub add: Option<Vec<String>>,
    pub remove: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct AssignTaskRequest {
    pub event_id: i64,
    pub task_list_id: i64,
    pub assign: bool,   // true - забронировать, false - отказаться
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct CompleteTaskRequest {
    pub event_id: i64,
    pub task_list_id: i64,
    pub completed: bool,
}
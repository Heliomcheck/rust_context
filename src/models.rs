use serde::{Deserialize, Serialize};
use serde_json::json;
use validator::{self, Validate, ValidationErrors};
use utoipa::{ToSchema};
use chrono::{DateTime, Utc};

use crate::{
    structs::*,
    errors::AppError
};

pub fn validation_errors_to_response(errors: ValidationErrors) -> AppError {
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

    AppError::BadRequest(serde_json::to_string(&error_map)
        .unwrap_or_else(|_| "Validation error".to_string()))
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
    #[validate(length(min = 10, max = 10,
        message = "Birthday format must be xx-xx-xxxx"))]
    pub birthday: Option<String>,
    #[validate(length(min = 1, max = 100,
        message = "Display name cannot be empty"))]
    pub display_name: Option<String>,
    #[validate(length(max = 100, 
        message = "Description length can't be more than 100 characters"))]
    pub description: Option<String>
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct GetUserDataResponseWrapper {
    pub user: UserDataResponse,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct UserDataResponse {
    pub user_id: i64,
    pub username: String,
    pub email: String,
    pub name: String,
    pub birthday: Option<String>
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateEventRequest {
    pub title: String,
    pub location: Option<String>,
    pub description_event: Option<String>,
    pub start_date_time: Option<String>,
    pub end_date_time: Option<String>,
    pub color: String
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateEventResponse {
    pub event_id: String,
    pub title: String,
    pub description_event: Option<String>,
    pub location: Option<String>,
    pub start_date_time: Option<String>, 
    pub end_date_time: Option<String>,
    pub color: String, 
    pub created_by: String,
    pub created_at: String,
    pub status_event: String
}


#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct EventPaths {
    pub event_id: i64,
    pub user_id: i64
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct EventModulesPaths {
    pub event_id: i64,
    pub module_id: i64
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct GetEventDetailedResponse {
    pub event: Events,
    pub invite_link: Option<String>,
    pub members: Vec<EventParticipant>,
    pub permissions: String
}
#[derive(Debug, Deserialize, Serialize, ToSchema, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct GetEvents {
    pub status: String,
    pub limit: i64,
    pub offset: i64
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct GetEventsResponse {
    pub events: Vec<Events>
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[allow(dead_code)]
pub struct UpdateStatusEventRequest {
    pub status: String
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct UpdateEventRequest {
    pub title: Option<String>,
    pub location: Option<String>,
    pub description_event: Option<String>,
    pub start_date_time: Option<String>,
    pub end_date_time: Option<String>,
    pub color: Option<String>
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct JoinEventRequest {
    pub event_id: i64,
    pub invite_token: String,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct UpdateUserPermissionsRequest {
    pub user_id: i64,
    pub new_permissions: String
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[allow(dead_code)]
pub struct EventMembers {
    pub user_id: i64,
    pub username: String,
    pub permissions: i32,
    pub joined_at: DateTime<Utc>
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct InviteUserToEventRequest {
    pub invite_token: String
}

#[derive(Debug, Deserialize, Serialize, ToSchema, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct EventModule {
    pub event_id: i64,
    pub module_id: i64
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct UpdateEventStatusRequest {
    pub status: String // active or archived
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
    pub title: String,
    pub options: Vec<String>,
    pub multiple_choice: bool
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct UpdatePollRequest {
    pub question: String,
}

#[derive(Debug, Deserialize, Serialize, Validate, ToSchema)]
pub struct VotePollRequest {
    pub poll_id: i64,
    #[validate(length(min = 1, message = "At least one option"))]
    pub option_indexes: Vec<i32>,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct DeletePollRequest {
    pub poll_id: i64,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct PollResponse {
    pub poll_id: i64
}

#[derive(Debug, Deserialize, Serialize, Validate, ToSchema)]
pub struct CreateItemListRequest { 
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
    pub item_list_id: i64,
    pub add: Option<Vec<String>>,
    pub remove: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct AssignItemRequest {
    pub item_list_id: i64,
    pub assign: bool,   // true - забронировать, false - отказаться
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct DeleteItemListRequest {
    pub item_list_id: i64,
}

#[derive(Debug, Deserialize, Serialize, Validate, ToSchema)]
pub struct CreateTaskListRequest {
    pub title: String,
    #[validate(length(min = 1, message = "Tasks cannot be empty"))]
    pub items: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Validate, ToSchema)]
pub struct CreateTaskListResponse {
    pub task_list_id: i64
}

#[derive(Debug, Deserialize, Serialize, Validate, ToSchema)]
pub struct DeleteTaskListResponse {
    pub task_list_id: i64
}

#[derive(Debug, Deserialize, Serialize, Validate, ToSchema)]
pub struct UpdateTaskListRequest {
    pub task_list_id: i64,
    pub add: Option<Vec<String>>,
    pub remove: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct AssignTaskRequest {
    pub task_list_id: i64,
    pub task_id: i64,
    pub assign: bool,   // true - забронировать, false - отказаться
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct CompleteTaskRequest {
    pub task_list_id: i64,
    pub task_id: i64,
    pub completed: bool,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct PathParams {
    pub first_id: i32,
    pub second_id: i32,
    pub third_id: i32
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PlanningModulesResponse {
    pub modules: Vec<PlanningModule>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type")]
pub enum PlanningModule {
    #[serde(rename = "poll")]
    Poll {
        id: String,
        title: String,
        data: PollModuleData,
    },
    #[serde(rename = "item_list")]
    ItemList {
        id: String,
        title: String,
        data: ItemListModuleData,
    },
    #[serde(rename = "task_list")]
    TaskList {
        id: String,
        title: String,
        data: TaskListModuleData,
    },
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PollModuleData {
    pub options: Vec<String>,
    pub multiple_choice: bool,
    pub votes: Vec<PollVote>,
    pub votes_count: Vec<i32>,
    pub own_vote: Vec<i32>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PollVote {
    pub option_index: i32,
    pub user_id: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ItemListModuleData {
    pub items: Vec<ItemListItemData>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ItemListItemData {
    pub id: String,
    pub text: String,
    pub assigned_user_id: Option<String>,
    pub assigned_user_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TaskListModuleData {
    pub items: Vec<TaskListItemData>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TaskListItemData {
    pub id: String,
    pub text: String,
    pub assigned_user_id: Option<String>,
    pub assigned_user_name: Option<String>,
    pub completed: bool,
}
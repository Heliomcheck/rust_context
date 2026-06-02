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
    pub display_name: Option<String>,
    pub birthday: Option<String>,
    pub description: Option<String>
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
    pub task_id: i64,
    pub assign: bool,   // true - забронировать, false - отказаться
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct CompleteTaskRequest {
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
// Альбомы

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateAlbumRequest {
    #[validate(length(min = 1, max = 200, message = "Title must be between 1 and 200 characters"))]
    pub title: String,
    #[validate(length(max = 1000))]
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]  // <-- добавить Deserialize
pub struct AlbumResponse {
    pub album_id: i64,
    pub event_id: i64,
    pub title: String,
    pub description: Option<String>,
    pub created_by: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]  // <--
pub struct AlbumWithPhotosResponse {
    pub album_id: i64,
    pub event_id: i64,
    pub title: String,
    pub description: Option<String>,
    pub created_by: i64,
    pub created_at: DateTime<Utc>,
    pub photos: Vec<PhotoResponse>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]  // <--
pub struct PhotoResponse {
    pub photo_id: i64,
    pub file_name: String,
    pub original_name: Option<String>,
    pub mime_type: Option<String>,
    pub file_size: Option<i64>,
    pub uploaded_by: i64,
    pub created_at: DateTime<Utc>,
    pub url: String,
}
#[cfg(test)]
mod tests {
    use super::*;
    use validator::Validate;

    #[test]
    fn register_request_valid() {
        let req = RegisterRequest {
            email: "user@example.com".into(),
            username: "validuser".into(),
            birthday: Some("01-01-2000".into()),
            display_name: "User".into(),
            description: Some("Hello".into()),
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn register_request_invalid_email() {
        let req = RegisterRequest {
            email: "notanemail".into(),
            username: "valid".into(),
            birthday: None,
            display_name: "Name".into(),
            description: None,
        };
        let errors = req.validate().unwrap_err();
        assert!(errors.field_errors().contains_key("email"));
    }

    #[test]
    fn register_request_short_username() {
        let req = RegisterRequest {
            email: "x@x.com".into(),
            username: "ab".into(),
            birthday: None,
            display_name: "Name".into(),
            description: None,
        };
        let errors = req.validate().unwrap_err();
        assert!(errors.field_errors().contains_key("username"));
    }

    #[test]
    fn register_request_invalid_birthday_format() {
        let req = RegisterRequest {
            email: "x@x.com".into(),
            username: "valid".into(),
            birthday: Some("2000-01-01".into()), // wrong format
            display_name: "Name".into(),
            description: None,
        };
        let errors = req.validate().unwrap_err();
        assert!(errors.field_errors().contains_key("birthday"));
    }

    #[test]
    fn code_request_valid() {
        let req = CodeRequest { email: "a@b.com".into() };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn verify_code_request_valid() {
        let req = VerifyCodeRequest { email: "a@b.com".into(), code: "123456".into() };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn verify_code_request_short_code() {
        let req = VerifyCodeRequest { email: "a@b.com".into(), code: "123".into() };
        assert!(req.validate().is_err());
    }

    #[test]
    fn edit_user_request_partial() {
        let req = EditUserRequest {
            username: Some("newuser".into()),
            birthday: None,
            display_name: None,
            description: None,
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn create_poll_request_valid() {
        let _ = CreatePollRequest {
            title: "Question".into(),
            options: vec!["A".into(), "B".into()],
            multiple_choice: false,
        };
        // Валидация не реализована для CreatePollRequest, поэтому просто проверяем, что не падает
        // (нет макроса Validate)
        // Если бы была, можно было бы вызвать validate()
    }

    #[test]
    fn validation_errors_to_response_bad_request() {
        // Симулируем ошибку валидации
        let mut errors = ValidationErrors::new();
        errors.add("username", validator::ValidationError::new("length"));
        let app_err = validation_errors_to_response(errors);
        match app_err {
            AppError::BadRequest(msg) => assert!(msg.contains("username")),
            _ => panic!("Expected BadRequest"),
        }
    }
}
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use validator::{self, Validate, ValidationErrors};
use axum::http::StatusCode;
use axum::Json;

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
    pub display_name: String
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
    #[validate(length(min = 32, max = 32))]
    pub token: String,
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
    pub avatar_url: Option<String>
}

#[derive(Debug, Serialize)]
pub struct UserDataResponse {
    pub id: i64,
    pub username: String,
    pub email: String,
    pub name: String,
    pub birthday: Option<String>
}
//test
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]// Проверяет, что ошибки валидации корректно преобразуются в HTTP ответ
    fn test_validation_error_response() {
        let mut errors = ValidationErrors::new();
        errors.add("email", validator::ValidationError::new("invalid"));
        let (status, body) = validation_errors_to_response(errors);
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }
}
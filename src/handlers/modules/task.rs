use axum::response::IntoResponse;
use axum::extract::State;
use std::sync::Arc;
use axum::Json;
use axum::http::StatusCode;
use serde_json::json;
use std::result::Result;
use axum_extra::TypedHeader;
use headers::{Authorization, authorization::Bearer};

use crate::{
    models::*,
    errors::AppError,
    structs::*,
};


#[utoipa::path(
    post,
    path = "/modules/task_list/create_item_list",
    tag = "Modules",
    security(
        ("bearerAuth" = [])
    ),
    request_body = CreateTaskListRequest,
    responses(
        (status = 201, description = "Task list created", body = CreateTaskListResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 403, description = "User doesn't have permission to invite or not in event", body = ErrorResponse),
        (status = 404, description = "User or event not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn create_task_list_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    Json(payload): Json<CreateTaskListRequest>
) -> Result<impl IntoResponse, AppError> {

    let task_list_id = 20;

    Ok((StatusCode::CREATED, Json(json!(CreateTaskListResponse { task_list_id }))))
}

#[utoipa::path(
    patch,
    path = "/modules/task_list/update_item_list",
    tag = "Modules",
    security(
        ("bearerAuth" = [])
    ),
    request_body = UpdateTaskListRequest,
    responses(
        (status = 204, description = "Task list updated", body = SuccessResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 403, description = "User doesn't have permission to invite or not in event", body = ErrorResponse),
        (status = 404, description = "User or event not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn update_task_list_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    Json(payload): Json<UpdateTaskListRequest>
) -> Result<impl IntoResponse, AppError> {

    let task_list_id = 20;

    Ok((StatusCode::NO_CONTENT, Json(json!(SuccessResponse { success: true }))))
}

#[utoipa::path(
    post,
    path = "/modules/task_list/assign_task",
    tag = "Modules",
    security(
        ("bearerAuth" = [])
    ),
    request_body = AssignTaskRequest,
    responses(
        (status = 204, description = "Task list assigned", body = SuccessResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 403, description = "User doesn't have permission to invite or not in event", body = ErrorResponse),
        (status = 404, description = "User or event not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn assign_task_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    Json(payload): Json<AssignTaskRequest>
) -> Result<impl IntoResponse, AppError> {

    let task_list_id = 20;

    Ok((StatusCode::NO_CONTENT, Json(json!(SuccessResponse { success: true }))))
}

#[utoipa::path(
    post,
    path = "/modules/task_list/complete_task",
    tag = "Modules",
    security(
        ("bearerAuth" = [])
    ),
    request_body = CompleteTaskRequest,
    responses(
        (status = 204, description = "Task list completed", body = SuccessResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 403, description = "User doesn't have permission to invite or not in event", body = ErrorResponse),
        (status = 404, description = "User or event not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn complete_task_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    Json(payload): Json<CompleteTaskRequest>
) -> Result<impl IntoResponse, AppError> {

    let task_list_id = 20;

    Ok((StatusCode::NO_CONTENT, Json(json!(SuccessResponse { success: true }))))
}

#[utoipa::path(
    post,
    path = "/modules/task_list/delete_task_list",
    tag = "Modules",
    security(
        ("bearerAuth" = [])
    ),
    request_body = DeleteTaskListResponse,
    responses(
        (status = 204, description = "Task list deleted", body = SuccessResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 403, description = "User doesn't have permission to invite or not in event", body = ErrorResponse),
        (status = 404, description = "User or event not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn delete_task_list_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    Json(payload): Json<DeleteTaskListResponse>
) -> Result<impl IntoResponse, AppError> {

    let task_list_id = 20;

    Ok((StatusCode::NO_CONTENT, Json(json!(SuccessResponse { success: true }))))
}
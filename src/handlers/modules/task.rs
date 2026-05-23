use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use axum_extra::extract::TypedHeader;
use headers::Authorization;
use std::sync::Arc;

use crate::{
    AppState,
    errors::AppError,
    models::{
        CreateTaskListRequest,
        UpdateTaskListRequest,
        AssignTaskRequest,
        CompleteTaskRequest,
        SuccessResponse,
    },
    data_base::plainning_modules::task_db::{
        create_task_list,
        get_task_list,
        update_task_list,
        assign_task,
        complete_task,
        delete_task_list,
        verify_task_list_in_event,
    },
    data_base::event_db::{check_user_in_event, has_permission},
    data_base::user_db::find_user_by_token,
};
use headers::authorization::Bearer;
use axum::{Router, extract::ws::{WebSocket, WebSocketUpgrade}, response::IntoResponse, routing::{self, trace}
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
    Path(event_id): Path<i64>,
    Json(payload): Json<CreateTaskListRequest>,
) -> Result<impl IntoResponse, AppError> {
    let token = auth.token().to_string();
    let user = find_user_by_token(&state.db_pool, &token)
        .await?
        .ok_or(AppError::Unauthorized)?;
    let user_id = user.user_id;

    let is_in_event = check_user_in_event(&state.db_pool, event_id, user_id).await?;
    if !is_in_event {
        return Err(AppError::Forbidden("User not in event".to_string()));
    }

    let has_perm = has_permission(&state.db_pool, event_id, user_id, 2).await?;
    if !has_perm {
        return Err(AppError::Forbidden("No permission to create task list".to_string()));
    }

    let task_list = create_task_list(
        &state.db_pool,
        event_id,
        &payload.title,
        &payload.tasks,
        user_id,
    )
    .await?;

    Ok((StatusCode::CREATED, Json(task_list)))
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

    Ok((StatusCode::NO_CONTENT, Json(SuccessResponse { success: true })))
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

    Ok((StatusCode::NO_CONTENT, Json(SuccessResponse { success: true })))
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

    Ok((StatusCode::NO_CONTENT, Json(SuccessResponse { success: true })))
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
    Path((event_id, module_id)): Path<(i64, i64)>,
) -> Result<impl IntoResponse, AppError> {
    let token = auth.token().to_string();
    let user = find_user_by_token(&state.db_pool, &token)
        .await?
        .ok_or(AppError::Unauthorized)?;
    let user_id = user.user_id;

    let has_perm = has_permission(&state.db_pool, event_id, user_id, 4).await?;
    if !has_perm {
        return Err(AppError::Forbidden("No permission to delete task list".to_string()));
    }

    delete_task_list(&state.db_pool, module_id, event_id).await?;

    Ok((StatusCode::NO_CONTENT, Json(SuccessResponse { success: true })))
}
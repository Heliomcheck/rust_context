use axum::{
    extract::{Path, State, Query},
    http::StatusCode,
    response::Json,
    response::IntoResponse
};
use axum_extra::extract::TypedHeader;
use headers::{
    Authorization,
    authorization::Bearer
};
use std::sync::Arc;

use crate::{
    AppState, data_base::{event_db::*, plainning_modules::task_db::*, user_db::*}, errors::AppError, models::*, permissions::EventPermissions, *
};


#[utoipa::path(
    post,
    path = "/events/{event_id}/planning/task_list",
    tag = "Modules",
    security(
        ("bearerAuth" = [])
    ),
    request_body = CreateTaskListRequest,
    responses(
        (status = 201, description = "Task list created", body = SuccessResponse),
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

    let _ = create_task_list(
        &state.db_pool,
        event_id,
        &payload.title,
        &payload.items,
        user_id,
    )
    .await?;

    Ok((StatusCode::OK, Json(SuccessResponse { success: true })))
}

#[utoipa::path(
    patch,
    path = "/events/{event_id}/planning/task_list/{module_id}",
    tag = "Modules",
    security(
        ("bearerAuth" = [])
    ),
    request_body = UpdateTaskListRequest,
    responses(
        (status = 200, description = "Task list updated", body = SuccessResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 403, description = "User doesn't have permission or not in event", body = ErrorResponse),
        (status = 404, description = "Task list or event not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn update_task_list_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    query: Query<EventModule>,
    Json(payload): Json<UpdateTaskListRequest>,
) -> Result<impl IntoResponse, AppError> {
    let user = get_user_for_handler_from_token(&state.db_pool, auth.token()).await?;
    
    let is_in_event = check_user_in_event(&state.db_pool, query.event_id, user.user_id).await?;
    if !is_in_event {
        return Err(AppError::UserNotInEvent("User not in event".to_string()));
    }
    
    let has_permission = has_permission(&state.db_pool, query.event_id, user.user_id, EventPermissions::MEMBER).await?;
    if !has_permission {
        return Err(AppError::Forbidden("Not enough permissions to update task list".to_string()));
    }
    
    let belongs = verify_task_list_in_event(&state.db_pool, payload.task_list_id, query.event_id).await?;
    if !belongs {
        return Err(AppError::NotFound("Task list not found in this event".to_string()));
    }
    
    let add_tasks = payload.add.unwrap_or_default();
    
    let remove_task_ids: Vec<i64> = payload // convert data
        .remove
        .unwrap_or_default()
        .into_iter()
        .filter_map(|id| id.parse::<i64>().ok())
        .collect();
    
    let _ = update_task_list(
        &state.db_pool,
        payload.task_list_id,
        &add_tasks,
        &remove_task_ids,
    )
    .await?;
    
    Ok((StatusCode::OK, Json(SuccessResponse { success: true })))
}

#[utoipa::path(
    patch,
    path = "/events/{event_id}/planning/task_list/{module_id}/tasks/{task_id}/assign",
    tag = "Modules",
    security(
        ("bearerAuth" = [])
    ),
    request_body = AssignTaskRequest,
    responses(
        (status = 200, description = "Task assigned/unassigned successfully", body = SuccessResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 403, description = "User doesn't have permission or not in event", body = ErrorResponse),
        (status = 404, description = "Task or event not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn assign_task_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    Path((event_id, module_id, task_list_id)): Path<(i64, i64, i64)>,
    Json(payload): Json<AssignTaskRequest>
) -> Result<impl IntoResponse, AppError> {
    let user = get_user_for_handler_from_token(&state.db_pool, auth.token()).await?;
    
    let is_in_event = check_user_in_event(&state.db_pool, event_id, user.user_id).await?;
    if !is_in_event {
        return Err(AppError::UserNotInEvent("User not in event".to_string()));
    }
    
    let belongs = verify_task_list_in_event(&state.db_pool, task_list_id, event_id).await?;
    if !belongs {
        return Err(AppError::NotFound("Task list not found in this event".to_string()));
    }
    
    assign_task(
        &state.db_pool,
        task_list_id,
        user.user_id,
        payload.assign,
    )
    .await?;
    
    let _ = get_task_list(&state.db_pool, task_list_id)
        .await?
        .ok_or(AppError::NotFound("Task list not found".to_string()))?;
    
    Ok((StatusCode::OK, Json(SuccessResponse { success: true })))
}

#[utoipa::path(
    patch,
    path = "/events/{event_id}/planning/task_list/{module_id}/tasks/{task_list_id}/complete",
    tag = "Modules",
    security(
        ("bearerAuth" = [])
    ),
    request_body = CompleteTaskRequest,
    responses(
        (status = 200, description = "Task completed/uncompleted successfully", body = SuccessResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 403, description = "User doesn't have permission or not in event", body = ErrorResponse),
        (status = 404, description = "Task or event not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn complete_task_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    Path((event_id, module_id, task_list_id)): Path<(i64, i64, i64)>,
    Json(payload): Json<CompleteTaskRequest>,
) -> Result<impl IntoResponse, AppError> {
    let user = get_user_for_handler_from_token(&state.db_pool, auth.token()).await?;
    
    let is_in_event = check_user_in_event(&state.db_pool, event_id, user.user_id).await?;
    if !is_in_event {
        return Err(AppError::UserNotInEvent("User not in event".to_string()));
    }
    
    let belongs = verify_task_list_in_event(&state.db_pool, task_list_id, event_id).await?;
    if !belongs {
        return Err(AppError::NotFound("Task list not found in this event".to_string()));
    }
    
    complete_task(
        &state.db_pool,
        payload.task_id,
        user.user_id,
        payload.completed,
    )
    .await?;
    
    let _ = get_task_list(&state.db_pool, task_list_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Task list not found".to_string()))?;

    Ok((StatusCode::OK, Json(SuccessResponse { success: true })))
}

#[utoipa::path(
    post,
    path = "/events/{event_id}/planning/task_list/{module_id}",
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

    Ok((StatusCode::OK, Json(SuccessResponse { success: true })))
}
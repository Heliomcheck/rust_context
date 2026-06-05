use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    response::IntoResponse
};
use axum_extra::extract::TypedHeader;
use headers::{Authorization, authorization::Bearer};
use std::sync::Arc;

use crate::{
    data_base::{event_db::*, plainning_modules::task_db::*, user_db::*},
    errors::AppError,
    models::*,
    structs::*,
    *,
};

// ====================== 1. Создание списка задач ======================
#[utoipa::path(
    post,
    path = "/events/{event_id}/planning/task_list",
    tag = "Modules",
    security(("bearerAuth" = [])),
    request_body = CreateTaskListRequest,
    responses(
        (status = 201, description = "Task list created", body = CreateTaskListResponse),
        (status = 400, description = "Bad request"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Not found")
    )
)]
pub async fn create_task_list_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    Path(event_id): Path<i64>,
    Json(payload): Json<CreateTaskListRequest>,
) -> Result<impl IntoResponse, AppError> {
    let token = auth.token().to_string();
    let user = find_user_by_token(&state.db_pool, &token).await?.ok_or(AppError::Unauthorized)?;
    let user_id = user.user_id;

    let is_in_event = check_user_in_event(&state.db_pool, event_id, user_id).await?;
    if !is_in_event {
        return Err(AppError::Forbidden("User not in event".into()));
    }

    let has_perm = has_permission(&state.db_pool, event_id, user_id, 2).await?;
    if !has_perm {
        return Err(AppError::Forbidden("No permission to create task list".into()));
    }

    let task_list = create_task_list(&state.db_pool, event_id, &payload.title, &payload.items, user_id).await?;
    Ok((StatusCode::CREATED, Json(task_list)))
}

// ====================== 2. Обновление списка задач ======================
#[utoipa::path(
    patch,
    path = "/events/{event_id}/planning/task_list/{module_id}",
    tag = "Modules",
    security(("bearerAuth" = [])),
    request_body = UpdateTaskListRequest,
    responses(
        (status = 200, description = "Task list updated", body = TaskListWithItems),
        (status = 400, description = "Bad request"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Not found")
    )
)]
pub async fn update_task_list_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    Path((event_id, module_id)): Path<(i64, i64)>,
    Json(payload): Json<UpdateTaskListRequest>,
) -> Result<impl IntoResponse, AppError> {
    let token = auth.token().to_string();
    let user = find_user_by_token(&state.db_pool, &token).await?.ok_or(AppError::Unauthorized)?;
    let user_id = user.user_id;

    let is_in_event = check_user_in_event(&state.db_pool, event_id, user_id).await?;
    if !is_in_event {
        return Err(AppError::Forbidden("User not in event".into()));
    }

    let has_perm = has_permission(&state.db_pool, event_id, user_id, 2).await?;
    if !has_perm {
        return Err(AppError::Forbidden("No permission to update task list".into()));
    }

    let belongs = verify_task_list_in_event(&state.db_pool, module_id, event_id).await?;
    if !belongs {
        return Err(AppError::NotFound("Task list not found in this event".into()));
    }

    let add = payload.add.unwrap_or_default();
    let remove = payload.remove.unwrap_or_default()
        .into_iter()
        .map(|id| id.parse::<i64>())
        .collect::<Result<Vec<i64>, _>>()
        .map_err(|_| AppError::BadRequest("Invalid task id format".into()))?;

    let updated = update_task_list(&state.db_pool, module_id, &add, &remove).await?;
    Ok((StatusCode::OK, Json(updated)))
}

// ====================== 3. Назначение/снятие ответственного за задачу ======================
#[utoipa::path(
    patch,
    path = "/events/{event_id}/planning/task_list/{module_id}/items/{task_id}/assign",
    tag = "Modules",
    security(("bearerAuth" = [])),
    request_body = AssignTaskRequest,
    responses(
        (status = 200, description = "Task assigned/unassigned", body = TaskListWithItems),
        (status = 400, description = "Bad request"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Not found")
    )
)]
pub async fn assign_task_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    Path((event_id, module_id, task_id)): Path<(i64, i64, i64)>,
    Json(payload): Json<AssignTaskRequest>,
) -> Result<impl IntoResponse, AppError> {
    let token = auth.token().to_string();
    let user = find_user_by_token(&state.db_pool, &token).await?.ok_or(AppError::Unauthorized)?;
    let user_id = user.user_id;

    let is_in_event = check_user_in_event(&state.db_pool, event_id, user_id).await?;
    if !is_in_event {
        return Err(AppError::Forbidden("User not in event".into()));
    }

    let belongs = verify_task_list_in_event(&state.db_pool, module_id, event_id).await?;
    if !belongs {
        return Err(AppError::NotFound("Task list not found in this event".into()));
    }

    // Проверяем, что задача принадлежит этому списку
    if !verify_task_in_list(&state.db_pool, task_id, module_id).await? {
        return Err(AppError::NotFound("Task not found in this list".into()));
    }

    assign_task(&state.db_pool, task_id, user_id, payload.assign).await?;

    let updated = get_task_list(&state.db_pool, module_id).await?.ok_or(AppError::NotFound("Task list not found".into()))?;
    Ok((StatusCode::OK, Json(updated)))
}

// ====================== 4. Отметка о выполнении задачи ======================
#[utoipa::path(
    patch,
    path = "/events/{event_id}/planning/task_list/{module_id}/items/{task_id}/complete",
    tag = "Modules",
    security(("bearerAuth" = [])),
    request_body = CompleteTaskRequest,
    responses(
        (status = 200, description = "Task completion toggled", body = TaskListWithItems),
        (status = 400, description = "Bad request"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Not found")
    )
)]
pub async fn complete_task_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    Path((event_id, module_id, task_id)): Path<(i64, i64, i64)>,
    Json(payload): Json<CompleteTaskRequest>,
) -> Result<impl IntoResponse, AppError> {
    let token = auth.token().to_string();
    let user = find_user_by_token(&state.db_pool, &token).await?.ok_or(AppError::Unauthorized)?;
    let user_id = user.user_id;

    let is_in_event = check_user_in_event(&state.db_pool, event_id, user_id).await?;
    if !is_in_event {
        return Err(AppError::Forbidden("User not in event".into()));
    }

    let belongs = verify_task_list_in_event(&state.db_pool, module_id, event_id).await?;
    if !belongs {
        return Err(AppError::NotFound("Task list not found in this event".into()));
    }

    if !verify_task_in_list(&state.db_pool, task_id, module_id).await? {
        return Err(AppError::NotFound("Task not found in this list".into()));
    }

    complete_task(&state.db_pool, task_id, user_id, payload.completed).await?;

    let updated = get_task_list(&state.db_pool, module_id).await?.ok_or(AppError::NotFound("Task list not found".into()))?;
    Ok((StatusCode::OK, Json(updated)))
}

// ====================== 5. Удаление списка задач ======================
#[utoipa::path(
    delete,
    path = "/events/{event_id}/planning/task_list/{module_id}",
    tag = "Modules",
    security(("bearerAuth" = [])),
    responses(
        (status = 204, description = "Task list deleted"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Not found")
    )
)]
pub async fn delete_task_list_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    Path((event_id, module_id)): Path<(i64, i64)>,
) -> Result<impl IntoResponse, AppError> {
    let token = auth.token().to_string();
    let user = find_user_by_token(&state.db_pool, &token).await?.ok_or(AppError::Unauthorized)?;
    let user_id = user.user_id;

    let has_perm = has_permission(&state.db_pool, event_id, user_id, 4).await?;
    if !has_perm {
        return Err(AppError::Forbidden("No permission to delete task list".into()));
    }

    delete_task_list(&state.db_pool, module_id, event_id).await?;
    Ok(StatusCode::NO_CONTENT)
}
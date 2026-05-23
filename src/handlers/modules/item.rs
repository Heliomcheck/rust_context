use axum::response::IntoResponse;
use axum::extract::{RawPathParams, State};
use serde::Deserialize;
use axum::extract::Path;
use std::sync::Arc;
use axum::Json;
use axum::http::StatusCode;
use serde_json::json;
use std::result::Result;
use axum_extra::TypedHeader;
use headers::{Authorization, authorization::Bearer};

use crate::structs::*;

use crate::{
    models::*,
    errors::AppError,
    data_base::{
        event_db::*,
        plainning_modules::item_db::*,
    },
    data_base::user_db::*,
    permissions::*,
};


#[utoipa::path(
    post,
    path = "/modules/item_list/create_item_list",
    tag = "Modules",
    security(
        ("bearerAuth" = [])
    ),
    request_body = CreateItemListRequest,
    responses(
        (status = 201, description = "Item list created", body = ItemListResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 403, description = "User doesn't have permission to invite or not in event", body = ErrorResponse),
        (status = 404, description = "User or event not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn create_item_list_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    Path(event_id): Path<i64>,
    Json(payload): Json<CreateItemListRequest>,
) -> Result<impl IntoResponse, AppError> {
    // 1. Проверяем токен и получаем user_id
    let token = auth.token().to_string();
    let user = find_user_by_token(&state.db_pool, &token)
        .await?
        .ok_or(AppError::Unauthorized)?;
    let user_id = user.user_id;

    // 2. Проверяем, что пользователь состоит в событии
    let is_in_event = check_user_in_event(&state.db_pool, event_id, user_id).await?;
    if !is_in_event {
        return Err(AppError::Forbidden("User not in event".to_string()));
    }

    // 3. Проверяем права на создание (нужны права на редактирование)
    let has_perm = has_permission(&state.db_pool, event_id, user_id, 2).await?; // 2 = EDIT_EVENT
    if !has_perm {
        return Err(AppError::Forbidden("No permission to create item list".to_string()));
    }

    // 4. Создаем item_list
    let item_list = create_item_list(
        &state.db_pool,
        event_id,
        &payload.title,
        &payload.items,
        user_id,
    )
    .await?;

    Ok((StatusCode::CREATED, Json(item_list)))
}

#[utoipa::path(
    patch,
    path = "/modules/item_list/update_item_list",
    tag = "Modules",
    security(
        ("bearerAuth" = [])
    ),
    request_body = UpdateItemsListRequest,
    responses(
        (status = 204, description = "Item list updated", body = SuccessResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 403, description = "User doesn't have permission to invite or not in event", body = ErrorResponse),
        (status = 404, description = "User or event not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn update_item_list_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    Path((event_id, module_id)): Path<(i64, i64)>,
    Json(payload): Json<UpdateItemsListRequest>,
) -> Result<impl IntoResponse, AppError> {
    // 1. Проверяем токен
    let token = auth.token().to_string();
    let user = find_user_by_token(&state.db_pool, &token)
        .await?
        .ok_or(AppError::Unauthorized)?;
    let user_id = user.user_id;

    // 2. Проверяем, что пользователь в событии
    let is_in_event = check_user_in_event(&state.db_pool, event_id, user_id).await?;
    if !is_in_event {
        return Err(AppError::Forbidden("User not in event".to_string()));
    }

    // 3. Проверяем права на редактирование
    let has_perm = has_permission(&state.db_pool, event_id, user_id, 2).await?;
    if !has_perm {
        return Err(AppError::Forbidden("No permission to update item list".to_string()));
    }

    // 4. Проверяем, что item_list принадлежит событию
    let belongs = verify_item_list_in_event(&state.db_pool, module_id, event_id).await?;
    if !belongs {
        return Err(AppError::NotFound("Not found".to_string()));
    }

    // 5. Обновляем
    let add = payload.add.unwrap_or_default();
    let remove = payload
        .remove
        .unwrap_or_default()
        .into_iter()
        .map(|id| id.parse::<i64>())
        .collect::<Result<Vec<i64>, _>>()
        .map_err(|_| AppError::BadRequest("Invalid item id format".to_string()))?;
    
    let updated = update_item_list(
        &state.db_pool,
        module_id,
        &add,
        &remove,
    )
    .await?;

    Ok((StatusCode::OK, Json(updated)))
}

#[utoipa::path(
    patch,
    path = "/modules/item_list/assign_item",
    tag = "Modules",
    security(
        ("bearerAuth" = [])
    ),
    request_body = AssignItemRequest,
    responses(
        (status = 204, description = "Item list assigned", body = SuccessResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 403, description = "User doesn't have permission to invite or not in event", body = ErrorResponse),
        (status = 404, description = "User or event not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn assign_item_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    Path((event_id, module_id, item_id)): Path<(i64, i64, i64)>,
    Json(payload): Json<AssignItemRequest>,
) -> Result<impl IntoResponse, AppError> {
    // 1. Проверяем токен
    let token = auth.token().to_string();
    let user = find_user_by_token(&state.db_pool, &token)
        .await?
        .ok_or(AppError::Unauthorized)?;
    let user_id = user.user_id;

    // 2. Проверяем, что пользователь в событии
    let is_in_event = check_user_in_event(&state.db_pool, event_id, user_id).await?;
    if !is_in_event {
        return Err(AppError::Forbidden("User not in event".to_string()));
    }

    // 3. Проверяем, что item_list принадлежит событию
    let belongs = verify_item_list_in_event(&state.db_pool, module_id, event_id).await?;
    if !belongs {
        return Err(AppError::NotFound("Not found".to_string()));
    }

    // 4. Бронируем/отменяем
    assign_item(&state.db_pool, item_id, user_id, payload.assign).await?;

    // 5. Возвращаем обновленный модуль
    let updated = get_item_list(&state.db_pool, module_id)
        .await?
        .ok_or(AppError::NotFound("Not found".to_string()))?;

    Ok((StatusCode::OK, Json(updated)))
}

#[utoipa::path(
    post,
    path = "/modules/item_list/delete_item_list",
    tag = "Modules",
    security(
        ("bearerAuth" = [])
    ),
    request_body = DeleteItemListRequest,
    responses(
        (status = 204, description = "Item list deleted", body = SuccessResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 403, description = "User doesn't have permission to invite or not in event", body = ErrorResponse),
        (status = 404, description = "User or event not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn delete_item_list_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    Path((event_id, module_id)): Path<(i64, i64)>,
) -> Result<impl IntoResponse, AppError> {
    // 1. Проверяем токен
    let token = auth.token().to_string();
    let user = find_user_by_token(&state.db_pool, &token)
        .await?
        .ok_or(AppError::Unauthorized)?;
    let user_id = user.user_id;

    // 2. Проверяем права на удаление
    let has_perm = has_permission(&state.db_pool, event_id, user_id, 4).await?; // 4 = DELETE_EVENT
    if !has_perm {
        return Err(AppError::Forbidden("No permission to delete item list".to_string()));
    }

    // 3. Удаляем
    delete_item_list(&state.db_pool, module_id, event_id).await?;

    Ok((StatusCode::NO_CONTENT, Json(SuccessResponse { success: true })))
}
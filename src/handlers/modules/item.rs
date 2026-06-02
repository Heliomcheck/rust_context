use axum::{
    response::IntoResponse,
    extract::State,
    extract::Path,
    Json,
    http::StatusCode
};
use std::{
    sync::Arc,
    result::Result
};
use axum_extra::TypedHeader;
use headers::{
    Authorization, 
    authorization::Bearer
};

use crate::permissions::EventPermissions;
use crate::structs::*;

use crate::{
    models::*,
    errors::AppError,
    data_base::{
        event_db::*,
        plainning_modules::item_db::*,
        user_db::*
    }
};


#[utoipa::path(
    post,
    path = "/events/{event_id}/planning/item_list",
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
    let token = auth.token().to_string();
    let user = find_user_by_token(&state.db_pool, &token)
        .await?
        .ok_or(AppError::Unauthorized)?;
    let user_id = user.user_id;

    let is_in_event = check_user_in_event(&state.db_pool, event_id, user_id).await?;
    if !is_in_event {
        return Err(AppError::Forbidden("User not in event".to_string()));
    }

    let has_perm = has_permission(&state.db_pool, event_id, user_id, EventPermissions::OWNER).await?; // 2 = EDIT_EVENT
    if !has_perm {
        return Err(AppError::Forbidden("No permission to create item list".to_string()));
    }

    let _ = create_item_list(
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
    path = "/events/{event_id}/planning/item_list/{module_id}",
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
    let token = auth.token().to_string();
    let user = find_user_by_token(&state.db_pool, &token)
        .await?
        .ok_or(AppError::Unauthorized)?;
    let user_id = user.user_id;

    let is_in_event = check_user_in_event(&state.db_pool, event_id, user_id).await?;
    if !is_in_event {
        return Err(AppError::Forbidden("User not in event".to_string()));
    }

    let has_perm = has_permission(&state.db_pool, event_id, user_id, EventPermissions::OWNER).await?;
    if !has_perm {
        return Err(AppError::Forbidden("No permission to update item list".to_string()));
    }

    let belongs = verify_item_list_in_event(&state.db_pool, module_id, event_id).await?;
    if !belongs {
        return Err(AppError::NotFound("Not found".to_string()));
    }

    let add = payload.add.unwrap_or_default();
    let remove = payload
        .remove
        .unwrap_or_default()
        .into_iter()
        .map(|id| id.parse::<i64>())
        .collect::<Result<Vec<i64>, _>>()
        .map_err(|_| AppError::BadRequest("Invalid item id format".to_string()))?;
    
    let _ = update_item_list(
        &state.db_pool,
        module_id,
        &add,
        &remove,
    )
    .await?;

    Ok((StatusCode::OK, Json(SuccessResponse { success: true })))
}

#[utoipa::path(
    patch,
    path = "/events/{event_id}/planning/item_list/{module_id}/items/{item_list_id}/assign",
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
    let token = auth.token().to_string();
    let user = find_user_by_token(&state.db_pool, &token)
        .await?
        .ok_or(AppError::Unauthorized)?;
    let user_id = user.user_id;

    let is_in_event = check_user_in_event(&state.db_pool, event_id, user_id).await?;
    if !is_in_event {
        return Err(AppError::Forbidden("User not in event".to_string()));
    }

    let belongs = verify_item_list_in_event(&state.db_pool, module_id, event_id).await?;
    if !belongs {
        return Err(AppError::NotFound("Not found".to_string()));
    }

    assign_item(&state.db_pool, item_id, user_id, payload.assign).await?;

    let _ = get_item_list(&state.db_pool, module_id)
        .await?
        .ok_or(AppError::NotFound("Not found".to_string()))?;

    Ok((StatusCode::OK, Json(SuccessResponse { success: true })))
}

#[utoipa::path(
    post,
    path = "/events/{event_id}/planning/item_list/{module_id}",
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
    let token = auth.token().to_string();
    let user = find_user_by_token(&state.db_pool, &token)
        .await?
        .ok_or(AppError::Unauthorized)?;
    let user_id = user.user_id;

    let has_perm = has_permission(&state.db_pool, event_id, user_id, EventPermissions::OWNER).await?; // 4 = DELETE_EVENT
    if !has_perm {
        return Err(AppError::Forbidden("No permission to delete item list".to_string()));
    }

    delete_item_list(&state.db_pool, module_id, event_id).await?;

    Ok((StatusCode::OK, Json(SuccessResponse {success: true})))
}
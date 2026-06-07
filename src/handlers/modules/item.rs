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
    security(("bearerAuth" = [])),
    request_body = CreateItemListRequest,
    responses(
        (status = 201, description = "Item list created", body = ItemListResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Not found")
    )
)]
pub async fn create_item_list_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    Path(event_id_str): Path<String>,
    Json(payload): Json<CreateItemListRequest>,
) -> Result<impl IntoResponse, AppError> {
    let event_id: i64 = event_id_str.parse().map_err(|_| AppError::BadRequest("Invalid event_id".into()))?;
    let token = auth.token().to_string();
    let user = find_user_by_token(&state.db_pool, &token).await?.ok_or(AppError::Unauthorized)?;
    let user_id = user.user_id;

    let is_in_event = check_user_in_event(&state.db_pool, event_id, user_id).await?;
    if !is_in_event {
        return Err(AppError::Forbidden("User not in event".into()));
    }

    let has_perm = has_permission(&state.db_pool, event_id, user_id, EventPermissions::OWNER).await?;
    if !has_perm {
        return Err(AppError::Forbidden("No permission to create item list".into()));
    }

    let item_list = create_item_list(&state.db_pool, event_id, &payload.title, &payload.items, user_id).await?;
    Ok((StatusCode::CREATED, Json(item_list)))
}

#[utoipa::path(
    patch,
    path = "/events/{event_id}/planning/item_list/{module_id}",
    tag = "Modules",
    security(("bearerAuth" = [])),
    request_body = UpdateItemsListRequest,
    responses(
        (status = 200, description = "Item list updated", body = ItemListWithItems),
        (status = 400, description = "Bad request"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Not found")
    )
)]
pub async fn update_item_list_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    Path((event_id_str, module_id_str)): Path<(String, String)>,
    Json(payload): Json<UpdateItemsListRequest>,
) -> Result<impl IntoResponse, AppError> {
    let event_id: i64 = event_id_str.parse().map_err(|_| AppError::BadRequest("Invalid event_id".into()))?;
    let module_id: i64 = module_id_str.parse().map_err(|_| AppError::BadRequest("Invalid module_id".into()))?;
    let token = auth.token().to_string();
    let user = find_user_by_token(&state.db_pool, &token).await?.ok_or(AppError::Unauthorized)?;
    let user_id = user.user_id;

    let is_in_event = check_user_in_event(&state.db_pool, event_id, user_id).await?;
    if !is_in_event {
        return Err(AppError::Forbidden("User not in event".into()));
    }

    let has_perm = has_permission(&state.db_pool, event_id, user_id, EventPermissions::OWNER).await?;
    if !has_perm {
        return Err(AppError::Forbidden("No permission to update item list".into()));
    }

    let belongs = verify_item_list_in_event(&state.db_pool, module_id, event_id).await?;
    if !belongs {
        return Err(AppError::NotFound("Not found".into()));
    }

    let add = payload.add.unwrap_or_default();
    let remove = payload.remove.unwrap_or_default()
        .into_iter()
        .map(|id| id.parse::<i64>())
        .collect::<Result<Vec<i64>, _>>()
        .map_err(|_| AppError::BadRequest("Invalid item id format".into()))?;

    let updated = update_item_list(&state.db_pool, module_id, &add, &remove).await?;
    Ok((StatusCode::OK, Json(updated)))
}

#[utoipa::path(
    patch,
    path = "/events/{event_id}/planning/item_list/{module_id}/items/{item_id}/assign",
    tag = "Modules",
    security(("bearerAuth" = [])),
    request_body = AssignItemRequest,
    responses(
        (status = 200, description = "Item assigned/unassigned", body = ItemListWithItems),
        (status = 400, description = "Bad request"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Not found")
    )
)]
pub async fn assign_item_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    Path((event_id_str, module_id_str, item_id_str)): Path<(String, String, String)>,
    Json(payload): Json<AssignItemRequest>,
) -> Result<impl IntoResponse, AppError> {
    let event_id: i64 = event_id_str.parse().map_err(|_| AppError::BadRequest("Invalid event_id".into()))?;
    let module_id: i64 = module_id_str.parse().map_err(|_| AppError::BadRequest("Invalid module_id".into()))?;
    let item_id: i64 = item_id_str.parse().map_err(|_| AppError::BadRequest("Invalid item_id".into()))?;
    let token = auth.token().to_string();
    let user = find_user_by_token(&state.db_pool, &token).await?.ok_or(AppError::Unauthorized)?;
    let user_id = user.user_id;

    let is_in_event = check_user_in_event(&state.db_pool, event_id, user_id).await?;
    if !is_in_event {
        return Err(AppError::Forbidden("User not in event".into()));
    }

    let belongs = verify_item_list_in_event(&state.db_pool, module_id, event_id).await?;
    if !belongs {
        return Err(AppError::NotFound("Item list not found in this event".into()));
    }

    if !verify_item_in_list(&state.db_pool, item_id, module_id).await? {
        return Err(AppError::NotFound("Item not found in this list".into()));
    }

    assign_item(&state.db_pool, item_id, user_id, payload.assign).await?;

    let updated = get_item_list(&state.db_pool, module_id).await?.ok_or(AppError::NotFound("Item list not found".into()))?;
    Ok((StatusCode::OK, Json(updated)))
}

#[utoipa::path(
    delete,
    path = "/events/{event_id}/planning/item_list/{module_id}",
    tag = "Modules",
    security(("bearerAuth" = [])),
    responses(
        (status = 204, description = "Item list deleted"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Not found")
    )
)]
pub async fn delete_item_list_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    Path((event_id_str, module_id_str)): Path<(String, String)>,
) -> Result<impl IntoResponse, AppError> {
    let event_id: i64 = event_id_str.parse().map_err(|_| AppError::BadRequest("Invalid event_id".into()))?;
    let module_id: i64 = module_id_str.parse().map_err(|_| AppError::BadRequest("Invalid module_id".into()))?;
    let token = auth.token().to_string();
    let user = find_user_by_token(&state.db_pool, &token).await?.ok_or(AppError::Unauthorized)?;
    let user_id = user.user_id;

    let has_perm = has_permission(&state.db_pool, event_id, user_id, EventPermissions::OWNER).await?;
    if !has_perm {
        return Err(AppError::Forbidden("No permission to delete item list".into()));
    }

    delete_item_list(&state.db_pool, module_id, event_id).await?;
    Ok(StatusCode::NO_CONTENT)
}
use axum::response::IntoResponse;
use axum::extract::State;
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
};


#[utoipa::path(
    post,
    path = "/modules/item_list/create_item_list",
    tag = "Modules",
    security(
        ("bearerAuth" = [])
    ),
    request_body = CreatePollRequest,
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
    Json(payload): Json<CreateItemListRequest>
) -> Result<impl IntoResponse, AppError> {

    let item_list_id = 20;
    
    Ok((StatusCode::CREATED, Json(json!(ItemListResponse { item_list_id }))))
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
    Json(payload): Json<UpdateItemsListRequest>
) -> Result<impl IntoResponse, AppError> {

    let item_list_id = 20;

    Ok((StatusCode::NO_CONTENT, Json(json!(SuccessResponse{success: true}))))
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
    Json(payload): Json<AssignItemRequest>
) -> Result<impl IntoResponse, AppError> {

    let item_list_id = 20;

    Ok((StatusCode::NO_CONTENT, Json(json!(SuccessResponse{success: true}))))
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
    Json(payload): Json<DeleteItemListRequest>
) -> Result<impl IntoResponse, AppError> {


    Ok((StatusCode::NO_CONTENT, Json(json!(SuccessResponse{success: true}))))
}
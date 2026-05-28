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
    path = "/events/{event_id}/planning/items",
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
    path = "/events/{event_id}/planning/items/{module_id}",
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
    path = "/events/{event_id}/planning/items/{module_id}/items/{item_id}/assign",
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
    path = "/events/{event_id}/planning/items/{module_id}",
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

//test
#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Router, body::Body, http::Request};
    use tower::ServiceExt;
    use std::sync::Arc;
    use tokio::sync::{Mutex, broadcast};
    use serde_json::json;

    use crate::{
        test_utils::setup_test_db,
        structs::AppState,
        user_store::UserStore,
        secrets::verification::VerificationStore,
        data_base::{
            user_db::{create_user_db, create_token},
            event_db::{create_event, add_member},
        },
        permissions::EventPermissions,
    };

    async fn setup(perm: i32) -> (Router, Arc<AppState>, i64, String, i64) {
        let pool = setup_test_db().await;
        let user_id = create_user_db(&pool, "item_user", "item_user@test.com", "Item User", &None, &None).await.unwrap();
        let token = "item_token";
        create_token(&pool, user_id, token, Utc::now() + chrono::Duration::hours(1)).await.unwrap();
        let event_id = create_event(&pool, "Item Event", None, None, None, Some("Room".into()), "#000".into()).await.unwrap();
        add_member(&pool, user_id, event_id, perm).await.unwrap();

        let state = Arc::new(AppState {
            tx: broadcast::channel(10).0,
            user_store: Arc::new(Mutex::new(UserStore::new())),
            verification_store: Arc::new(Mutex::new(VerificationStore::new())),
            db_pool: pool,
        });

        let app = Router::new()
            .route("/events/:event_id/planning/items", routing::post(create_item_list_handler))
            .route("/events/:event_id/planning/items/:module_id", routing::patch(update_item_list_handler))
            .route("/events/:event_id/planning/items/:module_id/items/:item_id/assign", routing::post(assign_item_handler))
            .route("/events/:event_id/planning/items/:module_id", routing::delete(delete_item_list_handler))
            .with_state(state.clone());

        (app, state, event_id, token.to_string(), user_id)
    }

    // ----------------- create item list -----------------
    #[tokio::test]
    async fn create_item_list_success() -> anyhow::Result<()> {
        let (app, _st, event_id, token, _uid) = setup(EventPermissions::OWNER).await;
        let payload = json!({"title":"Bring","items":["beer","chips"]});
        let req = Request::builder()
            .method("POST")
            .uri(&format!("/events/{}/planning/items", event_id))
            .header("Authorization", format!("Bearer {}", token))
            .header("content-type", "application/json")
            .body(Body::from(payload.to_string()))?;
        let resp = app.oneshot(req).await?;
        assert_eq!(resp.status(), StatusCode::CREATED);
        Ok(())
    }

    #[tokio::test]
    async fn create_item_list_no_perm() -> anyhow::Result<()> {
        let (app, _st, event_id, token, _uid) = setup(EventPermissions::MEMBER).await; // MEMBER не имеет права на создание
        let payload = json!({"title":"Fail","items":["item"]});
        let req = Request::builder()
            .method("POST")
            .uri(&format!("/events/{}/planning/items", event_id))
            .header("Authorization", format!("Bearer {}", token))
            .header("content-type", "application/json")
            .body(Body::from(payload.to_string()))?;
        let resp = app.oneshot(req).await?;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
        Ok(())
    }

    // ----------------- assign item -----------------
    async fn create_item_and_get_id(app: &Router, state: &Arc<AppState>, event_id: i64, token: &str) -> anyhow::Result<(i64, i64)> {
        let payload = json!({"title":"List","items":["item1"]});
        let req = Request::builder()
            .method("POST")
            .uri(&format!("/events/{}/planning/items", event_id))
            .header("Authorization", format!("Bearer {}", token))
            .header("content-type", "application/json")
            .body(Body::from(payload.to_string()))?;
        let resp = app.clone().oneshot(req).await?;
        assert_eq!(resp.status(), StatusCode::CREATED);
        let body_bytes = resp.into_body().collect().await?.to_bytes();
        let created: ItemListResponse = serde_json::from_slice(&body_bytes)?;
        // достаём item_id из базы
        let items = sqlx::query!("SELECT item_id FROM item_list_item WHERE item_list_id = $1", created.item_list_id)
            .fetch_all(&state.db_pool).await?;
        Ok((created.item_list_id, items[0].item_id))
    }

    #[tokio::test]
    async fn assign_item_success() -> anyhow::Result<()> {
        let (app, state, event_id, token, _uid) = setup(EventPermissions::OWNER).await;
        let (list_id, item_id) = create_item_and_get_id(&app, &state, event_id, &token).await?;
        let payload = json!({"item_list_id": list_id, "assign": true});
        let req = Request::builder()
            .method("POST")
            .uri(&format!("/events/{}/planning/items/{}/items/{}/assign", event_id, list_id, item_id))
            .header("Authorization", format!("Bearer {}", token))
            .header("content-type", "application/json")
            .body(Body::from(payload.to_string()))?;
        let resp = app.oneshot(req).await?;
        assert_eq!(resp.status(), StatusCode::OK);
        Ok(())
    }

    #[tokio::test]
    async fn assign_item_already_assigned() -> anyhow::Result<()> {
        let (app, state, event_id, token, _uid) = setup(EventPermissions::OWNER).await;
        let (list_id, item_id) = create_item_and_get_id(&app, &state, event_id, &token).await?;
        // первый раз
        let payload = json!({"item_list_id": list_id, "assign": true});
        let req = || Request::builder()
            .method("POST")
            .uri(&format!("/events/{}/planning/items/{}/items/{}/assign", event_id, list_id, item_id))
            .header("Authorization", format!("Bearer {}", token))
            .header("content-type", "application/json")
            .body(Body::from(payload.to_string())).unwrap();
        let _ = app.clone().oneshot(req()).await?;
        // второй раз – уже занято
        let resp = app.oneshot(req()).await?;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        Ok(())
    }

    // ----------------- delete item list -----------------
    #[tokio::test]
    async fn delete_item_list_owner() -> anyhow::Result<()> {
        let (app, state, event_id, token, _uid) = setup(EventPermissions::OWNER).await;
        let (list_id, _) = create_item_and_get_id(&app, &state, event_id, &token).await?;
        let req = Request::builder()
            .method("DELETE")
            .uri(&format!("/events/{}/planning/items/{}", event_id, list_id))
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())?;
        let resp = app.oneshot(req).await?;
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
        Ok(())
    }

    #[tokio::test]
    async fn delete_item_list_not_owner() -> anyhow::Result<()> {
        let pool = setup_test_db().await;
        // создаём событие с владельцем и членом
        let owner_id = create_user_db(&pool, "owner_item_del", "owner_item_del@test.com", "Owner", &None, &None).await.unwrap();
        let member_id = create_user_db(&pool, "member_item_del", "member_item_del@test.com", "Member", &None, &None).await.unwrap();
        let owner_token = "owner_token";
        let member_token = "member_token";
        create_token(&pool, owner_id, owner_token, Utc::now() + chrono::Duration::hours(1)).await.unwrap();
        create_token(&pool, member_id, member_token, Utc::now() + chrono::Duration::hours(1)).await.unwrap();
        let event_id = create_event(&pool, "Del Event", None, None, None, None, "#000".into()).await.unwrap();
        add_member(&pool, owner_id, event_id, EventPermissions::OWNER).await.unwrap();
        add_member(&pool, member_id, event_id, EventPermissions::MEMBER).await.unwrap();

        let state = Arc::new(AppState {
            tx: broadcast::channel(10).0,
            user_store: Arc::new(Mutex::new(UserStore::new())),
            verification_store: Arc::new(Mutex::new(VerificationStore::new())),
            db_pool: pool,
        });
        let app = Router::new()
            .route("/events/:event_id/planning/items/:module_id", routing::delete(delete_item_list_handler))
            .with_state(state.clone());

        // создаём item list от имени owner
        let create_payload = json!({"title":"List","items":["item"]});
        let create_req = Request::builder()
            .method("POST")
            .uri(&format!("/events/{}/planning/items", event_id))
            .header("Authorization", format!("Bearer {}", owner_token))
            .header("content-type", "application/json")
            .body(Body::from(create_payload.to_string()))?;
        let create_resp = Router::new()
            .route("/events/:event_id/planning/items", routing::post(create_item_list_handler))
            .with_state(state.clone())
            .oneshot(create_req).await?;
        let body_bytes = create_resp.into_body().collect().await?.to_bytes();
        let created: ItemListResponse = serde_json::from_slice(&body_bytes)?;

        // пытаемся удалить от имени члена
        let req = Request::builder()
            .method("DELETE")
            .uri(&format!("/events/{}/planning/items/{}", event_id, created.item_list_id))
            .header("Authorization", format!("Bearer {}", member_token))
            .body(Body::empty())?;
        let resp = app.oneshot(req).await?;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
        Ok(())
    }
}
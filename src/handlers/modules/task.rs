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

// ====================== Тесты ======================

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Router, body::Body, http::Request, routing};
    use tower::ServiceExt;
    use std::sync::Arc;
    use tokio::sync::{Mutex, broadcast};
    use serde_json::json;
    use chrono::Utc;

    use crate::{
        config::Config,
        test_utils::setup_test_db,
        structs::AppState,
        user_store::UserStore,
        secrets::verification::VerificationStore,
        data_base::{
            user_db::{create_user_db, create_token, find_user_by_id},
            event_db::{create_event, add_member},
        },
        permissions::EventPermissions,
    };

    async fn setup(perm: i32) -> (Router, Arc<AppState>, i64, String, i64) {
        let pool = setup_test_db().await;
        let user_id = create_user_db(&pool, "task_user", "task_user@test.com", "Task User", &None, &None).await.unwrap();
        let token = "task_token";
        create_token(&pool, user_id, token, Utc::now() + chrono::Duration::hours(1)).await.unwrap();
        let event_id = create_event(&pool, "Task Event", None, None, None, Some("Room".into()), "#000".into()).await.unwrap();
        add_member(&pool, user_id, event_id, perm).await.unwrap();

        let state = Arc::new(AppState {
            tx: broadcast::channel(10).0,
            user_store: Arc::new(Mutex::new(UserStore::new())),
            verification_store: Arc::new(Mutex::new(VerificationStore::new())),
            db_pool: pool,
            config: Config::from_env(),
        });

        let app = Router::new()
            .route("/events/{event_id}/planning/task_list", routing::post(create_task_list_handler))
            .route("/events/{event_id}/planning/task_list/{module_id}", routing::patch(update_task_list_handler))
            .route("/events/{event_id}/planning/task_list/{module_id}/items/{task_id}/assign", routing::patch(assign_task_handler))
            .route("/events/{event_id}/planning/task_list/{module_id}/items/{task_id}/complete", routing::patch(complete_task_handler))
            .route("/events/{event_id}/planning/task_list/{module_id}", routing::delete(delete_task_list_handler))
            .with_state(state.clone());

        (app, state, event_id, token.to_string(), user_id)
    }

    async fn create_task_list_and_get_ids(state: &Arc<AppState>, event_id: i64, token: &str) -> anyhow::Result<(i64, i64)> {
        let pool = &state.db_pool;
        let created = create_task_list(pool, event_id, "Test", &["task1".to_string()], 1).await?;
        Ok((created.task_list_id, created.items[0].task_id))
    }

    // ----------------- create -----------------
    #[tokio::test]
    async fn create_task_list_success() -> anyhow::Result<()> {
        let (app, _st, event_id, token, _uid) = setup(EventPermissions::OWNER).await;
        let payload = json!({"title":"To Do","items":["task1","task2"]});
        let req = Request::builder()
            .method("POST")
            .uri(&format!("/events/{}/planning/task_list", event_id))
            .header("Authorization", format!("Bearer {}", token))
            .header("content-type", "application/json")
            .body(Body::from(payload.to_string()))?;
        let resp = app.oneshot(req).await?;
        assert_eq!(resp.status(), StatusCode::CREATED);
        Ok(())
    }

    #[tokio::test]
    async fn create_task_list_no_perm() -> anyhow::Result<()> {
        let (app, _st, event_id, token, _uid) = setup(EventPermissions::MEMBER).await;
        let payload = json!({"title":"To Do","items":["task"]});
        let req = Request::builder()
            .method("POST")
            .uri(&format!("/events/{}/planning/task_list", event_id))
            .header("Authorization", format!("Bearer {}", token))
            .header("content-type", "application/json")
            .body(Body::from(payload.to_string()))?;
        let resp = app.oneshot(req).await?;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
        Ok(())
    }

    // ----------------- assign -----------------
    #[tokio::test]
    async fn assign_task_success() -> anyhow::Result<()> {
        let (app, state, event_id, token, _uid) = setup(EventPermissions::OWNER).await;
        let (list_id, task_id) = create_task_list_and_get_ids(&state, event_id, &token).await?;
        let payload = json!({"assign": true});
        let req = Request::builder()
            .method("PATCH")
            .uri(&format!("/events/{}/planning/task_list/{}/items/{}/assign", event_id, list_id, task_id))
            .header("Authorization", format!("Bearer {}", token))
            .header("content-type", "application/json")
            .body(Body::from(payload.to_string()))?;
        let resp = app.oneshot(req).await?;
        assert_eq!(resp.status(), StatusCode::OK);
        Ok(())
    }

    #[tokio::test]
    async fn assign_task_already_assigned() -> anyhow::Result<()> {
        let (app, state, event_id, token, _uid) = setup(EventPermissions::OWNER).await;
        let (list_id, task_id) = create_task_list_and_get_ids(&state, event_id, &token).await?;
        let payload = json!({"assign": true});
        let req_fn = || Request::builder()
            .method("PATCH")
            .uri(&format!("/events/{}/planning/task_list/{}/items/{}/assign", event_id, list_id, task_id))
            .header("Authorization", format!("Bearer {}", token))
            .header("content-type", "application/json")
            .body(Body::from(payload.to_string())).unwrap();
        let _ = app.clone().oneshot(req_fn()).await?;
        let resp = app.oneshot(req_fn()).await?;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        Ok(())
    }

    // ----------------- complete -----------------
    #[tokio::test]
    async fn complete_task_success() -> anyhow::Result<()> {
        let (app, state, event_id, token, _uid) = setup(EventPermissions::OWNER).await;
        let (list_id, task_id) = create_task_list_and_get_ids(&state, event_id, &token).await?;
        // назначим задачу
        let assign_payload = json!({"assign": true});
        let assign_req = Request::builder()
            .method("PATCH")
            .uri(&format!("/events/{}/planning/task_list/{}/items/{}/assign", event_id, list_id, task_id))
            .header("Authorization", format!("Bearer {}", token))
            .header("content-type", "application/json")
            .body(Body::from(assign_payload.to_string()))?;
        let _ = app.clone().oneshot(assign_req).await?;

        let complete_payload = json!({"completed": true});
        let complete_req = Request::builder()
            .method("PATCH")
            .uri(&format!("/events/{}/planning/task_list/{}/items/{}/complete", event_id, list_id, task_id))
            .header("Authorization", format!("Bearer {}", token))
            .header("content-type", "application/json")
            .body(Body::from(complete_payload.to_string()))?;
        let resp = app.oneshot(complete_req).await?;
        assert_eq!(resp.status(), StatusCode::OK);
        Ok(())
    }

    #[tokio::test]
    async fn complete_task_not_assigned() -> anyhow::Result<()> {
        let (app, state, event_id, token, _uid) = setup(EventPermissions::OWNER).await;
        let (list_id, task_id) = create_task_list_and_get_ids(&state, event_id, &token).await?;
        let payload = json!({"completed": true});
        let req = Request::builder()
            .method("PATCH")
            .uri(&format!("/events/{}/planning/task_list/{}/items/{}/complete", event_id, list_id, task_id))
            .header("Authorization", format!("Bearer {}", token))
            .header("content-type", "application/json")
            .body(Body::from(payload.to_string()))?;
        let resp = app.oneshot(req).await?;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        Ok(())
    }

    // ----------------- delete -----------------
    #[tokio::test]
    async fn delete_task_list_owner() -> anyhow::Result<()> {
        let (app, state, event_id, token, _uid) = setup(EventPermissions::OWNER).await;
        let (list_id, _) = create_task_list_and_get_ids(&state, event_id, &token).await?;
        let req = Request::builder()
            .method("DELETE")
            .uri(&format!("/events/{}/planning/task_list/{}", event_id, list_id))
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())?;
        let resp = app.oneshot(req).await?;
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
        Ok(())
    }

    #[tokio::test]
    async fn delete_task_list_not_owner() -> anyhow::Result<()> {
        let pool = setup_test_db().await;
        let owner_id = create_user_db(&pool, "owner_task_del", "owner_task_del@test.com", "Owner", &None, &None).await.unwrap();
        let member_id = create_user_db(&pool, "member_task_del", "member_task_del@test.com", "Member", &None, &None).await.unwrap();
        let owner_token = "owner_task_token";
        let member_token = "member_task_token";
        create_token(&pool, owner_id, owner_token, Utc::now() + chrono::Duration::hours(1)).await.unwrap();
        create_token(&pool, member_id, member_token, Utc::now() + chrono::Duration::hours(1)).await.unwrap();
        let event_id = create_event(&pool, "Del Task Event", None, None, None, None, "#000".into()).await.unwrap();
        add_member(&pool, owner_id, event_id, EventPermissions::OWNER).await.unwrap();
        add_member(&pool, member_id, event_id, EventPermissions::MEMBER).await.unwrap();

        let state = Arc::new(AppState {
            tx: broadcast::channel(10).0,
            user_store: Arc::new(Mutex::new(UserStore::new())),
            verification_store: Arc::new(Mutex::new(VerificationStore::new())),
            db_pool: pool,
            config: Config::from_env(),
        });

        // создаём task list от владельца (через прямой вызов БД, чтобы не усложнять)
        let created = create_task_list(&state.db_pool, event_id, "To Delete", &["task".to_string()], owner_id).await?;

        let app = Router::new()
            .route("/events/{event_id}/planning/task_list/{module_id}", routing::delete(delete_task_list_handler))
            .with_state(state);

        let req = Request::builder()
            .method("DELETE")
            .uri(&format!("/events/{}/planning/task_list/{}", event_id, created.task_list_id))
            .header("Authorization", format!("Bearer {}", member_token))
            .body(Body::empty())?;
        let resp = app.oneshot(req).await?;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
        Ok(())
    }
}
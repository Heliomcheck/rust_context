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
    Path((event_id, module_id, task_id)): Path<(i64, i64, i64)>,
    Json(payload): Json<AssignTaskRequest>
) -> Result<impl IntoResponse, AppError> {
    let user = get_user_for_handler_from_token(&state.db_pool, auth.token()).await?;
    
    let is_in_event = check_user_in_event(&state.db_pool, event_id, user.user_id).await?;
    if !is_in_event {
        return Err(AppError::UserNotInEvent("User not in event".to_string()));
    }
    
    let belongs = verify_task_list_in_event(&state.db_pool, module_id, event_id).await?;
    if !belongs {
        return Err(AppError::NotFound("Task list not found in this event".to_string()));
    }
    
    assign_task(
        &state.db_pool,
        task_id,
        user.user_id,
        payload.assign,
    )
    .await?;
    
    let _ = get_task_list(&state.db_pool, module_id)
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
    Path((event_id, module_id, task_id)): Path<(i64, i64, i64)>,
    Json(payload): Json<CompleteTaskRequest>,
) -> Result<impl IntoResponse, AppError> {
    let user = get_user_for_handler_from_token(&state.db_pool, auth.token()).await?;
    
    let is_in_event = check_user_in_event(&state.db_pool, event_id, user.user_id).await?;
    if !is_in_event {
        return Err(AppError::UserNotInEvent("User not in event".to_string()));
    }
    
    let belongs = verify_task_list_in_event(&state.db_pool, module_id, event_id).await?;
    if !belongs {
        return Err(AppError::NotFound("Task list not found in this event".to_string()));
    }
    
    complete_task(
        &state.db_pool,
        task_id,
        user.user_id,
        payload.completed,
    )
    .await?;
    
    let _ = get_task_list(&state.db_pool, module_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Task list not found".to_string()))?;

    Ok((StatusCode::OK, Json(SuccessResponse { success: true })))
}

#[utoipa::path(
    delete,
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
#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        Router, 
        body::Body, 
        http::Request
    };
    use tower::ServiceExt;
    use std::sync::Arc;
    use tokio::sync::{
        Mutex, 
        broadcast
    };
    use serde_json::json;
    use http_body_util::BodyExt;
    use chrono::Utc;

    use crate::{
        test_utils::*,
        structs::AppState,
        //user_store::UserStore,
        secrets::verification::VerificationStore,
        data_base::{
            user_db::{
                create_user_db, 
                create_token
            },
            event_db::{
                create_event, 
                add_member
            },
        },
        permissions::EventPermissions,
    };

    async fn setup(perm: i32) -> (Router, Arc<AppState>, i64, String, i64) {
        let _guard = lock_db().await;
        let pool = setup_test_db().await;
        let user_id = create_user_db(&pool, "task_user", "task_user@test.com", "Task User", &None, &None).await.unwrap();
        let token = "task_token";
        create_token(&pool, user_id, token, Utc::now() + chrono::Duration::hours(1)).await.unwrap();
        let event_id = create_event(&pool, "Task Event", None, None, None, Some("Room".into()), "#000".into()).await.unwrap();
        add_member(&pool, user_id, event_id, perm).await.unwrap();

        let state = Arc::new(AppState {
            tx: broadcast::channel(10).0,
            verification_store: Arc::new(Mutex::new(VerificationStore::new())),
            db_pool: pool,
        });

        let app = create_app(state.clone()).await;

        (app, state, event_id, token.to_string(), user_id)
    }

    async fn create_task_list_and_get_ids(app: &Router, state: &Arc<AppState>, event_id: i64, token: &str) -> anyhow::Result<(i64, i64)> {
        let payload = json!({"title":"Tasks","items":["task1"]});
        let req = Request::builder()
            .method("POST")
            .uri(&format!("/events/{}/planning/tasks", event_id))
            .header("Authorization", format!("Bearer {}", token))
            .header("content-type", "application/json")
            .body(Body::from(payload.to_string()))?;
        let resp = app.clone().oneshot(req).await?;
        assert_eq!(resp.status(), StatusCode::CREATED);
        let body_bytes = resp.into_body().collect().await?.to_bytes();
        let created: CreateTaskListResponse = serde_json::from_slice(&body_bytes)?;
        let tasks = sqlx::query!("SELECT task_id FROM task_list_item WHERE task_list_id = $1", created.task_list_id)
            .fetch_all(&state.db_pool).await?;
        Ok((created.task_list_id, tasks[0].task_id))
    }

    // ----------------- create -----------------
    #[tokio::test]
    async fn create_task_list_success() -> anyhow::Result<()> {
        let (app, _st, event_id, token, _uid) = setup(EventPermissions::OWNER).await;
        let payload = json!({"title":"To Do","items":["task1","task2"]});
        let req = Request::builder()
            .method("POST")
            .uri(&format!("/events/{}/planning/tasks", event_id))
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
            .uri(&format!("/events/{}/planning/tasks", event_id))
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
        let (list_id, task_id) = create_task_list_and_get_ids(&app, &state, event_id, &token).await?;
        let payload = json!({"task_list_id": list_id, "task_id": task_id, "assign": true});
        let req = Request::builder()
            .method("POST")
            .uri(&format!("/events/{}/planning/tasks/{}/items/{}/assign", event_id, list_id, task_id))
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
        let (list_id, task_id) = create_task_list_and_get_ids(&app, &state, event_id, &token).await?;
        let payload = json!({"task_list_id": list_id, "task_id": task_id, "assign": true});
        let req = || Request::builder()
            .method("POST")
            .uri(&format!("/events/{}/planning/tasks/{}/items/{}/assign", event_id, list_id, task_id))
            .header("Authorization", format!("Bearer {}", token))
            .header("content-type", "application/json")
            .body(Body::from(payload.to_string())).unwrap();
        let _ = app.clone().oneshot(req()).await?;
        let resp = app.oneshot(req()).await?;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        Ok(())
    }

    // ----------------- complete -----------------
    #[tokio::test]
    async fn complete_task_success() -> anyhow::Result<()> {
        let (app, state, event_id, token, _uid) = setup(EventPermissions::OWNER).await;
        let (list_id, task_id) = create_task_list_and_get_ids(&app, &state, event_id, &token).await?;
        // assign first
        let payload = json!({"task_list_id": list_id, "task_id": task_id, "assign": true});
        let req = Request::builder()
            .method("POST")
            .uri(&format!("/events/{}/planning/tasks/{}/items/{}/assign", event_id, list_id, task_id))
            .header("Authorization", format!("Bearer {}", token))
            .header("content-type", "application/json")
            .body(Body::from(payload.to_string()))?;
        let _ = app.clone().oneshot(req).await?;

        let complete_payload = json!({"task_list_id": list_id, "task_id": task_id, "completed": true});
        let req = Request::builder()
            .method("POST")
            .uri(&format!("/events/{}/planning/tasks/{}/items/{}/complete", event_id, list_id, task_id))
            .header("Authorization", format!("Bearer {}", token))
            .header("content-type", "application/json")
            .body(Body::from(complete_payload.to_string()))?;
        let resp = app.oneshot(req).await?;
        assert_eq!(resp.status(), StatusCode::OK);
        Ok(())
    }

    #[tokio::test]
    async fn complete_task_not_assigned() -> anyhow::Result<()> {
        let (app, state, event_id, token, _uid) = setup(EventPermissions::OWNER).await;
        let (list_id, task_id) = create_task_list_and_get_ids(&app, &state, event_id, &token).await?;
        let payload = json!({"task_list_id": list_id, "task_id": task_id, "completed": true});
        let req = Request::builder()
            .method("POST")
            .uri(&format!("/events/{}/planning/tasks/{}/items/{}/complete", event_id, list_id, task_id))
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
        let (list_id, _) = create_task_list_and_get_ids(&app, &state, event_id, &token).await?;
        let req = Request::builder()
            .method("DELETE")
            .uri(&format!("/events/{}/planning/tasks/{}", event_id, list_id))
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
            //user_store: Arc::new(Mutex::new(UserStore::new())),
            verification_store: Arc::new(Mutex::new(VerificationStore::new())),
            db_pool: pool,
        });
        let app = create_app(state.clone()).await;

        // создаём task list от owner
        let create_payload = json!({"title":"Tasks","items":["task"]});
        let create_req = Request::builder()
            .method("POST")
            .uri(&format!("/events/{}/planning/tasks", event_id))
            .header("Authorization", format!("Bearer {}", owner_token))
            .header("content-type", "application/json")
            .body(Body::from(create_payload.to_string()))?;
        let create_resp = Router::new()
            .route("/events/:event_id/planning/tasks", routing::post(create_task_list_handler))
            .with_state(state.clone())
            .oneshot(create_req).await?;
        let body_bytes = create_resp.into_body().collect().await?.to_bytes();
        let created: CreateTaskListResponse = serde_json::from_slice(&body_bytes)?;

        let req = Request::builder()
            .method("DELETE")
            .uri(&format!("/events/{}/planning/tasks/{}", event_id, created.task_list_id))
            .header("Authorization", format!("Bearer {}", member_token))
            .body(Body::empty())?;
        let resp = app.oneshot(req).await?;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
        Ok(())
    }
}
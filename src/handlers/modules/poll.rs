use axum::{
    response::IntoResponse,
    extract::Path,
    extract::State,
    Json,
    http::StatusCode,
};
use std::{
    sync::Arc,
    result::Result
};
use serde_json::json;
use axum_extra::TypedHeader;
use headers::{
    Authorization, 
    authorization::Bearer
};
use crate::{
    data_base::{
        event_db::*, 
        plainning_modules::poll_db::*
    }, 
    errors::AppError, 
    handlers::user::get_user_for_handler_from_token, 
    models::*, 
    permissions::*, 
    structs::*
};

use crate::data_base::plainning_modules::poll_db::verify_poll_in_event;

// ====================== Обработчики ======================

#[utoipa::path(
    post,
    path = "/events/{event_id}/planning/poll",
    tag = "Modules",
    security(("bearerAuth" = [])),
    request_body = CreatePollRequest,
    responses(
        (status = 201, description = "Poll created", body = PollResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Not found")
    )
)]
pub async fn create_poll_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    Path(event_id): Path<i64>,
    Json(payload): Json<CreatePollRequest>,
) -> Result<impl IntoResponse, AppError> {
    let user = get_user_for_handler_from_token(&state.db_pool, &auth.token()).await?;
    let event = get_event_by_id(&state.db_pool, event_id).await?;
    let is_member = check_user_in_event(&state.db_pool, event.event_id, user.user_id).await?;
    if !is_member {
        return Err(AppError::UserNotInEvent("User not in event".to_string()));
    }
    match check_user_permissions(&state.db_pool, &event, &user, EventPermissions::OWNER).await {
        Ok(true) => {},
        Ok(false) => return Err(AppError::UserNotInEvent("User doesn't have permission".to_string())),
        Err(e) => return Err(e),
    };

    let _ = create_poll(
        &state.db_pool, event_id, payload.title, user.user_id,
        payload.options, payload.multiple_choice
    ).await?;

    Ok((StatusCode::CREATED, Json(SuccessResponse { success: true })))
}

#[utoipa::path(
    put,
    path = "/events/{event_id}/planning/poll/{module_id}",
    tag = "Modules",
    security(("bearerAuth" = [])),
    request_body = UpdatePollRequest,
    responses(
        (status = 200, description = "Poll updated"),
        (status = 400, description = "Bad request"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Not found")
    )
)]
pub async fn update_poll_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    Path(path): Path<EventModule>,
    Json(payload): Json<UpdatePollRequest>
) -> Result<impl IntoResponse, AppError> {
    let user = get_user_for_handler_from_token(&state.db_pool, &auth.token()).await?;
    let event = get_event_by_id(&state.db_pool, path.event_id).await?;
    if !check_user_in_event(&state.db_pool, event.event_id, user.user_id).await? {
        return Err(AppError::UserNotInEvent("User not in event".to_string()));
    }
    match check_user_permissions(&state.db_pool, &event, &user, EventPermissions::OWNER).await {
        Ok(true) => {},
        Ok(false) => return Err(AppError::UserNotInEvent("Not allowed".to_string())),
        Err(e) => return Err(e),
    };

    let updated = edit_pool_question(&state.db_pool, path.module_id, payload.question).await?;
    if !updated {
        return Err(AppError::BadRequest("Poll not found".to_string()));
    }
    Ok((StatusCode::OK, Json(json!({"success": true}))))
}

#[utoipa::path(
    delete,
    path = "/events/{event_id}/planning/poll/{module_id}",
    tag = "Modules",
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "Poll deleted"),
        (status = 400, description = "Bad request"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Not found")
    )
)]
pub async fn delete_poll_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    Path(path): Path<EventModule>,
) -> Result<impl IntoResponse, AppError> {
    let user = get_user_for_handler_from_token(&state.db_pool, &auth.token()).await?;
    let event = get_event_by_id(&state.db_pool, path.event_id).await?;
    if !check_user_in_event(&state.db_pool, event.event_id, user.user_id).await? {
        return Err(AppError::UserNotInEvent("User not in event".to_string()));
    }
    match check_user_permissions(&state.db_pool, &event, &user, EventPermissions::OWNER).await {
        Ok(true) => {},
        Ok(false) => return Err(AppError::UserNotInEvent("Not allowed".to_string())),
        Err(e) => return Err(e),
    };

    let deleted = delete_poll(&state.db_pool, path.module_id).await?;
    if !deleted {
        return Err(AppError::BadRequest("Poll not found".to_string()));
    }
    Ok((StatusCode::OK, Json(SuccessResponse {success: true})))
}

// ====================== Голосование ======================

#[utoipa::path(
    patch,
    path = "/events/{event_id}/planning/poll/{module_id}/vote",
    tag = "Modules",
    security(("bearerAuth" = [])),
    request_body = VotePollRequest,
    responses(
        (status = 200, description = "Vote accepted"),
        (status = 400, description = "Bad request"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Not found")
    )
)]
pub async fn vote_poll_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    Path(path): Path<EventModule>,
    Json(payload): Json<VotePollRequest>,
) -> Result<impl IntoResponse, AppError> {
    let user = get_user_for_handler_from_token(&state.db_pool, &auth.token()).await?;
    let event = get_event_by_id(&state.db_pool, path.event_id).await?;

    if !check_user_in_event(&state.db_pool, event.event_id, user.user_id).await? {
        return Err(AppError::UserNotInEvent("User not in event".to_string()));
    }

    let poll_belongs = verify_poll_in_event(&state.db_pool, path.module_id, path.event_id).await?;
    if !poll_belongs {
        return Err(AppError::Forbidden("Poll not in this event".to_string()));
    }

    vote_on_poll(&state.db_pool, path.module_id, user.user_id, payload.option_indexes).await?;

    Ok((StatusCode::OK, Json(SuccessResponse { success: true })))
}

// ====================== Тесты ======================

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        Router, 
        body::Body, 
        http::Request,
        routing
    };
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
        let user_id = create_user_db(&pool, "poll_user", "poll_user@test.com", "Poll User", &None, &None).await.unwrap();
        let token = "poll_token";
        create_token(&pool, user_id, token, Utc::now() + chrono::Duration::hours(1)).await.unwrap();
        let event_id = create_event(&pool, "Poll Event", None, None, None, Some("Room".into()), "#000".into()).await.unwrap();
        add_member(&pool, user_id, event_id, perm).await.unwrap();

        let state = Arc::new(AppState {
            tx: broadcast::channel(10).0,
            user_store: Arc::new(Mutex::new(UserStore::new())),
            verification_store: Arc::new(Mutex::new(VerificationStore::new())),
            db_pool: pool,
            config: Config::from_env(),
        });

        let app = Router::new()
            .route("/events/{event_id}/planning/poll", routing::post(create_poll_handler))
            .route("/events/{event_id}/planning/poll/{module_id}", routing::put(update_poll_handler))
            .route("/events/{event_id}/planning/poll/{module_id}", routing::delete(delete_poll_handler))
            .route("/events/{event_id}/planning/poll/{module_id}/vote", routing::patch(vote_poll_handler))
            .with_state(state.clone());

        (app, state, event_id, token.to_string(), user_id)
    }

    // ----------------- create poll -----------------
    #[tokio::test]
    async fn create_poll_success() -> anyhow::Result<()> {
        let (app, _st, event_id, token, _uid) = setup(EventPermissions::OWNER).await;
        let payload = json!({"title":"Best day?","options":["Mon","Tue"],"multiple_choice":false});
        let req = Request::builder()
            .method("POST")
            .uri(&format!("/events/{}/planning/poll", event_id))
            .header("Authorization", format!("Bearer {}", token))
            .header("content-type", "application/json")
            .body(Body::from(payload.to_string()))?;
        let resp = app.oneshot(req).await?;
        assert_eq!(resp.status(), StatusCode::CREATED);
        Ok(())
    }

    #[tokio::test]
    async fn create_poll_too_few_options() -> anyhow::Result<()> {
        let (app, _st, event_id, token, _uid) = setup(EventPermissions::OWNER).await;
        let payload = json!({"title":"Fail","options":["OnlyOne"],"multiple_choice":false});
        let req = Request::builder()
            .method("POST")
            .uri(&format!("/events/{}/planning/poll", event_id))
            .header("Authorization", format!("Bearer {}", token))
            .header("content-type", "application/json")
            .body(Body::from(payload.to_string()))?;
        let resp = app.oneshot(req).await?;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        Ok(())
    }

    #[tokio::test]
    async fn create_poll_not_in_event() -> anyhow::Result<()> {
        let pool = setup_test_db().await;
        let stranger_id = create_user_db(&pool, "stranger", "stranger@test.com", "Stranger", &None, &None).await.unwrap();
        let token = "stranger_token";
        create_token(&pool, stranger_id, token, Utc::now() + chrono::Duration::hours(1)).await.unwrap();
        let event_id = create_event(&pool, "Event", None, None, None, None, "#000".into()).await.unwrap();
        let state = Arc::new(AppState {
            tx: broadcast::channel(10).0,
            user_store: Arc::new(Mutex::new(UserStore::new())),
            verification_store: Arc::new(Mutex::new(VerificationStore::new())),
            db_pool: pool,
            config: Config::from_env(),
        });
        let app = Router::new()
            .route("/events/{event_id}/planning/poll", routing::post(create_poll_handler))
            .with_state(state);

        let payload = json!({"title":"Poll","options":["A","B"],"multiple_choice":false});
        let req = Request::builder()
            .method("POST")
            .uri(&format!("/events/{}/planning/poll", event_id))
            .header("Authorization", format!("Bearer {}", token))
            .header("content-type", "application/json")
            .body(Body::from(payload.to_string()))?;
        let resp = app.oneshot(req).await?;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
        Ok(()) 
    }

    // ----------------- vote poll -----------------
    async fn poll_with_voter() -> (Router, Arc<AppState>, i64, String, i64, i64) {
        let pool = setup_test_db().await;
        let creator_id = create_user_db(&pool, "creator_vote", "creator_vote@test.com", "Creator", &None, &None).await.unwrap();
        let voter_id = create_user_db(&pool, "voter", "voter@test.com", "Voter", &None, &None).await.unwrap();
        let token = "voter_token";
        create_token(&pool, voter_id, token, Utc::now() + chrono::Duration::hours(1)).await.unwrap();
        let event_id = create_event(&pool, "Vote Event", None, None, None, None, "#000".into()).await.unwrap();
        add_member(&pool, creator_id, event_id, EventPermissions::OWNER).await.unwrap();
        add_member(&pool, voter_id, event_id, EventPermissions::MEMBER).await.unwrap();
        let module_id = create_poll(&pool, event_id, "Q".into(), creator_id, vec!["A".into(), "B".into()], false).await.unwrap();

        let state = Arc::new(AppState {
            tx: broadcast::channel(10).0,
            user_store: Arc::new(Mutex::new(UserStore::new())),
            verification_store: Arc::new(Mutex::new(VerificationStore::new())),
            db_pool: pool,
            config: Config::from_env(),
        });
        let app = Router::new()
            .route("/events/{event_id}/planning/poll/{module_id}/vote", routing::patch(vote_poll_handler))
            .with_state(state.clone());
        (app, state, event_id, token.to_string(), voter_id, module_id)
    }

    #[tokio::test]
    async fn vote_success() -> anyhow::Result<()> {
        let (app, _st, event_id, token, _voter, module_id) = poll_with_voter().await;
        let payload = json!({"option_indexes":[0]});
        let req = Request::builder()
            .method("PATCH")
            .uri(&format!("/events/{}/planning/poll/{}/vote", event_id, module_id))
            .header("Authorization", format!("Bearer {}", token))
            .header("content-type", "application/json")
            .body(Body::from(payload.to_string()))?;
        let resp = app.oneshot(req).await?;
        assert_eq!(resp.status(), StatusCode::OK);
        Ok(())
    }

    #[tokio::test]
    async fn vote_invalid_option_index() -> anyhow::Result<()> {
        let (app, _st, event_id, token, _voter, poll_id) = poll_with_voter().await;
        let payload = json!({"option_indexes":[5]});
        let req = Request::builder()
            .method("PATCH")
            .uri(&format!("/events/{}/planning/poll/{}/vote", event_id, poll_id))
            .header("Authorization", format!("Bearer {}", token))
            .header("content-type", "application/json")
            .body(Body::from(payload.to_string()))?;
        let resp = app.oneshot(req).await?;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        Ok(())
    }

    #[tokio::test]
    async fn vote_not_in_event() -> anyhow::Result<()> {
        let pool = setup_test_db().await;
        let user_id = create_user_db(&pool, "vote_user", "vote_user@test.com", "Vote User", &None, &None).await.unwrap();
        let token = "vote_token";
        create_token(&pool, user_id, token, Utc::now() + chrono::Duration::hours(1)).await.unwrap();
        let event_with_poll = create_event(&pool, "Event With Poll", None, None, None, None, "#000".into()).await.unwrap();
        add_member(&pool, user_id, event_with_poll, EventPermissions::OWNER).await.unwrap();
        let module_id = create_poll(
            &pool, event_with_poll, "Question".into(), user_id,
            vec!["A".into(), "B".into()], false
        ).await.unwrap();
        let event_id = create_event(&pool, "Event Without Poll", None, None, None, None, "#111".into()).await.unwrap();
        add_member(&pool, user_id, event_id, EventPermissions::OWNER).await.unwrap();

        let state = Arc::new(AppState {
            tx: broadcast::channel(10).0,
            user_store: Arc::new(Mutex::new(UserStore::new())),
            verification_store: Arc::new(Mutex::new(VerificationStore::new())),
            db_pool: pool,
            config: Config::from_env(),
        });
        let app = Router::new()
            .route("/events/{event_id}/planning/poll/{module_id}/vote", routing::patch(vote_poll_handler))
            .with_state(state);

        let payload = json!({"option_indexes":[0]});
        let req = Request::builder()
            .method("PATCH")
            .uri(&format!("/events/{}/planning/poll/{}/vote", event_id, module_id))
            .header("Authorization", format!("Bearer {}", token))
            .header("content-type", "application/json")
            .body(Body::from(payload.to_string()))?;
        let resp = app.oneshot(req).await?;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
        Ok(())
    }

    // ----------------- update poll -----------------
    #[tokio::test]
    async fn update_poll_owner() -> anyhow::Result<()> {
        let pool = setup_test_db().await;
        let (app, _st, event_id, token, user_id) = setup(EventPermissions::OWNER).await;
        let module_id = create_poll(&pool, event_id, "Old Q".to_string(), user_id, vec!["A".into(),"B".into()], false).await?;
        let payload = json!({"question":"New Q"});
        let req = Request::builder()
            .method("PUT")
            .uri(&format!("/events/{}/planning/poll/{}", event_id, module_id))
            .header("Authorization", format!("Bearer {}", token))
            .header("content-type", "application/json")
            .body(Body::from(payload.to_string()))?;
        let resp = app.oneshot(req).await?;
        assert_eq!(resp.status(), StatusCode::OK);
        Ok(())
    }

    #[tokio::test]
    async fn update_poll_not_owner() -> anyhow::Result<()> {
        let pool = setup_test_db().await;
        let (app, _st, event_id, _token, owner_id) = setup(EventPermissions::OWNER).await;
        let module_id = create_poll(&pool, event_id, "Q".into(), owner_id, vec!["A".into(),"B".into()], false).await.unwrap();
        let member_id = create_user_db(&pool, "member_update", "member_update@test.com", "Member", &None, &None).await.unwrap();
        let member_token = "member_token";
        create_token(&pool, member_id, member_token, Utc::now() + chrono::Duration::hours(1)).await.unwrap();
        add_member(&pool, member_id, event_id, EventPermissions::MEMBER).await.unwrap();
        let payload = json!({"question":"Hack"});
        let req = Request::builder()
            .method("PUT")
            .uri(&format!("/events/{}/planning/poll/{}", event_id, module_id))
            .header("Authorization", format!("Bearer {}", member_token))
            .header("content-type", "application/json")
            .body(Body::from(payload.to_string()))?;
        let resp = app.oneshot(req).await?;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
        Ok(())
    }

    // ----------------- delete poll -----------------
    #[tokio::test]
    async fn delete_poll_owner() -> anyhow::Result<()> {
        let pool = setup_test_db().await;
        let (app, _st, event_id, token, user_id) = setup(EventPermissions::OWNER).await;
        let module_id = create_poll(&pool, event_id, "Del".into(), user_id, vec!["A".into(),"B".into()], false).await.unwrap();
        let req = Request::builder()
            .method("DELETE")
            .uri(&format!("/events/{}/planning/poll/{}", event_id, module_id))
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())?;
        let resp = app.oneshot(req).await?;
        assert_eq!(resp.status(), StatusCode::OK);
        Ok(())
    }

    use http_body_util::BodyExt;
    #[tokio::test]
    async fn delete_poll_not_owner() -> anyhow::Result<()> {
        let pool = setup_test_db().await;
        let (app, _st, event_id, _token, owner_id) = setup(EventPermissions::OWNER).await;
        let module_id = create_poll(&pool, event_id, "Del".into(), owner_id, vec!["A".into(),"B".into()], false).await.unwrap();
        let member_id = create_user_db(&pool, "member_del", "member_del@test.com", "Member", &None, &None).await.unwrap();
        let member_token = "member_del_token";
        create_token(&pool, member_id, member_token, Utc::now() + chrono::Duration::hours(1)).await.unwrap();
        add_member(&pool, member_id, event_id, EventPermissions::MEMBER).await.unwrap();

        {
            let mut store = _st.user_store.lock().await;
            let user = find_user_by_id(&pool, member_id).await?.unwrap();
            store.users.insert(member_id, user);
        }

        let req = Request::builder()
            .method("DELETE")
            .uri(&format!("/events/{}/planning/poll/{}", event_id, module_id))
            .header("Authorization", format!("Bearer {}", member_token))
            .body(Body::empty())?;
        let resp = app.oneshot(req).await?;
        let status = resp.status();
        let (_, body) = resp.into_parts();
        let bytes = body.collect().await?.to_bytes();
        let body_str = String::from_utf8_lossy(&bytes);
        println!("🔍 Response status: {}", status);
        println!("🔍 Response body: {}", body_str);
        assert_eq!(status, StatusCode::FORBIDDEN);
        Ok(())
    }
}
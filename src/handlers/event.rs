use chrono::{DateTime, Utc};
use axum::{
    response::IntoResponse,
    extract::{
        State,
        Query
    },
    extract::path::Path,
    Json,
    http::StatusCode,
    http::HeaderMap,
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
    structs::*,
    models::*,
    errors::AppError,
    data_base::{
        event_db::*,
        plainning_modules::poll_db::*,
        plainning_modules::item_db::*,
        plainning_modules::task_db::*,
    },
    permissions::*,
    handlers::user::get_user_for_handler_from_token,
    *
};

// Event handlers

#[utoipa::path(
    post,
    path = "/events",
    tag = "Event",
    security(
        ("bearerAuth" = [])
    ),
    request_body = CreateEventRequest,
    responses(
        (status = 201, description = "Event created", body = CreateEventResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 404, description = "User or event not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn create_event_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    Json(payload): Json<CreateEventRequest>,
) -> Result<impl IntoResponse, AppError> {
    let user = get_user_for_handler_from_token(&state.db_pool, auth.token()).await?;

    let start_date_time = match payload.start_date_time {
        Some(ref s) => {
            let dt = s.parse::<DateTime<Utc>>().map_err(|_| AppError::BadRequest("Invalid start date format. Use ISO 8601".to_string()))?;
            Some(dt)
        }
        None => None,
    };
    let end_date_time = match payload.end_date_time{
        Some(ref s) => {
            let dt = s.parse::<DateTime<Utc>>().map_err(|_| AppError::BadRequest("Invalid end date format. Use ISO 8601".to_string()))?;
            Some(dt)
        }
        None => None,
    };

    let event_id = create_event(
        &state.db_pool,
        &payload.title,
        payload.description_event,
        start_date_time,
        end_date_time,
        payload.location,
        payload.color
    ).await?;

    let permissions = EventPermissions::new()
        .add_permission(EventPermissions::OWNER)
        .add_permission(EventPermissions::ADMIN)
        .get_bits(); 
    
    let _ = add_member(&state.db_pool, user.user_id, event_id, permissions).await?;

    let event = get_event_by_id(&state.db_pool, event_id).await?;
    
    let created_by = check_user_permissions(&state.db_pool, &event, &user, EventPermissions::OWNER).await?;

    let event_response = CreateEventResponse {
        event_id: event.event_id.to_string(),
        title: event.title,
        description_event: event.description_event,
        location: event.location,
        start_date_time: event.start_date_time.map(|dt| dt.to_rfc3339()),
        end_date_time: event.end_date_time.map(|dt| dt.to_rfc3339()),
        color: event.color,
        created_by: created_by.to_string(), 
        created_at: event.created_at.to_rfc3339(),
        status_event: event.status_event.to_string()
    };

    Ok((StatusCode::CREATED, Json(event_response)))
}

#[utoipa::path(
    get,
    path = "/events",
    tag = "Event",
    security(
        ("bearerAuth" = [])
    ),
    responses(
        (status = 200, description = "Event details retrieved", body = GetEventsResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 404, description = "User or event not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn get_user_events_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    query: Query<GetEvents>
) -> Result<impl IntoResponse, AppError> {
    let user = get_user_for_handler_from_token(&state.db_pool, auth.token()).await?;

    let events = get_user_events(&state.db_pool, user.user_id, query.limit, query.offset, query.status.clone()).await?;

    Ok((StatusCode::OK, Json(json!({"events": events}))))
}

#[utoipa::path(
    get,
    path = "/events/{event_id}",
    tag = "Event",
    security(
        ("bearerAuth" = [])
    ),
    params(
        ("event_id" = i64, Path, description = "Event ID")
    ),
    responses(
        (status = 200, description = "Event details retrieved", body = GetEventDetailedResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "User not in event", body = ErrorResponse),
        (status = 404, description = "Event not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn get_detailed_event_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    Path(event_id): Path<i64>
) -> Result<impl IntoResponse, AppError> {
    let user = get_user_for_handler_from_token(&state.db_pool, auth.token()).await?;

    let event = get_event_by_id(&state.db_pool, event_id).await?;

    let is_member = check_user_in_event(&state.db_pool, event.event_id, user.user_id).await?;
    if !is_member {
        return Err(AppError::UserNotInEvent("User not in event".to_string()));
    }

    let members = get_users_in_event(&state.db_pool, event.event_id).await?;

    let permissions = get_user_permissions(&state.db_pool, event.event_id, user.user_id).await?;

    let _ = match check_user_permissions(&state.db_pool, &event, &user, EventPermissions::OWNER).await {
        Ok(true) => {
            let invite_token = create_event_token(&state.db_pool, event_id, 144).await?;
            let invite_link = format!("https://kruug.netlify.app/invite?token={}", invite_token);
            return Ok((StatusCode::OK, Json(json!(GetEventDetailedResponse {
                    event: event, 
                    invite_link: Some(invite_link), 
                    members: members,
                    permissions: format!("{:b}", permissions.get_bits())
            }))));
        },
        Ok(false) => {
            return Ok((StatusCode::OK, Json(json!(GetEventDetailedResponse {
                    event: event, 
                    invite_link: None, 
                    members: members,
                    permissions: format!("{:b}", permissions.get_bits())
            }))));
        },
        Err(_) => return Err(AppError::Internal("Error check user permissions".to_string())),
    };
}

#[utoipa::path(
    put,
    path = "/events/{event_id}",
    tag = "Event",
    security(
        ("bearerAuth" = [])
    ),
    request_body = UpdateEventRequest,
    responses(
        (status = 200, description = "Event updated", body = Events),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 403, description = "User doesn't have permission to update event or not in event or token invalid", body = ErrorResponse),
        (status = 404, description = "User or event not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn update_event_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    Path(event_id): Path<i64>,
    Json(payload): Json<UpdateEventRequest>,
) -> Result<impl IntoResponse, AppError> {
    let user = get_user_for_handler_from_token(&state.db_pool, auth.token()).await?;

    let event = get_event_by_id(&state.db_pool, event_id).await?;

    let is_member = check_user_in_event(&state.db_pool, event.event_id, user.user_id).await?;
    if !is_member {
        return Err(AppError::UserNotInEvent("User not in event".to_string()));
    }

    let _ = match check_user_permissions(&state.db_pool, &event, &user, EventPermissions::OWNER).await {
        Ok(true) => true,
        Ok(false) => return Err(AppError::UserNotInEvent("User doesn't have permission to invite".to_string())),
        Err(e) => return Err(e),
    };

    update_event(
        &state.db_pool, 
        event_id, 
        payload.title, 
        payload.description_event, 
        payload.start_date_time.and_then(|s| s.parse::<DateTime<Utc>>().ok()), 
        payload.end_date_time.and_then(|s| s.parse::<DateTime<Utc>>().ok()), 
        payload.color,
        payload.location
    ).await?;

    let event_new = get_event_by_id(&state.db_pool, event_id).await?;

    Ok((StatusCode::OK, Json(json!({"event" : event_new}))))
}

#[utoipa::path(
    delete,
    path = "/events/{event_id}",
    tag = "Event",
    security(
        ("bearerAuth" = [])
    ),
    params(
        ("event_id" = i64, Path, description = "Event ID")
    ),
    responses(
        (status = 204, description = "Event deleted successfully"),
        (status = 403, description = "Forbidden - not enough permissions", body = ErrorResponse),
        (status = 404, description = "Event not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn delete_event_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    Path(event_id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    let user = get_user_for_handler_from_token(&state.db_pool, auth.token()).await?;
    
    let has_permission = has_permission(&state.db_pool, event_id, user.user_id, EventPermissions::OWNER).await?;
    if !has_permission {
        return Err(AppError::Forbidden("Only event owner can delete the event".to_string()));
    }
    
    delete_event(&state.db_pool, event_id).await?;
    
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    get,
    path = "/events/{event_id}/planning",
    tag = "Event",
    security(
        ("bearerAuth" = [])
    ),
    params(
        ("event_id" = i64, Path, description = "Event ID")
    ),
    responses(
        (status = 200, description = "Modules retrieved successfully", body = PlanningModulesResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "User not in event", body = ErrorResponse),
        (status = 404, description = "Event not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn get_modules_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    Path(event_id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    // 1. Получаем пользователя из токена
    let user = get_user_for_handler_from_token(&state.db_pool, auth.token()).await?;
    
    // 2. Проверяем, что пользователь состоит в событии
    let is_in_event = check_user_in_event(&state.db_pool, event_id, user.user_id).await?;
    if !is_in_event {
        return Err(AppError::Forbidden("User not in event".to_string()));
    }
    
    let mut modules: Vec<PlanningModule> = Vec::new();
    
    // 3. Получаем все опросы
    let polls = get_event_polls(&state.db_pool, event_id).await?;
    for poll in polls {
        let poll_data = get_poll_with_votes(&state.db_pool, poll.poll_id, user.user_id).await?;
        
        modules.push(PlanningModule::Poll {
            id: poll.poll_id.to_string(),
            title: poll.question,
            data: PollModuleData {
                options: poll_data.options,
                multiple_choice: poll.multiple_choice,
                votes: poll_data.votes,
                votes_count: poll_data.votes_count,
                own_vote: poll_data.own_vote,
            },
        });
    }
    
    // 4. Получаем все списки вещей
    let item_lists = get_event_item_lists(&state.db_pool, event_id).await?;
    for item_list in item_lists {
        let items: Vec<ItemListItemData> = item_list.items
            .into_iter()
            .map(|item| ItemListItemData {
                id: item.item_id.to_string(),
                text: item.item_text,
                assigned_user_id: item.assigned_user_id.map(|id| id.to_string()),
                assigned_user_name: item.assigned_user_name,
            })
            .collect();
        
        modules.push(PlanningModule::ItemList {
            id: item_list.item_list_id.to_string(),
            title: item_list.title,
            data: ItemListModuleData { items },
        });
    }
    
    // 5. Получаем все списки задач
    let task_lists = get_event_task_lists(&state.db_pool, event_id).await?;
    for task_list in task_lists {
        let tasks: Vec<TaskListItemData> = task_list.items
            .into_iter()
            .map(|task| TaskListItemData {
                id: task.task_id.to_string(),
                text: task.task_text,
                assigned_user_id: task.assigned_user_id.map(|id| id.to_string()),
                assigned_user_name: task.assigned_user_name,
                completed: task.is_completed,
            })
            .collect();
        
        modules.push(PlanningModule::TaskList {
            id: task_list.task_list_id.to_string(),
            title: task_list.title,
            data: TaskListModuleData { items: tasks },
        });
    }
    
    Ok(Json(PlanningModulesResponse { modules }))
}
// pub async fn get_event_modules_handler(

//     State(state): State<Arc<AppState>>,
//     auth: TypedHeader<Authorization<Bearer>>,
//     Json(payload): Json<InviteUserToEventRequest>,
// ) -> Result<impl IntoResponse, AppError> {
//     let user = match find_user_by_token(&state.db_pool, auth.token()).await? {
//         Some(u) => u,
//         None => return Err(AppError::UserNotFound),
//     };
//     let event = get_event_by_id(&state.db_pool, payload.event_id).await?;

//     let is_member = check_user_in_event(&state.db_pool, event.event_id, user.user_id).await?;
//     if !is_member {
//         return Err(AppError::UserNotInEvent("User not in event".to_string()));
//     }

//     let _ = match check_user_permissions(&state.db_pool, &event, &user, EventPermissions::INVITE).await {
//         Ok(true) => true,
//         Ok(false) => return Err(AppError::UserNotInEvent("User doesn't have permission to invite".to_string())),
//         Err(e) => return Err(e),
//     };
//     let modules = get_event_modules(&state.db_pool, event.event_id).await?;

//     Ok((StatusCode::OK, Json(json!({"modules": event.modules}))))
// }

// #[utoipa::path(
//     post,
//     path = "/events/add_user",
//     tag = "Event",
//     security(
//         ("bearerAuth" = [])
//     ),
//     request_body = InviteUserToEventRequest,
//     responses(
//         (status = 204, description = "User added to event", body = SuccessResponse),
//         (status = 400, description = "Bad request", body = ErrorResponse),
//         (status = 403, description = "User doesn't have permission to invite or not in event", body = ErrorResponse),
//         (status = 404, description = "User or event not found", body = ErrorResponse),
//         (status = 500, description = "Internal server error", body = ErrorResponse)
//     )
// )]
// pub async fn add_user_to_event_handler(
//     State(state): State<Arc<AppState>>,
//     auth: TypedHeader<Authorization<Bearer>>,
//     query: Query<InviteUserToEventRequest>,
//     Path(path): Path<EventPaths>,
// ) -> Result<impl IntoResponse, AppError> {
//     let user = get_user_for_handler_from_token(&state.db_pool, auth.token()).await?;

//     let event = get_event_by_id(&state.db_pool, path.event_id).await?;

//     // check invite link in future
//     let _ = add_member(&state.db_pool, user.user_id, event.event_id, EventPermissions::MEMBER).await?;

//     Ok((StatusCode::NO_CONTENT, Json(json!({"success": true}))))
// }

#[utoipa::path(
    post,
    path = "/events/{event_id}/members/{user_id}",
    tag = "Event",
    security(
        ("bearerAuth" = [])
    ),
    request_body = GetEventRequest,
    responses(
        (status = 204, description = "User deleted from event", body = SuccessResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 403, description = "User doesn't have permission to invite or not in event", body = ErrorResponse),
        (status = 404, description = "User or event not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn delete_user_from_event_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    Path(path): Path<EventPaths>
) -> Result<impl IntoResponse, AppError> {
    // check user master
    let user = get_user_for_handler_from_token(&state.db_pool, auth.token()).await?;

    let event = get_event_by_id(&state.db_pool, path.event_id).await?;

    let is_member = check_user_in_event(&state.db_pool, event.event_id, user.user_id).await?;
    if !is_member {
        return Err(AppError::UserNotInEvent("User not in event".to_string()));
    }

    let _ = match check_user_permissions(&state.db_pool, &event, &user, EventPermissions::OWNER).await {
        Ok(true) => true,
        Ok(false) => return Err(AppError::UserNotInEvent("User doesn't have permission to delete member".to_string())),
        Err(e) => return Err(e),
    };
    // chech user slave
    let user_id_for_deleting = get_user_for_handler_from_id(&state.db_pool, path.user_id).await?;

    let is_member = check_user_in_event(&state.db_pool, event.event_id, user_id_for_deleting.user_id).await?;
    if !is_member {
        return Err(AppError::UserNotInEvent("User not in event".to_string()));
    }
    let _ = remove_member(&state.db_pool, user_id_for_deleting.user_id, event.event_id).await?;

    Ok((StatusCode::NO_CONTENT, Json(json!({"success": true}))))
}

#[utoipa::path(
    post,
    path = "/events/{event_id}/members/{user_id}",
    tag = "Event",
    security(
        ("bearerAuth" = [])
    ),
    request_body = UpdateUserPermissionsRequest,
    responses(
        (status = 204, description = "Updated user permissions", body = SuccessResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 403, description = "User doesn't have permission to invite or not in event", body = ErrorResponse),
        (status = 404, description = "User or event not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn update_user_permissions_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    Path(path): Path<EventPaths>,
    Json(payload): Json<UpdateUserPermissionsRequest>,
) -> Result<impl IntoResponse, AppError> {

    // for user who make request

    let user_master = get_user_for_handler_from_token(&state.db_pool, auth.token()).await?;
    
    let event = get_event_by_id(&state.db_pool, path.event_id).await?;

    let is_member = check_user_in_event(&state.db_pool, event.event_id, user_master.user_id).await?;
    if !is_member {
        return Err(AppError::UserNotInEvent("User not in event".to_string()));
    }
    let _ = match check_user_permissions(&state.db_pool, &event, &user_master, EventPermissions::OWNER).await {
        Ok(true) => true,
        Ok(false) => return Err(AppError::UserNotInEvent("User doesn't have permission to update permissions".to_string())),
        Err(e) => return Err(e),
    };

    // for user for which need to change the rights

    let user_slave = get_user_for_handler_from_id(&state.db_pool, payload.user_id).await?;

    let is_member = check_user_in_event(&state.db_pool, event.event_id, user_slave.user_id).await?;
    if !is_member {
        return Err(AppError::UserNotInEvent("User not in event".to_string()));
    }
    let _ = match check_user_permissions(&state.db_pool, &event, &user_slave, EventPermissions::OWNER).await {
        Ok(true) => true,
        Ok(false) => return Err(AppError::UserNotInEvent("User doesn't have permission to update permissions".to_string())),
        Err(e) => return Err(e),
    };

    let new_permissions = match payload.new_permissions.parse::<i32>() {
        Ok(permis) => permis,
        Err(_) => return Err(AppError::BadRequest("permissions must be number".to_string())),
    };

    // edit permissions

    let _ = update_user_permissions(&state.db_pool, event.event_id, user_slave.user_id, new_permissions).await?;

    Ok((StatusCode::NO_CONTENT, Json(json!({"success": true}))))
}

#[utoipa::path(
    post,
    path = "/events/{event_id}/join",
    tag = "Event",
    security(
        ("bearerAuth" = [])
    ),
    request_body = JoinEventRequest,
    responses(
        (status = 204, description = "Join in event successfully", body = SuccessResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 403, description = "User doesn't have permission to invite or not in event", body = ErrorResponse),
        (status = 404, description = "User or event not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn event_join_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    Json(payload): Json<JoinEventRequest>,
) -> Result<impl IntoResponse, AppError> {
    let user = get_user_for_handler_from_token(&state.db_pool, auth.token()).await?;
    
    let token_event_id = match get_event_id_by_token(&state.db_pool, &payload.invite_token).await? {
        Some(id) => id,
        None => return Err(AppError::BadRequest("Invalid or expired invite token".to_string())),
    };
    
    if token_event_id != payload.event_id {
        return Err(AppError::BadRequest("Token does not match this event".to_string()));
    }
    
    let event = get_event_by_id(&state.db_pool, payload.event_id).await?;
    
    let is_member = check_user_in_event(&state.db_pool, event.event_id, user.user_id).await?;
    if is_member {
        return Err(AppError::BadRequest("User already in event".to_string()));
    }
    
    add_member(&state.db_pool, user.user_id, event.event_id, EventPermissions::MEMBER).await?;
    
    Ok((StatusCode::OK, Json(SuccessResponse { success: true })))
}

#[utoipa::path(
    patch,
    path = "/events/{event_id}/status",
    tag = "Event",
    security(
        ("bearerAuth" = [])
    ),
    params(
        ("event_id" = i64, Path, description = "Event ID")
    ),
    request_body = UpdateEventStatusRequest,
    responses(
        (status = 200, description = "Event status updated successfully", body = SuccessResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 403, description = "Forbidden - not enough permissions", body = ErrorResponse),
        (status = 404, description = "Event not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn update_event_status_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    Path(event_id): Path<i64>,
    Json(payload): Json<UpdateEventStatusRequest>,
) -> Result<impl IntoResponse, AppError> {
    let user = get_user_for_handler_from_token(&state.db_pool, auth.token()).await?;
    
    let is_in_event = check_user_in_event(&state.db_pool, event_id, user.user_id).await?;
    if !is_in_event {
        return Err(AppError::UserNotInEvent("User is not a member of this event".to_string()));
    }
    
    let has_permission = has_permission(&state.db_pool, event_id, user.user_id, EventPermissions::OWNER).await?;
    if !has_permission {
        return Err(AppError::Forbidden("Only event owner can change event status".to_string()));
    }
    
    update_event_status(&state.db_pool, event_id, payload.status).await?;
    
    Ok((StatusCode::OK, Json(SuccessResponse { success: true })))
}

use tokio::fs;
use std::path::PathBuf;
use axum::http::header;
use axum_extra::extract::Multipart;
use blake3;

const EVENT_UPLOAD_DIR: &str = "uploads/event_avatars";

fn get_event_avatar_dir(event_id: i64) -> PathBuf {
    PathBuf::from(EVENT_UPLOAD_DIR).join(format!("event_{}", event_id))
}

async fn compute_event_avatar_etag(event_id: i64) -> Result<String, AppError> {
    let event_dir = get_event_avatar_dir(event_id);
    
    let mut avatar_path = None;
    if let Ok(mut entries) = fs::read_dir(&event_dir).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            let name = entry.file_name();
            if name.to_string_lossy().starts_with("avatar.") {
                avatar_path = Some(entry.path());
                break;
            }
        }
    }
    
    let path = match avatar_path {
        Some(p) => p,
        None => return Ok("\"\"".to_string()),
    };
    
    let data = fs::read(&path).await.map_err(|e| {
        tracing::error!("Failed to read file: {}", e);
        AppError::Internal("Failed to read file".to_string())
    })?;
    let hash = blake3::hash(&data);
    Ok(format!("\"{}\"", hash.to_hex()))
}

#[utoipa::path(
    post,
    path = "/events/{event_id}/avatar",
    tag = "Event",
    security(
        ("bearerAuth" = [])
    ),
    params(
        ("event_id" = i64, Path, description = "Event ID")
    ),
    responses(
        (status = 200, description = "Avatar uploaded successfully", body = SuccessResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "Event not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn upload_event_avatar_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    Path(event_id): Path<i64>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, AppError> {
    let user = get_user_for_handler_from_token(&state.db_pool, auth.token()).await?;
    
    let is_in_event = check_user_in_event(&state.db_pool, event_id, user.user_id).await?;
    if !is_in_event {
        return Err(AppError::Forbidden("User not in event".to_string()));
    }
    
    let has_permission = has_permission(&state.db_pool, event_id, user.user_id, EventPermissions::OWNER).await?;
    if !has_permission {
        return Err(AppError::Forbidden("Not enough permissions".to_string()));
    }
    
    while let Ok(Some(field)) = multipart.next_field().await {
        if field.name() != Some("avatar") {
            continue;
        }
        
        let file_name = match field.file_name() {
            Some(name) => name.to_string(),
            None => continue,
        };
        
        let ext = std::path::Path::new(&file_name)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("jpg");
        
        let data = field.bytes().await.map_err(|e| {
            tracing::error!("Failed to read file: {}", e);
            AppError::BadRequest("Failed to read file".to_string())
        })?;
        
        let event_dir = get_event_avatar_dir(event_id);
        
        fs::create_dir_all(&event_dir).await.map_err(|e| {
            tracing::error!("Failed to create event dir: {}", e);
            AppError::Internal("Failed to create directory".to_string())
        })?;
        
        if let Ok(mut entries) = fs::read_dir(&event_dir).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                let name = entry.file_name();
                if name.to_string_lossy().starts_with("avatar.") {
                    let _ = fs::remove_file(entry.path()).await;
                }
            }
        }
        
        let new_name = format!("avatar.{}", ext);
        let save_path = event_dir.join(&new_name);
        
        fs::write(&save_path, data).await.map_err(|e| {
            tracing::error!("Failed to save file: {}", e);
            AppError::Internal("Failed to save file".to_string())
        })?;
        
        return Ok((StatusCode::OK, Json(SuccessResponse { success: true })));
    }
    
    Err(AppError::BadRequest("No file provided".to_string()))
}

#[utoipa::path(
    get,
    path = "/events/{event_id}/avatar",
    tag = "Event",
    params(
        ("event_id" = i64, Path, description = "Event ID")
    ),
    responses(
        (status = 200, description = "Get avatar successfully"),
        (status = 304, description = "Not modified"),
        (status = 404, description = "Avatar not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn get_event_avatar_handler(
    headers: HeaderMap,
    Path(event_id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    let current_etag = compute_event_avatar_etag(event_id).await?;
    
    if let Some(if_none_match) = headers.get(header::IF_NONE_MATCH) {
        if if_none_match.to_str().unwrap_or("") == current_etag {
            return Ok(StatusCode::NOT_MODIFIED.into_response());
        }
    }
    
    let event_dir = get_event_avatar_dir(event_id);
    
    if !event_dir.exists() {
        return Err(AppError::NotFound("Avatar not found".to_string()));
    }
    
    let mut avatar_path = None;
    if let Ok(mut entries) = fs::read_dir(&event_dir).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            let name = entry.file_name();
            if name.to_string_lossy().starts_with("avatar.") {
                avatar_path = Some(entry.path());
                break;
            }
        }
    }
    
    let path = avatar_path.ok_or_else(|| AppError::NotFound("Avatar not found".to_string()))?;
    
    let mime = mime_guess::from_path(&path).first_or_octet_stream();
    let data = fs::read(&path).await.map_err(|e| {
        tracing::error!("Failed to read file: {}", e);
        AppError::Internal("Failed to read file".to_string())
    })?;
    
    let mut response = (StatusCode::OK, data).into_response();
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        mime.to_string().parse().unwrap(),
    );
    response.headers_mut().insert(
        header::ETAG,
        current_etag.parse().unwrap(),
    );
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        "public, max-age=3600".parse().unwrap(),
    );
    
    Ok(response)
}

#[utoipa::path(
    delete,
    path = "/events/{event_id}/avatar",
    tag = "Event",
    security(
        ("bearerAuth" = [])
    ),
    params(
        ("event_id" = i64, Path, description = "Event ID")
    ),
    responses(
        (status = 200, description = "Avatar deleted successfully", body = SuccessResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "Avatar not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn delete_event_avatar_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    Path(event_id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    let user = get_user_for_handler_from_token(&state.db_pool, auth.token()).await?;
    
    let has_permission = has_permission(&state.db_pool, event_id, user.user_id, EventPermissions::OWNER).await?;
    if !has_permission {
        return Err(AppError::Forbidden("Not enough permissions".to_string()));
    }
    
    let event_dir = get_event_avatar_dir(event_id);
    
    if let Ok(mut entries) = fs::read_dir(&event_dir).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            let name = entry.file_name();
            if name.to_string_lossy().starts_with("avatar.") {
                let _ = fs::remove_file(entry.path()).await;
            }
        }
    }
    
    Ok((StatusCode::OK, Json(SuccessResponse { success: true })))
}

// Plainning module handlers


// pub async fn get_event_polls_handler(
//     State(state): State<Arc<AppState>>,
//     auth: TypedHeader<Authorization<Bearer>>,
//     Json(payload): Json<CreatePollRequest>,
// ) -> Result<impl IntoResponse, AppError> {
//     let user = match find_user_by_token(&state.db_pool, auth.token()).await? {
//         Some(u) => u,
//         None => return Err(AppError::UserNotFound),
//     };
//     let event = get_event_by_id(&state.db_pool, payload.event_id).await?;

//     let is_member = check_user_in_event(&state.db_pool, event.event_id, user.user_id).await?;
//     if !is_member {
//         return Err(AppError::UserNotInEvent("User not in event".to_string()));
//     }
//     let _ = match check_user_permissions(&state.db_pool, &event, &user, EventPermissions::OWNER).await {
//         Ok(true) => true,
//         Ok(false) => return Err(AppError::UserNotInEvent("User doesn't have permission to update permissions".to_string())),
//         Err(e) => return Err(e),
//     };
//     let polls = get_event_polls(&state.db_pool, event.event_id).await?;

//     Ok((StatusCode::CREATED, Json(json!({"polls": polls}))))
// }

// #[debug_handler]
// pub async fn test_handler(
//     State(state): State<Arc<AppState>>,
//     auth: TypedHeader<Authorization<Bearer>>,
//     Json(payload): Json<CreateEventRequest>,
// ) -> Result<impl IntoResponse, AppError> {
//     Ok((StatusCode::CREATED, Json(json!({"status": "ok"}))))
// }
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        test_utils::setup_test_db,
        data_base::user_db::{create_user_db, create_token},
        data_base::event_db::{create_event, add_member, get_event_members},
        permissions::EventPermissions,
        user_store::UserStore,
        secrets::verification::VerificationStore,
    };
    use axum::{Router, body::Body, http::Request};
    use tower::ServiceExt;
    use chrono::Utc;
    use tokio::sync::broadcast;
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use http_body_util::BodyExt;
    use serde_json::json;

    // ---- helpers ----

    async fn create_state(pool: sqlx::PgPool) -> Arc<AppState> {
        Arc::new(AppState {
            tx: broadcast::channel(10).0,
            user_store: Arc::new(Mutex::new(UserStore::new())),
            verification_store: Arc::new(Mutex::new(VerificationStore::new())),
            db_pool: pool,
        })
    }

    async fn new_user_and_token(pool: &sqlx::PgPool, name: &str, email: &str, token_str: &str) -> (i64, String) {
        let uid = create_user_db(pool, name, email, name, &None, &None).await.unwrap();
        create_token(pool, uid, token_str, Utc::now() + chrono::Duration::hours(1)).await.unwrap();
        (uid, token_str.to_string())
    }

    async fn new_event(pool: &sqlx::PgPool) -> i64 {
        create_event(pool, "Test Event", Some("Desc".into()), None, None, Some("Room".into()), "#123456".into())
            .await.unwrap()
    }

    fn app_with_routes(state: Arc<AppState>) -> Router {
        Router::new()
            .route("/events", routing::post(create_event_handler))
            .route("/events/:event_id", routing::get(get_detailed_event_handler))
            .route("/events/:event_id", routing::put(update_event_handler))
            .route("/events/:event_id", routing::delete(delete_event_handler))
            .route("/events/:event_id/join", routing::post(event_join_handler))
            .route("/events/:event_id/status", routing::patch(update_event_status_handler))
            .route("/events/:event_id/avatar", routing::post(upload_event_avatar_handler))
            .route("/events/:event_id/avatar", routing::get(get_event_avatar_handler))
            .route("/events/:event_id/avatar", routing::delete(delete_event_avatar_handler))
            .with_state(state)
    }

    // -----------------------------------------------------------
    // create_event_handler
    // -----------------------------------------------------------
    #[tokio::test]
    async fn create_event_success() -> anyhow::Result<()> {
        let pool = setup_test_db().await;
        let (uid, token) = new_user_and_token(&pool, "creator", "creator@test.com", "token_creator").await;
        let state = create_state(pool).await;
        let app = app_with_routes(state);

        let payload = json!({
            "title": "Birthday",
            "description_event": "Party",
            "start_date_time": null,
            "end_date_time": null,
            "color": "#ff0000",
            "location": "Home"
        });
        let req = Request::builder()
            .method("POST")
            .uri("/events")
            .header("Authorization", format!("Bearer {}", token))
            .header("content-type", "application/json")
            .body(Body::from(payload.to_string()))?;
        let resp = app.oneshot(req).await?;
        assert_eq!(resp.status(), StatusCode::CREATED);
        Ok(())
    }

    #[tokio::test]
    async fn create_event_invalid_date() -> anyhow::Result<()> {
        let pool = setup_test_db().await;
        let (_uid, token) = new_user_and_token(&pool, "creator2", "creator2@test.com", "token_creator2").await;
        let state = create_state(pool).await;
        let app = app_with_routes(state);

        let payload = json!({
            "title": "Test",
            "start_date_time": "not-a-date",
            "end_date_time": null,
            "color": "#000000"
        });
        let req = Request::builder()
            .method("POST")
            .uri("/events")
            .header("Authorization", format!("Bearer {}", token))
            .header("content-type", "application/json")
            .body(Body::from(payload.to_string()))?;
        let resp = app.oneshot(req).await?;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        Ok(())
    }

    // -----------------------------------------------------------
    // get_detailed_event_handler
    // -----------------------------------------------------------
    async fn setup_detailed(pool: &sqlx::PgPool) -> (i64, String, i64) {
        let (uid, token) = new_user_and_token(pool, "det_user", "det@test.com", "det_token").await;
        let eid = new_event(pool).await;
        add_member(pool, uid, eid, EventPermissions::OWNER).await.unwrap();
        (eid, token, uid)
    }

    #[tokio::test]
    async fn get_detailed_event_success() -> anyhow::Result<()> {
        let pool = setup_test_db().await;
        let (eid, token, _) = setup_detailed(&pool).await;
        let state = create_state(pool).await;
        let app = app_with_routes(state);

        let req = Request::builder()
            .method("GET")
            .uri(&format!("/events/{}", eid))
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())?;
        let resp = app.oneshot(req).await?;
        assert_eq!(resp.status(), StatusCode::OK);
        Ok(())
    }

    #[tokio::test]
    async fn get_detailed_user_not_in_event() -> anyhow::Result<()> {
        let pool = setup_test_db().await;
        let (_eid, _token, _uid) = setup_detailed(&pool).await;
        let (stranger_id, stranger_token) = new_user_and_token(&pool, "stranger", "stranger@test.com", "str_token").await;
        let eid2 = new_event(&pool).await; // stranger not added
        let state = create_state(pool).await;
        let app = app_with_routes(state);

        let req = Request::builder()
            .method("GET")
            .uri(&format!("/events/{}", eid2))
            .header("Authorization", format!("Bearer {}", stranger_token))
            .body(Body::empty())?;
        let resp = app.oneshot(req).await?;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
        Ok(())
    }

    #[tokio::test]
    async fn get_detailed_event_not_found() -> anyhow::Result<()> {
        let pool = setup_test_db().await;
        let (uid, token) = new_user_and_token(&pool, "ghost", "ghost@test.com", "ghost_token").await;
        let state = create_state(pool).await;
        let app = app_with_routes(state);

        let req = Request::builder()
            .method("GET")
            .uri("/events/99999")
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())?;
        let resp = app.oneshot(req).await?;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
        Ok(())
    }

    // -----------------------------------------------------------
    // update_event_handler
    // -----------------------------------------------------------
    #[tokio::test]
    async fn update_event_owner() -> anyhow::Result<()> {
        let pool = setup_test_db().await;
        let (eid, token, _) = setup_detailed(&pool).await;
        let state = create_state(pool).await;
        let app = app_with_routes(state);

        let payload = json!({"title": "Updated title"});
        let req = Request::builder()
            .method("PUT")
            .uri(&format!("/events/{}", eid))
            .header("Authorization", format!("Bearer {}", token))
            .header("content-type", "application/json")
            .body(Body::from(payload.to_string()))?;
        let resp = app.oneshot(req).await?;
        assert_eq!(resp.status(), StatusCode::OK);
        Ok(())
    }

    #[tokio::test]
    async fn update_event_not_owner() -> anyhow::Result<()> {
        let pool = setup_test_db().await;
        let (eid, _, owner_uid) = setup_detailed(&pool).await;
        let (member_uid, member_token) = new_user_and_token(&pool, "member", "member@test.com", "member_token").await;
        add_member(&pool, member_uid, eid, EventPermissions::MEMBER).await.unwrap();
        let state = create_state(pool).await;
        let app = app_with_routes(state);

        let payload = json!({"title": "Hack"});
        let req = Request::builder()
            .method("PUT")
            .uri(&format!("/events/{}", eid))
            .header("Authorization", format!("Bearer {}", member_token))
            .header("content-type", "application/json")
            .body(Body::from(payload.to_string()))?;
        let resp = app.oneshot(req).await?;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN); // UserNotInEvent с сообщением о правах
        Ok(())
    }

    // -----------------------------------------------------------
    // delete_event_handler
    // -----------------------------------------------------------
    #[tokio::test]
    async fn delete_event_owner() -> anyhow::Result<()> {
        let pool = setup_test_db().await;
        let (eid, token, _) = setup_detailed(&pool).await;
        let state = create_state(pool).await;
        let app = app_with_routes(state);

        let req = Request::builder()
            .method("DELETE")
            .uri(&format!("/events/{}", eid))
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())?;
        let resp = app.oneshot(req).await?;
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
        Ok(())
    }

    #[tokio::test]
    async fn delete_event_not_owner() -> anyhow::Result<()> {
        let pool = setup_test_db().await;
        let (eid, _, _owner_uid) = setup_detailed(&pool).await;
        let (member_uid, member_token) = new_user_and_token(&pool, "mem2", "mem2@test.com", "mem2_token").await;
        add_member(&pool, member_uid, eid, EventPermissions::MEMBER).await.unwrap();
        let state = create_state(pool).await;
        let app = app_with_routes(state);

        let req = Request::builder()
            .method("DELETE")
            .uri(&format!("/events/{}", eid))
            .header("Authorization", format!("Bearer {}", member_token))
            .body(Body::empty())?;
        let resp = app.oneshot(req).await?;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
        Ok(())
    }

    // -----------------------------------------------------------
    // event_join_handler
    // -----------------------------------------------------------
    #[tokio::test]
    async fn join_event_with_valid_token() -> anyhow::Result<()> {
        let pool = setup_test_db().await;
        let (owner_uid, owner_token) = new_user_and_token(&pool, "owner_join", "owner_join@test.com", "owner_join_tok").await;
        let eid = new_event(&pool).await;
        add_member(&pool, owner_uid, eid, EventPermissions::OWNER).await.unwrap();
        let invite_token = create_event_token(&pool, eid, 1).await.unwrap();

        let (joiner_uid, joiner_token) = new_user_and_token(&pool, "joiner", "joiner@test.com", "joiner_tok").await;
        let state = create_state(pool).await;
        let app = app_with_routes(state);

        let payload = json!({"event_id": eid, "invite_token": invite_token});
        let req = Request::builder()
            .method("POST")
            .uri(&format!("/events/{}/join", eid))
            .header("Authorization", format!("Bearer {}", joiner_token))
            .header("content-type", "application/json")
            .body(Body::from(payload.to_string()))?;
        let resp = app.oneshot(req).await?;
        assert_eq!(resp.status(), StatusCode::OK);
        Ok(())
    }

    #[tokio::test]
    async fn join_event_invalid_token() -> anyhow::Result<()> {
        let pool = setup_test_db().await;
        let (uid, token) = new_user_and_token(&pool, "bad_joiner", "bad_joiner@test.com", "bad_join_tok").await;
        let eid = new_event(&pool).await;
        let state = create_state(pool).await;
        let app = app_with_routes(state);

        let payload = json!({"event_id": eid, "invite_token": "fake_token"});
        let req = Request::builder()
            .method("POST")
            .uri(&format!("/events/{}/join", eid))
            .header("Authorization", format!("Bearer {}", token))
            .header("content-type", "application/json")
            .body(Body::from(payload.to_string()))?;
        let resp = app.oneshot(req).await?;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        Ok(())
    }

    // -----------------------------------------------------------
    // update_event_status_handler
    // -----------------------------------------------------------
    #[tokio::test]
    async fn update_event_status_owner() -> anyhow::Result<()> {
        let pool = setup_test_db().await;
        let (eid, token, _) = setup_detailed(&pool).await;
        let state = create_state(pool).await;
        let app = app_with_routes(state);

        let payload = json!({"status": "archived"});
        let req = Request::builder()
            .method("PATCH")
            .uri(&format!("/events/{}/status", eid))
            .header("Authorization", format!("Bearer {}", token))
            .header("content-type", "application/json")
            .body(Body::from(payload.to_string()))?;
        let resp = app.oneshot(req).await?;
        assert_eq!(resp.status(), StatusCode::OK);
        Ok(())
    }

    #[tokio::test]
    async fn update_event_status_not_owner() -> anyhow::Result<()> {
        let pool = setup_test_db().await;
        let (eid, _, _owner_uid) = setup_detailed(&pool).await;
        let (member_uid, member_token) = new_user_and_token(&pool, "status_mem", "stat_mem@test.com", "stat_mem_tok").await;
        add_member(&pool, member_uid, eid, EventPermissions::MEMBER).await.unwrap();
        let state = create_state(pool).await;
        let app = app_with_routes(state);

        let payload = json!({"status": "archived"});
        let req = Request::builder()
            .method("PATCH")
            .uri(&format!("/events/{}/status", eid))
            .header("Authorization", format!("Bearer {}", member_token))
            .header("content-type", "application/json")
            .body(Body::from(payload.to_string()))?;
        let resp = app.oneshot(req).await?;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
        Ok(())
    }

    // -----------------------------------------------------------
    // upload / get / delete event avatar
    // -----------------------------------------------------------
    // (Эти тесты требуют работы с multipart, поэтому я приведу один базовый сценарий)
    #[tokio::test]
    async fn upload_event_avatar_success() -> anyhow::Result<()> {
        let pool = setup_test_db().await;
        let (eid, token, _) = setup_detailed(&pool).await;
        let state = create_state(pool).await;
        let app = app_with_routes(state);

        // собираем multipart с изображением 1x1 PNG
        let boundary = "testboundary";
        let body = format!(
            "--{0}\r\nContent-Disposition: form-data; name=\"avatar\"; filename=\"test.png\"\r\nContent-Type: image/png\r\n\r\n\x89PNG\r\n\x1a\n--{0}--\r\n",
            boundary
        );
        let req = Request::builder()
            .method("POST")
            .uri(&format!("/events/{}/avatar", eid))
            .header("Authorization", format!("Bearer {}", token))
            .header("content-type", format!("multipart/form-data; boundary={}", boundary))
            .body(Body::from(body))?;
        let resp = app.oneshot(req).await?;
        assert_eq!(resp.status(), StatusCode::OK);
        Ok(())
    }

    #[tokio::test]
    async fn get_event_avatar_not_found() -> anyhow::Result<()> {
        let pool = setup_test_db().await;
        let (eid, _token, _) = setup_detailed(&pool).await;
        let state = create_state(pool).await;
        let app = app_with_routes(state);

        let req = Request::builder()
            .method("GET")
            .uri(&format!("/events/{}/avatar", eid))
            .body(Body::empty())?;
        let resp = app.oneshot(req).await?;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
        Ok(())
    }

    #[tokio::test]
    async fn delete_event_avatar_no_permission() -> anyhow::Result<()> {
        let pool = setup_test_db().await;
        let (eid, _, _owner_uid) = setup_detailed(&pool).await;
        let (member_uid, member_token) = new_user_and_token(&pool, "mem_avatar", "mem_avatar@test.com", "mem_av_tok").await;
        add_member(&pool, member_uid, eid, EventPermissions::MEMBER).await.unwrap();
        let state = create_state(pool).await;
        let app = app_with_routes(state);

        let req = Request::builder()
            .method("DELETE")
            .uri(&format!("/events/{}/avatar", eid))
            .header("Authorization", format!("Bearer {}", member_token))
            .body(Body::empty())?;
        let resp = app.oneshot(req).await?;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
        Ok(())
    }
}
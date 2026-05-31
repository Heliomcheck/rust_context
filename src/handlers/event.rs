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

    match check_user_permissions(&state.db_pool, &event, &user, EventPermissions::OWNER).await {
        Ok(true) => {
            let invite_token = match get_or_create_event_token(&state.db_pool, event_id).await {
                Ok(token) => token,
                Err(e) => {
                    tracing::error!("Failed to get/create token: {}", e);
                    return Err(AppError::Internal("Failed to create invite token".to_string()));
                }
            };
            
            let is_valid = is_event_token_valid(&state.db_pool, &invite_token).await?;
            
            let final_token = if !is_valid {
                match create_event_token(&state.db_pool, event_id, 300).await {
                    Ok(token) => token,
                    Err(e) => {
                        tracing::error!("Failed to create new token: {}", e);
                        return Err(AppError::Internal("Failed to create invite token".to_string()));
                    }
                }
            } else {
                invite_token
            };
            
            let invite_link = format!("https://kruug.netlify.app/invite?token={}", final_token);
            
            Ok((StatusCode::OK, Json(json!(GetEventDetailedResponse {
                event,
                invite_link: Some(invite_link),
                members,
                permissions: format!("{:b}", permissions.get_bits())
            }))))
        }
        Ok(false) => {
            Ok((StatusCode::OK, Json(json!(GetEventDetailedResponse {
                event,
                invite_link: None,
                members,
                permissions: format!("{:b}", permissions.get_bits())
            }))))
        }
        Err(e) => {
            tracing::error!("Error checking user permissions: {:?}", e);
            Err(AppError::Internal("Error check user permissions".to_string()))
        }
    }
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
    
    Ok((StatusCode::OK, Json(SuccessResponse { success: true })))
}

#[utoipa::path(
    get,
    path = "/events/:event_id/planning",
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
//test
//#[cfg(test)]
//         assert_eq!(response.status(), StatusCode::CREATED);
//     }

//     #[tokio::test]
//     async fn test_create_event_handler_unauthorized() {
//         let app = create_test_app().await;

//         let payload = json!({
//             "title": "Birthday Party",
//             "description": "Test event",
//             "start_date_time": null,
//             "end_date_time": null,
//             "color": 1
//         });

//         let request = Request::builder()
//             .method("POST")
//             .uri("/events")
//             .header("content-type", "application/json")
//             .header("authorization", "Bearer invalid_token")
//             .body(Body::from(payload.to_string()))
//             .unwrap();

//         let response = app.oneshot(request).await.unwrap();

//         assert!(response.status() == StatusCode::NOT_FOUND
//             || response.status() == StatusCode::UNAUTHORIZED);
//     }

//     #[tokio::test]
//     async fn test_create_event_invalid_date() {
//         let app = create_test_app().await;

//         let state = app.state::<Arc<AppState>>().unwrap();

//         let token = create_test_user_and_token(&state.db_pool).await;

//         let payload = json!({
//             "title": "Birthday Party",
//             "description": "Test event",
//             "start_date_time": "invalid_date",
//             "end_date_time": null,
//             "color": 1
//         });

//         let request = Request::builder()
//             .method("POST")
//             .uri("/events")
//             .header("content-type", "application/json")
//             .header("authorization", format!("Bearer {}", token))
//             .body(Body::from(payload.to_string()))
//             .unwrap();

//         let response = app.oneshot(request).await.unwrap();

//         assert_eq!(response.status(), StatusCode::BAD_REQUEST);
//     }

    // #[tokio::test]
    // async fn test_get_user_events_handler() {
    //     let app = create_test_app().await;

    //     let state = app.state::<Arc<AppState>>().unwrap();

    //     let token = create_test_user_and_token(&state.db_pool).await;

    //     let request = Request::builder()
    //         .method("GET")
    //         .uri("/events/user")
    //         .header("authorization", format!("Bearer {}", token))
    //         .body(Body::empty())
    //         .unwrap();

    //     let response = app.oneshot(request).await.unwrap();

    //     assert_eq!(response.status(), StatusCode::OK);
    // }
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        test_utils::setup_test_db,
        data_base::user_db::{create_user_db, create_token},
        data_base::event_db::{create_event, add_member},
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

    /// Создаёт тестовое приложение с зарегистрированными роутами
    async fn test_app() -> (Router, Arc<AppState>, i64, String) {
        let pool = setup_test_db().await;
        
        // Создаём пользователя
        let user_id = create_user_db(
            &pool,
            "testuser",
            "test@example.com",
            "Test User",
            &None,
            &None,
        ).await.unwrap();
        
        let token = "test_valid_token";
        create_token(&pool, user_id, token, Utc::now() + chrono::Duration::hours(1))
            .await
            .unwrap();
        
        // Создаём событие
        let event_id = create_event(
            &pool,
            "Test Event",
            Some("Test Description".to_string()),
            None,
            None,
            Some("jfkdfj".to_string()),
            "#123456".to_string(),
        ).await.unwrap();
        
        // Добавляем пользователя в событие
        add_member(&pool, user_id, event_id, EventPermissions::OWNER).await.unwrap();
        
        let state = Arc::new(AppState {
            tx: broadcast::channel(10).0,
            user_store: Arc::new(Mutex::new(UserStore::new())),
            verification_store: Arc::new(Mutex::new(VerificationStore::new())),
            db_pool: pool,
        });
        
        {
            let mut store = state.user_store.lock().await;
            store.load_from_db(&state.db_pool).await.unwrap();
        }
        
        let app = Router::new()
            .route("/events/{event_id}", routing::get(get_detailed_event_handler))
            .with_state(state.clone());
        
        (app, state, event_id, token.to_string())
    }

    /// Тест 1: Успешное получение события
    #[tokio::test]
    async fn test_get_detailed_event_success() {
        let (app, _state, event_id, token) = test_app().await;
        
        let request = Request::builder()
            .method("GET")
            .uri(&format!("/events/{}", event_id))
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();
        
        let response = app.oneshot(request).await.unwrap();
        let status = response.status();
        let (_, body) = response.into_parts();
        let bytes = body.collect().await.unwrap().to_bytes();
        let body_str = String::from_utf8_lossy(&bytes);
        
        println!("Status: {}", status);
        println!("Body: {}", body_str);
        
        assert_eq!(status, StatusCode::OK, "Expected 200 OK, got {}", status);
        
        let json: serde_json::Value = serde_json::from_str(&body_str).unwrap();
        assert!(json.get("event").is_some(), "Missing 'event' field");
        assert!(json.get("members").is_some(), "Missing 'members' field");
        assert!(json.get("permissions").is_some(), "Missing 'permissions' field");
    }

    /// Тест 2: Пользователь не в событии -> 403
    #[tokio::test]
    async fn test_get_detailed_event_user_not_in_event() {
        let pool = setup_test_db().await;
        
        let user_id = create_user_db(
            &pool,
            "testuser2",
            "test2@example.com",
            "Test User 2",
            &None,
            &None,
        ).await.unwrap();
        
        let token = "test_token_2";
        create_token(&pool, user_id, token, Utc::now() + chrono::Duration::hours(1))
            .await
            .unwrap();
        
        let event_id = create_event(
            &pool,
            "Test Event 2",
            None,
            None,
            None,
            Some("jfkdfj".to_string()),
            "#123456".to_string(),
        ).await.unwrap();
        
        // НЕ добавляем пользователя в событие
        
        let state = Arc::new(AppState {
            tx: broadcast::channel(10).0,
            user_store: Arc::new(Mutex::new(UserStore::new())),
            verification_store: Arc::new(Mutex::new(VerificationStore::new())),
            db_pool: pool,
        });
        
        {
            let mut store = state.user_store.lock().await;
            store.load_from_db(&state.db_pool).await.unwrap();
        }
        
        let app = Router::new()
            .route("/events/{event_id}", routing::get(get_detailed_event_handler))
            .with_state(state);
        
        let request = Request::builder()
            .method("GET")
            .uri(&format!("/events/{}", event_id))
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();
        
        let response = app.oneshot(request).await.unwrap();
        let status = response.status();
        
        println!("Status: {}", status);
        
        assert_eq!(status, StatusCode::FORBIDDEN, "Expected 403 Forbidden, got {}", status);
    }

    /// Тест 3: Событие не существует -> 404
    #[tokio::test]
    async fn test_get_detailed_event_not_found() {
        let pool = setup_test_db().await;
        
        let user_id = create_user_db(
            &pool,
            "testuser3",
            "test3@example.com",
            "Test User 3",
            &None,
            &None,
        ).await.unwrap();
        
        let token = "test_token_3";
        create_token(&pool, user_id, token, Utc::now() + chrono::Duration::hours(1))
            .await
            .unwrap();
        
        let state = Arc::new(AppState {
            tx: broadcast::channel(10).0,
            user_store: Arc::new(Mutex::new(UserStore::new())),
            verification_store: Arc::new(Mutex::new(VerificationStore::new())),
            db_pool: pool,
        });
        
        {
            let mut store = state.user_store.lock().await;
            store.load_from_db(&state.db_pool).await.unwrap();
        }
        
        let app = Router::new()
            .route("/events/{event_id}", routing::get(get_detailed_event_handler))
            .with_state(state);
        
        let request = Request::builder()
            .method("GET")
            .uri("/events/99999")
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();
        
        let response = app.oneshot(request).await.unwrap();
        let status = response.status();
        
        println!("Status: {}", status);
        
        assert_eq!(status, StatusCode::NOT_FOUND, "Expected 404 Not Found, got {}", status);
    }

    /// Тест 4: Невалидный токен -> 401
    #[tokio::test]
    async fn test_get_detailed_event_invalid_token() {
        let pool = setup_test_db().await;
        
        // Создаём событие, которое существует
        let event_id = create_event(
            &pool,
            "Test Event",
            None,
            None,
            None,
            Some("jfkdfj".to_string()),
            "#123456".to_string(),
        ).await.unwrap();
        
        let state = Arc::new(AppState {
            tx: broadcast::channel(10).0,
            user_store: Arc::new(Mutex::new(UserStore::new())),
            verification_store: Arc::new(Mutex::new(VerificationStore::new())),
            db_pool: pool,
        });
        
        let app = Router::new()
            .route("/events/{event_id}", routing::get(get_detailed_event_handler))
            .with_state(state);
        
        let request = Request::builder()
            .method("GET")
            .uri(&format!("/events/{}", event_id))
            .header("Authorization", "Bearer invalid_token_xyz")
            .body(Body::empty())
            .unwrap();
        
        let response = app.oneshot(request).await.unwrap();
        let status = response.status();
        
        println!("Status: {}", status);
        
        assert_eq!(status, StatusCode::UNAUTHORIZED, "Expected 401 Unauthorized, got {}", status);
    }

    /// Тест 5: Нет заголовка Authorization -> 401
    #[tokio::test]
    async fn test_get_detailed_event_no_auth_header() {
        let pool = setup_test_db().await;
        
        let event_id = create_event(
            &pool,
            "Test Event",
            None,
            None,
            None,
            Some("jfkdfj".to_string()),
            "#123456".to_string(),
        ).await.unwrap();
        
        let state = Arc::new(AppState {
            tx: broadcast::channel(10).0,
            user_store: Arc::new(Mutex::new(UserStore::new())),
            verification_store: Arc::new(Mutex::new(VerificationStore::new())),
            db_pool: pool,
        });
        
        let app = Router::new()
            .route("/events/{event_id}", routing::get(get_detailed_event_handler))
            .with_state(state);
        
        let request = Request::builder()
            .method("GET")
            .uri(&format!("/events/{}", event_id))
            .body(Body::empty())
            .unwrap();
        
        let response = app.oneshot(request).await.unwrap();
        let status = response.status();
        
        println!("Status: {}", status);
        
        assert_eq!(status, StatusCode::UNAUTHORIZED, "Expected 401 Unauthorized, got {}", status);
    }
}
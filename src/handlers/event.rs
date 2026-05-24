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
        payload.color, 
    ).await?;

    let _ = add_member(&state.db_pool, user.user_id, event_id, EventPermissions::OWNER).await?;

    let event = get_event_by_id(&state.db_pool, event_id).await?;
    
    let created_by = check_user_permissions(&state.db_pool, &event, &user, EventPermissions::OWNER).await?;

    let event_response = CreateEventResponse {
        event_id: event.event_id.to_string(),
        title: event.title,
        description_event: event.description_event,
        location: Some("test".to_string()),
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

    let events = get_user_events(&state.db_pool, user.user_id, query.limit, query.offset).await?;

    Ok((StatusCode::OK, Json(json!({"events": events}))))
}

#[utoipa::path(
    get,
    path = "/event/{event_id}",
    tag = "Event",
    security(
        ("bearerAuth" = [])
    ),
    request_body = GetEventRequest,
    responses(
        (status = 200, description = "Event details retrieved", body = GetEventDetailedResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 404, description = "User or event not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn get_detailed_event_handler(
    State(state): State<Arc<AppState>>,
    auth: TypedHeader<Authorization<Bearer>>,
    Path(path): Path<EventPaths>
) -> Result<impl IntoResponse, AppError> {
    let user = get_user_for_handler_from_token(&state.db_pool, auth.token()).await?;

    let event = get_event_by_id(&state.db_pool, path.event_id).await?;

    let is_member = check_user_in_event(&state.db_pool, event.event_id, user.user_id).await?;
    if !is_member {
        return Err(AppError::UserNotInEvent("User not in event".to_string()));
    }

    let members = get_users_in_event(&state.db_pool, event.event_id).await?;

    let permissions = get_user_permissions(&state.db_pool, event.event_id, user.user_id).await?;

    let _ = match check_user_permissions(&state.db_pool, &event, &user, EventPermissions::OWNER).await {
        Ok(true) => {
            let invite_link = "http://krug.com/token_invite".to_string();// create_event_token(&state.db_pool, event.event_id).await?;
            return Ok((StatusCode::OK, Json(json!(GetEventDetailedResponse {
                    event: event, 
                    invite_link: Some(invite_link), 
                    members: members,
                    permissions: permissions.get_bits().to_string()
            }))));
        },
        Ok(false) => {
            return Ok((StatusCode::OK, Json(json!(GetEventDetailedResponse {
                    event: event, 
                    invite_link: None, 
                    members: members,
                    permissions: permissions.get_bits().to_string()
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
    Path(path): Path<EventPaths>,
    Json(payload): Json<UpdateEventRequest>,
) -> Result<impl IntoResponse, AppError> {
    let user = get_user_for_handler_from_token(&state.db_pool, auth.token()).await?;

    let event = get_event_by_id(&state.db_pool, path.event_id).await?;

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
        path.event_id, 
        payload.title, 
        payload.description_event, 
        payload.start_date_time.and_then(|s| s.parse::<DateTime<Utc>>().ok()), 
        payload.end_date_time.and_then(|s| s.parse::<DateTime<Utc>>().ok()), 
        payload.color,
        payload.location
    ).await?;

    let event_new = get_event_by_id(&state.db_pool, path.event_id).await?;

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
            id: format!("poll_{}", poll.poll_id),
            title: poll.question,
            data: PollModuleData {
                options: poll_data.options,
                multiple_choice: poll.more_than_one_vote,
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
                id: format!("item_{}", item.item_id),
                text: item.item_text,
                assigned_user_id: item.assigned_user_id.map(|id| id.to_string()),
                assigned_user_name: item.assigned_user_name,
            })
            .collect();
        
        modules.push(PlanningModule::ItemList {
            id: format!("items_{}", item_list.item_list_id),
            title: item_list.title,
            data: ItemListModuleData { items },
        });
    }
    
    // 5. Получаем все списки задач
    let task_lists = get_event_task_lists(&state.db_pool, event_id).await?;
    for task_list in task_lists {
        let tasks: Vec<TaskListItemData> = task_list.tasks
            .into_iter()
            .map(|task| TaskListItemData {
                id: format!("task_{}", task.task_id),
                text: task.task_text,
                assigned_user_id: task.assigned_user_id.map(|id| id.to_string()),
                assigned_user_name: task.assigned_user_name,
                completed: task.is_completed,
            })
            .collect();
        
        modules.push(PlanningModule::TaskList {
            id: format!("tasks_{}", task_list.task_list_id),
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
    let user = get_user_for_handler_from_token(&state.db_pool, &auth.token()).await?;
    
    let event = get_event_by_id(&state.db_pool, payload.event_id).await?;

    let is_member = check_user_in_event(&state.db_pool, event.event_id, user.user_id).await?;
    if !is_member {
        return Err(AppError::UserNotInEvent("User not in event".to_string()));
    }
    let _ = match check_user_permissions(&state.db_pool, &event, &user, EventPermissions::OWNER).await {
        Ok(true) => true,
        Ok(false) => return Err(AppError::UserNotInEvent("User doesn't have permission to update permissions".to_string())),
        Err(e) => return Err(e),
    };


    Ok((StatusCode::NO_CONTENT, Json(json!({"success": true}))))
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
// #[cfg(test)]
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

//     #[tokio::test]
//     async fn test_get_user_events_handler() {
//         let app = create_test_app().await;

//         let state = app.state::<Arc<AppState>>().unwrap();

//         let token = create_test_user_and_token(&state.db_pool).await;

//         let request = Request::builder()
//             .method("GET")
//             .uri("/events/user")
//             .header("authorization", format!("Bearer {}", token))
//             .body(Body::empty())
//             .unwrap();

//         let response = app.oneshot(request).await.unwrap();

//         assert_eq!(response.status(), StatusCode::OK);
//     }
// }
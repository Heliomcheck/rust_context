// src/api_doc.rs
use utoipa::{OpenApi, openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme}};

use crate::{
    structs::*,
    models::*,
    handlers::event::*,
    handlers::user::*,
    handlers::auth::*,
    handlers::modules::poll::*,
    handlers::modules::item::*,
    handlers::modules::task::*,
    test_utils::*,
};

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Rust Context API",
        description = "API for event management, polls, and real-time chat",
        version = "0.3",
        contact(
            name = "Heliom",
            email = "heliom.check@gmail.com"
        )
    ),
    servers(
        (url = "http://localhost:8080", description = "Development server"),
        (url = "https://api.rust-context.com", description = "Production server")
    ),
    tags(
        (name = "Auth", description = "Authentication endpoints"),
        (name = "User", description = "User management"),
        (name = "Event", description = "Event management"),
        (name = "Avatar", description = "Avatar management"),
        (name = "Modules", description = "Modules management"),
        (name = "test_server", description = "Test server endpoints")
        //(name = "Chat", description = "WebSocket chat")
    ),
    paths(
        create_event_handler,
        get_user_events_handler,
        get_detailed_event_handler,
        event_join_handler,

        request_code_handler,
        verify_code_handler,
        resend_code_handler,
        register_handler,
        token_validate_handler,
        logout_handler,
        username_check_handler,

        user_edit_handler,
        get_user_data_handler,
        upload_avatar_handler,
        get_avatar_handler,
        add_user_to_event_handler,
        delete_user_from_event_handler,
        update_user_permissions_handler,

        create_poll_handler,
        update_poll_handler,
        delete_poll_handler,
        vote_poll_handler,

        create_item_list_handler,
        update_item_list_handler,
        assign_item_handler,
        delete_item_list_handler,

        create_task_list_handler,
        update_task_list_handler,
        assign_task_handler,
        complete_task_handler,
        delete_task_list_handler,
        // create_item_handler,
        // update_item_handler,
        // create_task_handler,
        // update_task_handler,

        health_handler,
    ),
    components(
        schemas(
            // Auth
            RegisterRequestWrapper,
            RegisterRequest,
            CodeRequest,
            VerifyCodeRequest,
            RegisterResponse,
            NewUserVerifyResponse,
            // User
            User,
            CheckUsernameRequest,
            EditUserRequest,
            UserDataResponse,
            GetUserDataResponseWrapper,
            // Event
            CreateEventRequest,
            CreateEventResponse,
            GetEventRequest,
            GetEventsResponse,
            GetEventDetailedResponse,
            UpdateUserPermissionsRequest,
            InviteUserToEventRequest,
            JoinEventRequest,
            // Poll
            PollResponse,
            CreatePollRequest,
            UpdatePollRequest,
            DeletePollRequest,
            VotePollRequest,
            // Item_list
            CreateItemListRequest,
            ItemListResponse,
            UpdateItemsListRequest,
            AssignItemRequest,
            DeleteItemListRequest,
            // Task_list
            CreateTaskListRequest,
            UpdateTaskListRequest,
            AssignTaskRequest,
            CompleteTaskRequest,
            CreateTaskListResponse,
            DeleteTaskListResponse,
            // Responses
            SuccessResponse,
            ErrorResponse,
            // PollOption,
            // VoteRequest,
            // // Common
            // ErrorResponse,
            // SuccessResponse,
        )
    ),
    security(
        ("bearer_auth" = [])
    ),
    modifiers(&SecurityAddon)
)]
pub struct ApiDoc;

pub struct SecurityAddon;

impl utoipa::Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer_auth",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(utoipa::openapi::security::HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .description(Some("JWT token obtained after login"))
                        .build(),
                ),
            );
        }
    }
}
// src/api_doc.rs
use utoipa::{OpenApi, openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme}};
use utoipa::OpenApi;
use crate::{
    structs::*,
    models::*,
    handlers::event::*,
    handlers::user::*,
    handlers::auth::*,
    handlers::poll::*, 
    handlers::item::*,
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
        (name = "Poll", description = "Poll management"),
        //(name = "Chat", description = "WebSocket chat")
    ),
        paths(
        // Auth
        request_code_handler,
        verify_code_handler,
        resend_code_handler,
        register_handler,
        token_validate_handler,
        logout_handler,
        username_check_handler,
        // User
        user_edit_handler,
        get_user_data_handler,
        upload_avatar_handler,
        get_avatar_handler,
        // Events
        create_event_handler,
        list_events_handler,
        get_event_handler,
        update_event_handler,
        delete_event_handler,
        join_event_handler,
        add_member_handler,
        remove_member_handler,
        update_member_permissions_handler,
        // Polls
        create_poll_handler,
        list_polls_handler,
        get_poll_handler,
        update_poll_handler,
        delete_poll_handler,
        vote_handler,
        // Items
        create_item_handler,
        list_items_handler,
        update_item_handler,
        delete_item_handler,
    ),
    components(
        schemas(
            // Auth
            RegisterRequest,
            CodeRequest,
            VerifyCodeRequest,
            // User
            User,
            CheckUsernameRequest,
            EditUserRequest,
            UserDataResponse,
            // Event
            CreateEventRequest,
            CreateEventResponse,
            GetEventRequest,
            GetEventResponse,
            UpdateUserPermissionsRequest,
            InviteUserToEventRequest,
            // Poll
            CreatePollRequest,
            UpdatePollRequest,
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
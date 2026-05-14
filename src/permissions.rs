use serde::Serialize;
use sqlx::PgPool;
use crate::{
    errors::AppError, 
    models::*, 
    structs::*
};

#[derive(Debug, Clone, Serialize)]
pub struct EventPermissions {
    bits: i32
}

impl EventPermissions {
    pub fn new(bits: i32) -> Self {
        Self { bits }
    }

    pub fn empty() -> Self {
        Self { bits: 0 }
    }

    pub fn full() -> Self {
        Self { bits: Self::ALL }
    }

    pub const INVITE: i32 = 1 << 0;
    pub const UPDATE_PERMISSIONS: i32 = 1 << 1;
    pub const DELETE_MEMBER: i32 = 1 << 2;
    pub const REMOVE_MEMBER: i32 = 1 << 1;
    pub const EDIT_EVENT: i32 = 1 << 2;
    pub const BAN_MEMBER: i32 = 1 << 3;
    pub const CREATE_MODULE: i32 = 1 << 4;
    pub const UPDATE_MODULE: i32 = 1 << 4;
    pub const DELETE_MODULE: i32 = 1 << 5;
    pub const VIEW_STATS: i32 = 1 << 6;
    pub const MANAGE_ROLES: i32 = 1 << 7;
    pub const VIEW_LOGS: i32 = 1 << 8;
    pub const ADMIN: i32 = 1 << 30;
    pub const OWNER: i32 = 1 << 31;

    pub const MEMBER: i32 = Self::INVITE | Self::VIEW_STATS;
    pub const MODERATOR: i32 = Self::INVITE | Self::REMOVE_MEMBER | Self::BAN_MEMBER | Self::VIEW_STATS;
    pub const ADMIN_FULL: i32 = Self::ADMIN | Self::MODERATOR | Self::EDIT_EVENT | Self::MANAGE_ROLES | Self::VIEW_LOGS;
    pub const ALL: i32 = Self::OWNER | Self::ADMIN_FULL | Self::CREATE_MODULE | Self::DELETE_MODULE;

    pub fn check_permission(&self, permission: i32) -> bool {
        (self.bits & permission) != 0
    }

    pub fn add_permission(&mut self, permission: i32) {
        self.bits |= permission;
    }
    
    pub fn remove_permission(&mut self, permission: i32) {
        self.bits &= !permission;
    }
}

pub async fn check_user_permissions(
    pool: &PgPool, 
    event: &Events,
    user: &User,
    permission: i32
) -> Result<bool, AppError> {
    let result = sqlx::query!(
        r#"
        SELECT eu.permissions
        FROM event_user eu
        WHERE event_id = $1 and user_id = $2
        "#,
        event.event_id,
        user.user_id
    )
    .fetch_one(pool)
    .await?;

    let permissions = EventPermissions::new(result.permissions);
     
    Ok(permissions.check_permission(permission))
}

pub async fn get_user_permissions(
    pool: &PgPool, 
    event_id: i64,
    user_id: i64
) -> Result<EventPermissions, AppError> {
    let result = sqlx::query!(
        r#"
        SELECT permissions
        FROM event_user
        WHERE event_id = $1 and user_id = $2
        "#,
        event_id,
        user_id
    )
    .fetch_one(pool)
    .await?;

    Ok(EventPermissions::new(result.permissions))
}

pub async fn update_user_permissions(
    pool: &PgPool,
    event_id: i64,
    user_id: i64,
    new_permissions: i32
) -> Result<(), AppError> {
    sqlx::query!(
        r#"
        UPDATE event_user
        SET permissions = $1
        WHERE event_id = $2 AND user_id = $3
        "#,
        new_permissions,
        event_id,
        user_id
    )
    .execute(pool)
    .await?;

    Ok(())
}
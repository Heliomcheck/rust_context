use serde::Serialize;
use sqlx::PgPool;
use utoipa::{ToSchema};
use crate::{
    errors::AppError,
    structs::*
};
use crate::data_base::event_db::has_permission;

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct EventPermissions {
    bits: i32
}

impl EventPermissions {
    pub fn new() -> Self {
        Self { bits: 0b000 }
    }
    pub fn new_value(bits: i32) -> Self {
        Self { bits }
    }

    #[allow(dead_code)]
    pub fn empty() -> Self {
        Self { bits: 0 }
    }

    #[allow(dead_code)]
    pub fn full() -> Self {
        Self { bits: Self::OWNER }
    }

    pub fn get_bits(&self) -> i32 {
        self.bits
    }

    // Существующие константы
    pub const OWNER: i32 = 1 << 2;
    #[allow(dead_code)]
    pub const ADMIN: i32 = 1 << 1;
    pub const MEMBER: i32 = 1 << 0;

    // Новые константы для альбомов
    pub const CREATE_ALBUM: i32 = 1 << 3;   // 8
    pub const DELETE_ALBUM: i32 = 1 << 4;   // 16
    pub const UPLOAD_PHOTO: i32 = 1 << 5;   // 32
    pub const DELETE_PHOTO: i32 = 1 << 6;   // 64

    pub fn check_permission(&self, permission: i32) -> bool {
        (self.bits & permission) != 0
    }

    pub fn add_permission(&mut self, permission: i32) -> &mut Self {
        self.bits |= permission;
        self
    }

    #[allow(dead_code)]
    pub fn remove_permission(&mut self, permission: i32) {
        self.bits &= !permission;
    }

    pub fn to_binary_string_short(&self) -> String {
        format!("{:03b}", self.bits)
    }
}

pub async fn check_user_permissions(
    pool: &PgPool, 
    engrok config add-authtokenvent: &Events,
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

    let permissions = EventPermissions::new_value(result.permissions);
     
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

    Ok(EventPermissions::new_value(result.permissions))
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

/// Проверяет, имеет ли пользователь хотя бы одно из указанных прав в событии.
pub async fn has_any_permission(
    pool: &PgPool,
    event_id: i64,
    user_id: i64,
    permissions: &[i32],
) -> Result<bool, AppError> {
    for &perm in permissions {
        if has_permission(pool, event_id, user_id, perm).await? {
            return Ok(true);
        }
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_base::{event_db, user_db};
    use crate::test_utils::setup_test_db;

    #[test]
    fn test_event_permissions_basic_operations() {
        let mut permissions: EventPermissions = EventPermissions::full();
        assert!(permissions.check_permission(EventPermissions::OWNER));
        assert!(!permissions.check_permission(EventPermissions::ADMIN));

        permissions.add_permission(EventPermissions::ADMIN);
        assert!(permissions.check_permission(EventPermissions::ADMIN));
    }

    #[tokio::test]
    async fn test_check_and_update_user_permissions_with_db() -> anyhow::Result<()> {
        let pool = setup_test_db().await;

        let user_id = user_db::create_user_db(
            &pool,
            "perms_user",
            "perms@mail.com",
            "Perms User",
            &None,
            &None,
        ).await?;

        let event_id = event_db::create_event(
            &pool,
            "Perms Event",
            None,
            None,
            None,
            None,
            "#abcdef".to_string(),
        ).await?;

        event_db::add_member(&pool, user_id, event_id, EventPermissions::OWNER).await?;

        let event = event_db::get_event_by_id(&pool, event_id).await?;
        let user = user_db::find_user_by_id(&pool, user_id).await?.unwrap();

        let has_owner = check_user_permissions(&pool, &event, &user, EventPermissions::OWNER).await?;
        assert!(has_owner);

        update_user_permissions(&pool, event_id, user_id, EventPermissions::ADMIN).await?;
        let updated_permissions = get_user_permissions(&pool, event_id, user_id).await?;
        assert!(updated_permissions.check_permission(EventPermissions::ADMIN));

        Ok(())
    }
}
use serde::Serialize;
use sqlx::PgPool;
use crate::{
    errors::AppError,
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

    // pub const INVITE: i32 = 1 << 0;
    // pub const UPDATE_PERMISSIONS: i32 = 1 << 1;
    // pub const DELETE_MEMBER: i32 = 1 << 2;
    // pub const REMOVE_MEMBER: i32 = 1 << 1;
    // pub const EDIT_EVENT: i32 = 1 << 2;
    // pub const BAN_MEMBER: i32 = 1 << 3;
    // pub const CREATE_MODULE: i32 = 1 << 4;
    // pub const UPDATE_MODULE: i32 = 1 << 4;
    // pub const DELETE_MODULE: i32 = 1 << 5;
    // pub const VIEW_STATS: i32 = 1 << 6;
    // pub const MANAGE_ROLES: i32 = 1 << 7;
    // pub const VIEW_LOGS: i32 = 1 << 8;
    // pub const ADMIN: i32 = 1 << 30;
    // pub const OWNER: i32 = 1 << 31;

    pub const OWNER:i32 = 1 << 0;
    #[allow(dead_code)]
    pub const ADMIN:i32 = 1 << 1;
    pub const MEMBER:i32 = 1 << 2;


    pub fn check_permission(&self, permission: i32) -> bool {
        (self.bits & permission) != 0
    }

    #[allow(dead_code)]
    pub fn add_permission(&mut self, permission: i32) {
        self.bits |= permission;
    }

    #[allow(dead_code)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_base::{event_db, user_db};
    use crate::test_utils::setup_test_db;


    #[test]
    fn test_event_permissions_basic_operations() {
        let mut permissions = EventPermissions::full();
        assert!(permissions.check_permission(EventPermissions::OWNER));
        assert!(!permissions.check_permission(EventPermissions::ADMIN));
        
        // Добавляем ADMIN
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
        )
        .await?;

        let event_id = event_db::create_event(
            &pool,
            "Perms Event",
            None,
            None,
            None,
            "#abcdef".to_string(),
        )
        .await?;

        event_db::add_member(&pool, user_id, event_id, EventPermissions::OWNER).await?;

        let event = event_db::get_event_by_id(&pool, event_id).await?;
        let user = user_db::find_user_by_id(&pool, user_id).await?.unwrap();

        let has_owner = check_user_permissions(&pool, &event, &user, EventPermissions::OWNER)
            .await?;
        assert!(has_owner);

        update_user_permissions(&pool, event_id, user_id, EventPermissions::ADMIN).await?;
        let updated_permissions = get_user_permissions(&pool, event_id, user_id).await?;
        assert!(updated_permissions.check_permission(EventPermissions::ADMIN));

        Ok(())
    }
}

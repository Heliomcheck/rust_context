use multipart::server::nickel::nickel::status::StatusCode::BadRequest;
use sqlx::PgPool;
use crate::{
    errors::AppError, 
    models::*, 
    structs::*
};

pub async fn check_user_permissions(
    pool: &PgPool, 
    event: &Events,
    user: &User,
    index: i64
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

    let us_index = match usize::try_from(index) {
        Ok(us) => us,
        Err(_) => {
            return Err(AppError::BadRequest("Bad transform index to usize".to_string())); 
        }
    };
    
    if result.permissions.chars().nth(us_index) == Some('1') {
        return Ok(true)
    } else {
        return Ok(false)
    }
}
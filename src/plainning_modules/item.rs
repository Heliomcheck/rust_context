use sqlx::PgPool;
use crate::structs::Item;

pub async fn create_item(
    pool: &PgPool,
    event_id: i64,
    content: &str,
    user_id: i64,
) -> Result<Item, sqlx::Error> {
    let row = sqlx::query_as!(
        Item,
        r#"
        INSERT INTO items (event_id, content, created_by)
        VALUES ($1, $2, $3)
        RETURNING item_id, event_id, content, done, created_by, created_at
        "#,
        event_id,
        content,
        user_id
    )
    .fetch_one(pool)
    .await?;

    Ok(row)
}

pub async fn get_items(
    pool: &PgPool,
    event_id: i64,
) -> Result<Vec<Item>, sqlx::Error> {
    let items = sqlx::query_as!(
        Item,
        r#"
        SELECT item_id, event_id, content, done, created_by, created_at
        FROM items
        WHERE event_id = $1
        ORDER BY created_at ASC
        "#,
        event_id
    )
    .fetch_all(pool)
    .await?;

    Ok(items)
}

pub async fn update_item(
    pool: &PgPool,
    item_id: i64,
    content: Option<&str>,
    done: Option<bool>,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query!(
        r#"
        UPDATE items
        SET content = COALESCE($1, content),
            done = COALESCE($2, done)
        WHERE item_id = $3
        "#,
        content,
        done,
        item_id
    )
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

pub async fn delete_item(
    pool: &PgPool,
    item_id: i64,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query!(
        r#"
        DELETE FROM items
        WHERE item_id = $1
        "#,
        item_id
    )
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}
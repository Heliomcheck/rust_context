use sqlx::PgPool;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use crate::errors::AppError;
use std::collections::HashMap;
use crate::{
    //errors::AppError,
};

// ============== СТРУКТУРЫ ДЛЯ ОТВЕТОВ ==============

#[derive(Debug, Serialize, Deserialize)]
pub struct ItemListWithItems {
    pub item_list_id: i64,
    pub event_id: i64,
    pub title: String,
    pub created_by: i64,
    pub created_at: DateTime<Utc>,
    pub items: Vec<ItemListItem>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ItemListItem {
    pub item_id: i64,
    pub item_text: String,
    pub assigned_user_id: Option<i64>,
    pub assigned_user_name: Option<String>,
}

// ============== ОСНОВНЫЕ ФУНКЦИИ ==============

/// Создание item_list с пунктами
pub async fn create_item_list(
    pool: &PgPool,
    event_id: i64,
    title: &str,
    items: &[String],
    created_by: i64,
) -> Result<ItemListWithItems, AppError> {
    let mut tx = pool.begin().await?;

    let row = sqlx::query!(
        r#"
        INSERT INTO item_list (event_id, title, created_by)
        VALUES ($1, $2, $3)
        RETURNING item_list_id, created_at
        "#,
        event_id,
        title,
        created_by
    )
    .fetch_one(&mut *tx)
    .await?;

    let item_list_id = row.item_list_id;
    let created_at = row.created_at.unwrap_or_else(Utc::now);

    let mut saved_items = Vec::new();
    for item_text in items {
        let item_row = sqlx::query!(
            r#"
            INSERT INTO item_list_item (item_list_id, item_text)
            VALUES ($1, $2)
            RETURNING item_id
            "#,
            item_list_id,
            item_text
        )
        .fetch_one(&mut *tx)
        .await?;

        saved_items.push(ItemListItem {
            item_id: item_row.item_id,
            item_text: item_text.clone(),
            assigned_user_id: None,
            assigned_user_name: None,
        });
    }

    tx.commit().await?;

    Ok(ItemListWithItems {
        item_list_id,
        event_id,
        title: title.to_string(),
        created_by,
        created_at,
        items: saved_items,
    })
}

/// Получение item_list со всеми пунктами
pub async fn get_item_list(
    pool: &PgPool,
    item_list_id: i64,
) -> Result<Option<ItemListWithItems>, AppError> {
    let row = sqlx::query!(
        r#"
        SELECT 
            il.event_id,
            il.title,
            il.created_by,
            il.created_at,
            COALESCE(
                json_agg(
                    json_build_object(
                        'item_id', ili.item_id,
                        'item_text', ili.item_text,
                        'assigned_user_id', ili.assigned_user_id,
                        'assigned_user_name',
                            CASE 
                                WHEN ili.assigned_user_id IS NOT NULL THEN 
                                    COALESCE(u.display_name, u.username)
                                ELSE NULL
                            END
                    ) ORDER BY ili.item_id
                ) FILTER (WHERE ili.item_id IS NOT NULL),
                '[]'::json
            ) as items
        FROM item_list il
        LEFT JOIN item_list_item ili ON il.item_list_id = ili.item_list_id
        LEFT JOIN users u ON ili.assigned_user_id = u.user_id
        WHERE il.item_list_id = $1 AND il.is_active = true
        GROUP BY il.item_list_id
        "#,
        item_list_id
    )
    .fetch_optional(pool)
    .await?;

    let row = match row {
        Some(r) => r,
        None => return Ok(None),
    };
    let items: Vec<ItemListItem> = match row.items {
        Some(items_json) => serde_json::from_value(items_json).unwrap_or_default(),
        None => vec![],
    };
    let created_at = row.created_at.unwrap_or_else(Utc::now);

    Ok(Some(ItemListWithItems {
        item_list_id,
        event_id: row.event_id,
        title: row.title,
        created_by: row.created_by,
        created_at: created_at,
        items,
    }))
}

/// Обновление item_list (добавление/удаление пунктов)
pub async fn update_item_list(
    pool: &PgPool,
    item_list_id: i64,
    add_items: &[String],
    remove_item_ids: &[i64],
) -> Result<ItemListWithItems, AppError> {
    let mut tx = pool.begin().await?;

    // Добавляем новые пункты
    for item_text in add_items {
        sqlx::query!(
            r#"
            INSERT INTO item_list_item (item_list_id, item_text)
            VALUES ($1, $2)
            "#,
            item_list_id,
            item_text
        )
        .execute(&mut *tx)
        .await?;
    }

    // Удаляем пункты (только если не забронированы)
    if !remove_item_ids.is_empty() {
        let rows = sqlx::query!(
            r#"
            DELETE FROM item_list_item
            WHERE item_id = ANY($1)
                AND item_list_id = $2
                AND assigned_user_id IS NULL
            RETURNING item_id
            "#,
            remove_item_ids,
            item_list_id
        )
        .fetch_all(&mut *tx)
        .await?;

        if rows.len() != remove_item_ids.len() {
            return Err(AppError::BadRequest("Some items are assigned and cannot be deleted".to_string()));
        }
    }

    tx.commit().await?;

    // Возвращаем обновленный список
    get_item_list(pool, item_list_id)
        .await?
        .ok_or(AppError::NotFound("Not found".to_string()))
}

/// Бронирование/отмена бронирования пункта
pub async fn assign_item(
    pool: &PgPool,
    item_id: i64,
    user_id: i64,
    assign: bool,
) -> Result<(), AppError> {
    if assign {
        let result = sqlx::query!(
            r#"
            UPDATE item_list_item
            SET assigned_user_id = $1
            WHERE item_id = $2 AND assigned_user_id IS NULL
            RETURNING item_id
            "#,
            user_id,
            item_id
        )
        .fetch_optional(pool)
        .await?;

        if result.is_none() {
            return Err(AppError::BadRequest("Item is already assigned".to_string()));
        }
    } else {
        let result = sqlx::query!(
            r#"
            UPDATE item_list_item
            SET assigned_user_id = NULL
            WHERE item_id = $1 AND assigned_user_id = $2
            RETURNING item_id
            "#,
            item_id,
            user_id
        )
        .fetch_optional(pool)
        .await?;

        if result.is_none() {
            return Err(AppError::BadRequest("Item is not assigned to you".to_string()));
        }
    }

    Ok(())
}

/// Удаление item_list
pub async fn delete_item_list(
    pool: &PgPool,
    item_list_id: i64,
    event_id: i64,
) -> Result<(), AppError> {
    let result = sqlx::query!(
        r#"
        DELETE FROM item_list
        WHERE item_list_id = $1 AND event_id = $2
        RETURNING item_list_id
        "#,
        item_list_id,
        event_id
    )
    .fetch_optional(pool)
    .await?;

    if result.is_none() {
        return Err(AppError::NotFound("Not found".to_string()));
    }

    Ok(())
}

/// Проверка принадлежности item_list к событию
pub async fn verify_item_list_in_event(
    pool: &PgPool,
    item_list_id: i64,
    event_id: i64,
) -> Result<bool, AppError> {
    let row = sqlx::query!(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM item_list
            WHERE item_list_id = $1 AND event_id = $2 AND is_active = true
        ) as "exists!"
        "#,
        item_list_id,
        event_id
    )
    .fetch_one(pool)
    .await?;

    Ok(row.exists)
}
use sqlx::PgPool;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use crate::errors::AppError;

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

pub async fn get_event_item_lists(
    pool: &PgPool,
    event_id: i64,
) -> Result<Vec<ItemListWithItems>, AppError> {
    let rows = sqlx::query!(
        r#"
        SELECT 
            il.item_list_id,
            il.title,
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
        WHERE il.event_id = $1 AND il.is_active = true
        GROUP BY il.item_list_id
        ORDER BY il.created_at DESC
        "#,
        event_id
    )
    .fetch_all(pool)
    .await?;
    
    let mut result = Vec::new();
    for row in rows {
        let items: Vec<ItemListItem> = match row.items {
            Some(items_json) => serde_json::from_value(items_json).unwrap_or_default(),
            None => vec![],
        };
        
        result.push(ItemListWithItems {
            item_list_id: row.item_list_id,
            event_id,
            title: row.title,
            created_by: 0, // не нужно для ответа
            created_at: chrono::Utc::now(), // не нужно для ответа
            items,
        });
    }
    
    Ok(result)
}

//test
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;
    use crate::data_base::user_db::create_user_db;
    use crate::data_base::event_db::{create_event, add_member};
    use crate::permissions::EventPermissions;

    #[tokio::test]
    async fn test_create_and_get_item_list() -> anyhow::Result<()> {
        let _guard = lock_db().await;
        let pool = setup_test_db().await;
        let user_id = create_user_db(&pool, "itemdbuser", "itemdb@mail.com", "User", &None, &None).await?;
        let event_id = create_event(&pool, "Event", None, None, None, None, "#123".to_string()).await?;
        add_member(&pool, user_id, event_id, EventPermissions::OWNER).await?;

        let list = create_item_list(&pool, event_id, "List", &["item1".to_string(), "item2".to_string()], user_id).await?;
        assert_eq!(list.items.len(), 2);

        let fetched = get_item_list(&pool, list.item_list_id).await?.expect("should exist");
        assert_eq!(fetched.title, "List");
        Ok(())
    }

    #[tokio::test]
    async fn test_assign_and_unassign_item() -> anyhow::Result<()> {
        let _guard = lock_db().await;
        let pool = setup_test_db().await;
        let user_id = create_user_db(&pool, "assigner", "assigner@mail.com", "User", &None, &None).await?;
        let event_id = create_event(&pool, "Event", None, None, None, None, "#123".to_string()).await?;
        add_member(&pool, user_id, event_id, EventPermissions::OWNER).await?;

        let list = create_item_list(&pool, event_id, "List", &["item".to_string()], user_id).await?;
        let item_id = list.items[0].item_id;

        assign_item(&pool, item_id, user_id, true).await?;
        let list = get_item_list(&pool, list.item_list_id).await?.unwrap();
        assert_eq!(list.items[0].assigned_user_id, Some(user_id));

        assign_item(&pool, item_id, user_id, false).await?;
        let list = get_item_list(&pool, list.item_list_id).await?.unwrap();
        assert_eq!(list.items[0].assigned_user_id, None);
        Ok(())
    }

    #[tokio::test]
    async fn test_assign_item_already_assigned() -> anyhow::Result<()> {
        let _guard = lock_db().await;
        let pool = setup_test_db().await;
        let user1 = create_user_db(&pool, "user1", "user1@mail.com", "U1", &None, &None).await?;
        let user2 = create_user_db(&pool, "user2", "user2@mail.com", "U2", &None, &None).await?;
        let event_id = create_event(&pool, "Event", None, None, None, None, "#123".to_string()).await?;
        add_member(&pool, user1, event_id, EventPermissions::OWNER).await?;
        add_member(&pool, user2, event_id, EventPermissions::MEMBER).await?;

        let list = create_item_list(&pool, event_id, "List", &["item".to_string()], user1).await?;
        let item_id = list.items[0].item_id;

        assign_item(&pool, item_id, user1, true).await?;
        let result = assign_item(&pool, item_id, user2, true).await;
        assert!(result.is_err()); // уже назначено
        Ok(())
    }

    #[tokio::test]
    async fn test_delete_item_list() -> anyhow::Result<()> {
        let _guard = lock_db().await;
        let pool = setup_test_db().await;
        let user_id = create_user_db(&pool, "deleter", "deleter@mail.com", "User", &None, &None).await?;
        let event_id = create_event(&pool, "Event", None, None, None, None, "#123".to_string()).await?;
        add_member(&pool, user_id, event_id, EventPermissions::OWNER).await?;

        let list = create_item_list(&pool, event_id, "List", &["item".to_string()], user_id).await?;
        delete_item_list(&pool, list.item_list_id, event_id).await?;
        let result = get_item_list(&pool, list.item_list_id).await?;
        assert!(result.is_none());
        Ok(())
    }

    #[tokio::test]
    async fn test_update_item_list_add_and_remove() -> anyhow::Result<()> {
        let _guard = lock_db().await;
        let pool = setup_test_db().await;
        let user_id = create_user_db(&pool, "updater", "updater@mail.com", "User", &None, &None).await?;
        let event_id = create_event(&pool, "Event", None, None, None, None, "#123".to_string()).await?;
        add_member(&pool, user_id, event_id, EventPermissions::OWNER).await?;

        let list = create_item_list(&pool, event_id, "List", &["item1".to_string()], user_id).await?;
        let item_to_remove = list.items[0].item_id;

        let updated = update_item_list(
            &pool,
            list.item_list_id,
            &["item2".to_string()],
            &[item_to_remove],
        ).await?;
        assert_eq!(updated.items.len(), 1);
        assert_eq!(updated.items[0].item_text, "item2");
        Ok(())
    }
}
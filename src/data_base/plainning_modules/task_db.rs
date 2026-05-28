use sqlx::PgPool;
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use crate::errors::AppError;

// ============== СТРУКТУРЫ ДЛЯ ОТВЕТОВ ==============

#[derive(Debug, Serialize, Deserialize)]
pub struct TaskListWithItems {
    pub task_list_id: i64,
    pub event_id: i64,
    pub title: String,
    pub created_by: i64,
    pub created_at: DateTime<Utc>,
    pub items: Vec<TaskListItem>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TaskListItem {
    pub task_id: i64,
    pub task_text: String,
    pub assigned_user_id: Option<i64>,
    pub assigned_user_name: Option<String>,
    pub is_completed: bool,
}

// ============== ОСНОВНЫЕ ФУНКЦИИ ==============

/// Создание task_list с задачами
pub async fn create_task_list(
    pool: &PgPool,
    event_id: i64,
    title: &str,
    tasks: &[String],
    created_by: i64,
) -> Result<TaskListWithItems, AppError> {
    let mut tx = pool.begin().await?;

    let row = sqlx::query!(
        r#"
        INSERT INTO task_list (event_id, title, created_by)
        VALUES ($1, $2, $3)
        RETURNING task_list_id, created_at
        "#,
        event_id,
        title,
        created_by
    )
    .fetch_one(&mut *tx)
    .await?;

    let task_list_id = row.task_list_id;
    let created_at = row.created_at.unwrap();

    let mut saved_tasks = Vec::new();
    for task_text in tasks {
        let task_row = sqlx::query!(
            r#"
            INSERT INTO task_list_item (task_list_id, task_text)
            VALUES ($1, $2)
            RETURNING task_id
            "#,
            task_list_id,
            task_text
        )
        .fetch_one(&mut *tx)
        .await?;

        saved_tasks.push(TaskListItem {
            task_id: task_row.task_id,
            task_text: task_text.clone(),
            assigned_user_id: None,
            assigned_user_name: None,
            is_completed: false,
        });
    }

    tx.commit().await?;

    Ok(TaskListWithItems {
        task_list_id,
        event_id,
        title: title.to_string(),
        created_by,
        created_at,
        items: saved_tasks,
    })
}

/// Получение task_list со всеми задачами
pub async fn get_task_list(
    pool: &PgPool,
    task_list_id: i64,
) -> Result<Option<TaskListWithItems>, AppError> {
    let row = sqlx::query!(
        r#"
        SELECT 
            tl.event_id,
            tl.title,
            tl.created_by,
            tl.created_at,
            COALESCE(
                json_agg(
                    json_build_object(
                        'task_id', tli.task_id,
                        'task_text', tli.task_text,
                        'assigned_user_id', tli.assigned_user_id,
                        'assigned_user_name',
                            CASE 
                                WHEN tli.assigned_user_id IS NOT NULL THEN 
                                    COALESCE(u.display_name, u.username)
                                ELSE NULL
                            END,
                        'is_completed', tli.is_completed
                    ) ORDER BY tli.task_id
                ) FILTER (WHERE tli.task_id IS NOT NULL),
                '[]'::json
            ) as tasks
        FROM task_list tl
        LEFT JOIN task_list_item tli ON tl.task_list_id = tli.task_list_id
        LEFT JOIN users u ON tli.assigned_user_id = u.user_id
        WHERE tl.task_list_id = $1 AND tl.is_active = true
        GROUP BY tl.task_list_id
        "#,
        task_list_id
    )
    .fetch_optional(pool)
    .await?;

    let row = match row {
        Some(r) => r,
        None => return Ok(None),
    };

    let created_at = row.created_at.unwrap();
    let tasks: Vec<TaskListItem> = match row.tasks {
        Some(tasks_json) => serde_json::from_value(tasks_json).unwrap_or_default(),
        None => vec![],
    };

    Ok(Some(TaskListWithItems {
        task_list_id,
        event_id: row.event_id,
        title: row.title,
        created_by: row.created_by,
        created_at,
        items: tasks,
    }))
}

/// Обновление task_list (добавление/удаление задач)
pub async fn update_task_list(
    pool: &PgPool,
    task_list_id: i64,
    add_tasks: &[String],
    remove_task_ids: &[i64],
) -> Result<TaskListWithItems, AppError> {
    let mut tx = pool.begin().await?;

    // Добавляем новые задачи
    for task_text in add_tasks {
        sqlx::query!(
            r#"
            INSERT INTO task_list_item (task_list_id, task_text)
            VALUES ($1, $2)
            "#,
            task_list_id,
            task_text
        )
        .execute(&mut *tx)
        .await?;
    }

    // Удаляем задачи (только если не забронированы)
    if !remove_task_ids.is_empty() {
        let rows = sqlx::query!(
            r#"
            DELETE FROM task_list_item
            WHERE task_id = ANY($1)
                AND task_list_id = $2
                AND assigned_user_id IS NULL
            RETURNING task_id
            "#,
            remove_task_ids,
            task_list_id
        )
        .fetch_all(&mut *tx)
        .await?;

        if rows.len() != remove_task_ids.len() {
            return Err(AppError::BadRequest("Some tasks are assigned and cannot be deleted".to_string()));
        }
    }

    tx.commit().await?;

    get_task_list(pool, task_list_id)
        .await?
        .ok_or(AppError::NotFound("Task list not found".to_string()))
}

/// Бронирование/отмена бронирования задачи
pub async fn assign_task(
    pool: &PgPool,
    task_id: i64,
    user_id: i64,
    assign: bool,
) -> Result<(), AppError> {
    if assign {
        let result = sqlx::query!(
            r#"
            UPDATE task_list_item
            SET assigned_user_id = $1, is_completed = false
            WHERE task_id = $2 AND assigned_user_id IS NULL
            RETURNING task_id
            "#,
            user_id,
            task_id
        )
        .fetch_optional(pool)
        .await?;

        if result.is_none() {
            return Err(AppError::BadRequest("Task is already assigned".to_string()));
        }
    } else {
        let result = sqlx::query!(
            r#"
            UPDATE task_list_item
            SET assigned_user_id = NULL, is_completed = false
            WHERE task_id = $1 AND assigned_user_id = $2
            RETURNING task_id
            "#,
            task_id,
            user_id
        )
        .fetch_optional(pool)
        .await?;

        if result.is_none() {
            return Err(AppError::BadRequest("Task is not assigned to you".to_string()));
        }
    }

    Ok(())
}

/// Отметка о выполнении задачи
pub async fn complete_task(
    pool: &PgPool,
    task_id: i64,
    user_id: i64,
    completed: bool,
) -> Result<(), AppError> {
    // Проверяем, что задача забронирована текущим пользователем
    let row = sqlx::query!(
        r#"
        UPDATE task_list_item
        SET is_completed = $1
        WHERE task_id = $2 AND assigned_user_id = $3
        RETURNING task_id
        "#,
        completed,
        task_id,
        user_id
    )
    .fetch_optional(pool)
    .await?;

    if row.is_none() {
        return Err(AppError::BadRequest("Task is not assigned to you".to_string()));
    }

    Ok(())
}

/// Удаление task_list
pub async fn delete_task_list(
    pool: &PgPool,
    task_list_id: i64,
    event_id: i64,
) -> Result<(), AppError> {
    let result = sqlx::query!(
        r#"
        DELETE FROM task_list
        WHERE task_list_id = $1 AND event_id = $2
        RETURNING task_list_id
        "#,
        task_list_id,
        event_id
    )
    .fetch_optional(pool)
    .await?;

    if result.is_none() {
        return Err(AppError::NotFound("Task list not found".to_string()));
    }

    Ok(())
}

/// Проверка принадлежности task_list к событию
pub async fn verify_task_list_in_event(
    pool: &PgPool,
    task_list_id: i64,
    event_id: i64,
) -> Result<bool, AppError> {
    let row = sqlx::query!(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM task_list
            WHERE task_list_id = $1 AND event_id = $2 AND is_active = true
        ) as "exists!"
        "#,
        task_list_id,
        event_id
    )
    .fetch_one(pool)
    .await?;

    Ok(row.exists)
}

pub async fn get_event_task_lists(
    pool: &PgPool,
    event_id: i64,
) -> Result<Vec<TaskListWithItems>, AppError> {
    let rows = sqlx::query!(
        r#"
        SELECT 
            tl.task_list_id,
            tl.title,
            COALESCE(
                json_agg(
                    json_build_object(
                        'task_id', tli.task_id,
                        'task_text', tli.task_text,
                        'assigned_user_id', tli.assigned_user_id,
                        'assigned_user_name',
                            CASE 
                                WHEN tli.assigned_user_id IS NOT NULL THEN 
                                    COALESCE(u.display_name, u.username)
                                ELSE NULL
                            END,
                        'is_completed', tli.is_completed
                    ) ORDER BY tli.task_id
                ) FILTER (WHERE tli.task_id IS NOT NULL),
                '[]'::json
            ) as tasks
        FROM task_list tl
        LEFT JOIN task_list_item tli ON tl.task_list_id = tli.task_list_id
        LEFT JOIN users u ON tli.assigned_user_id = u.user_id
        WHERE tl.event_id = $1 AND tl.is_active = true
        GROUP BY tl.task_list_id
        ORDER BY tl.created_at DESC
        "#,
        event_id
    )
    .fetch_all(pool)
    .await?;
    
    let mut result = Vec::new();
    for row in rows {
        let tasks: Vec<TaskListItem> = match row.tasks {
            Some(tasks_json) => serde_json::from_value(tasks_json).unwrap_or_default(),
            None => vec![],
        };
        
        result.push(TaskListWithItems {
            task_list_id: row.task_list_id,
            event_id,
            title: row.title,
            created_by: 0,
            created_at: chrono::Utc::now(),
            items: tasks,
        });
    }
    
    Ok(result)
}

//test
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::setup_test_db;
    use crate::data_base::user_db::create_user_db;
    use crate::data_base::event_db::{create_event, add_member};
    use crate::permissions::EventPermissions;

    #[tokio::test]
    async fn test_create_and_get_task_list() -> anyhow::Result<()> {
        let pool = setup_test_db().await;
        let user_id = create_user_db(&pool, "taskdbtest", "taskdb@mail.com", "User", &None, &None).await?;
        let event_id = create_event(&pool, "Event", None, None, None, None, "#123".to_string()).await?;
        add_member(&pool, user_id, event_id, EventPermissions::OWNER).await?;

        let list = create_task_list(&pool, event_id, "Tasks", &["task1".to_string(), "task2".to_string()], user_id).await?;
        assert_eq!(list.items.len(), 2);

        let fetched = get_task_list(&pool, list.task_list_id).await?.expect("should exist");
        assert_eq!(fetched.title, "Tasks");
        Ok(())
    }

    #[tokio::test]
    async fn test_assign_and_complete_task() -> anyhow::Result<()> {
        let pool = setup_test_db().await;
        let user_id = create_user_db(&pool, "assigncomp", "assigncomp@mail.com", "User", &None, &None).await?;
        let event_id = create_event(&pool, "Event", None, None, None, None, "#123".to_string()).await?;
        add_member(&pool, user_id, event_id, EventPermissions::OWNER).await?;

        let list = create_task_list(&pool, event_id, "Tasks", &["task1".to_string()], user_id).await?;
        let task_id = list.items[0].task_id;

        assign_task(&pool, task_id, user_id, true).await?;
        let list = get_task_list(&pool, list.task_list_id).await?.unwrap();
        assert_eq!(list.items[0].assigned_user_id, Some(user_id));

        complete_task(&pool, task_id, user_id, true).await?;
        let list = get_task_list(&pool, list.task_list_id).await?.unwrap();
        assert!(list.items[0].is_completed);

        // unassign
        assign_task(&pool, task_id, user_id, false).await?;
        let list = get_task_list(&pool, list.task_list_id).await?.unwrap();
        assert_eq!(list.items[0].assigned_user_id, None);
        Ok(())
    }

    #[tokio::test]
    async fn test_complete_task_not_assigned() -> anyhow::Result<()> {
        let pool = setup_test_db().await;
        let user_id = create_user_db(&pool, "compnotass", "compna@mail.com", "User", &None, &None).await?;
        let event_id = create_event(&pool, "Event", None, None, None, None, "#123".to_string()).await?;
        add_member(&pool, user_id, event_id, EventPermissions::OWNER).await?;

        let list = create_task_list(&pool, event_id, "Tasks", &["task1".to_string()], user_id).await?;
        let task_id = list.items[0].task_id;

        let result = complete_task(&pool, task_id, user_id, true).await;
        assert!(result.is_err());
        Ok(())
    }

    #[tokio::test]
    async fn test_delete_task_list() -> anyhow::Result<()> {
        let pool = setup_test_db().await;
        let user_id = create_user_db(&pool, "deltask", "deltask@mail.com", "User", &None, &None).await?;
        let event_id = create_event(&pool, "Event", None, None, None, None, "#123".to_string()).await?;
        add_member(&pool, user_id, event_id, EventPermissions::OWNER).await?;

        let list = create_task_list(&pool, event_id, "Tasks", &["task1".to_string()], user_id).await?;
        delete_task_list(&pool, list.task_list_id, event_id).await?;
        let result = get_task_list(&pool, list.task_list_id).await?;
        assert!(result.is_none());
        Ok(())
    }
}
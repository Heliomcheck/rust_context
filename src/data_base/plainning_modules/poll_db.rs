use sqlx::PgPool;
use std::str;
#[allow(unused_imports)]
use std::collections::HashMap;
use crate::models::PollVote;

use crate::{
    errors::AppError,
    structs::*,
};

pub struct PollWithVotes {
    pub options: Vec<String>,
    pub votes: Vec<PollVote>,
    pub votes_count: Vec<i32>,
    pub own_vote: Vec<i32>,
}

// #[derive(Debug, Serialize, Deserialize)]
// pub struct PollVote {
//     pub option_index: i32,
//     pub user_id: String,
// }

pub async fn create_poll(
    pool: &PgPool, 
    event_id: i64, 
    question: String,
    user_id: i64,
    options: Vec<String>,
    more_than_one_vote: bool,
) -> Result<i64, AppError> {
    if options.len() < 2 {
        return Err(AppError::BadRequest("Poll must have at least 2 options".to_string()));
    }

    if options.len() > 20 {
        return Err(AppError::BadRequest("Too many options (max 20)".to_string()));
    }

    let mut tx = pool.begin().await?;

    let create_poll = sqlx::query!(
        r#"
        INSERT INTO poll(event_id, question, created_by, more_than_one_vote)
        VALUES ($1, $2, $3, $4)
        RETURNING poll_id
        "#,
        event_id,
        question,
        user_id,
        more_than_one_vote
    )
    .fetch_one(&mut *tx)
    .await?;

    let poll_id = create_poll.poll_id;

    // Сохраняем опции с position
    for (position, option_text) in options.iter().enumerate() {
        sqlx::query!(
            r#"
            INSERT INTO poll_option (poll_id, option_text, position)
            VALUES ($1, $2, $3)
            "#,
            poll_id,
            option_text,
            position as i32
        )
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    Ok(poll_id)
}

#[allow(dead_code)]
pub async fn get_poll_by_id(
    pool: &PgPool,
    poll_id: i64,
) -> Result<Poll, AppError> {
    let poll_info = sqlx::query!(
        r#"
        SELECT 
            poll_id,
            question,
            created_by,
            created_at,
            is_active,
            more_than_one_vote,
            event_id
        FROM poll
        WHERE poll_id = $1 AND is_active = true
        "#,
        poll_id
    )
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("Poll {} not found", poll_id)))?;

    Ok(Poll {
        poll_id: poll_info.poll_id,
        question: poll_info.question,
        created_by: poll_info.created_by,
        created_at: poll_info.created_at,
        is_active: poll_info.is_active,
        multiple_choice: poll_info.more_than_one_vote,
    })
}

#[allow(dead_code)]
pub async fn get_count_of_options(
    pool: &PgPool,
    poll_id: i64
) -> Result<i64, sqlx::Error> {
    let poll_info = sqlx::query!(
        r#"
        SELECT more_than_one_vote
        FROM poll
        WHERE poll_id = $1
        "#,
        poll_id
    )
    .fetch_optional(pool)
    .await?;

    match poll_info {
        Some(info) => {
            if info.more_than_one_vote {
                Ok(100)
            } else {
                Ok(1)
            }
        },
        None => return Err(sqlx::Error::RowNotFound)
    }
}

pub async fn vote_on_poll(
    pool: &PgPool,
    poll_id: i64,
    user_id: i64,
    option_indexes: Vec<i32>,
) -> Result<bool, AppError> {
    // Получаем маппинг position → option_id
    let options = sqlx::query!(
        r#"
        SELECT option_id, position
        FROM poll_option
        WHERE poll_id = $1
        "#,
        poll_id
    )
    .fetch_all(pool)
    .await?;
    
    let position_to_option_id: std::collections::HashMap<i32, i64> = options
        .iter()
        .map(|row| (row.position, row.option_id))
        .collect();
    
    // Конвертируем индексы в option_id
    let mut option_ids = Vec::new();
    for idx in option_indexes {
        if let Some(option_id) = position_to_option_id.get(&idx) {
            option_ids.push(*option_id);
        } else {
            return Err(AppError::BadRequest(format!("Option index {} not found", idx)));
        }
    }
    
    // Транзакция
    let mut tx = pool.begin().await?;
    
    // Удаляем старые голоса
    sqlx::query!(
        r#"
        DELETE FROM poll_votes
        WHERE poll_id = $1 AND user_id = $2
        "#,
        poll_id,
        user_id
    )
    .execute(&mut *tx)
    .await?;
    
    // Добавляем новые голоса
    for option_id in option_ids {
        sqlx::query!(
            r#"
            INSERT INTO poll_votes (poll_id, user_id, option_id)
            VALUES ($1, $2, $3)
            "#,
            poll_id,
            user_id,
            option_id
        )
        .execute(&mut *tx)
        .await?;
    }
    
    tx.commit().await?;
    
    Ok(true)
}

pub async fn delete_poll(
    pool: &PgPool,
    poll_id: i64
) -> Result<bool, sqlx::Error> {
    let row = sqlx::query!(
        r#"
        DELETE FROM poll
        WHERE poll_id = $1
        "#,
        poll_id
    ).execute(pool)
    .await?;

    Ok(row.rows_affected() > 0)
}

pub async fn edit_pool_question(
    pool: &PgPool,
    poll_id: i64,
    question: String
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query!(
        r#"
        UPDATE poll
        SET question = $1
        WHERE poll_id = $2
        "#,
        question,
        poll_id
    )
    .execute(pool)
    .await?;

    Ok(result.rows_affected() == 1)
}

pub async fn get_event_polls(
    pool: &PgPool,
    event_id: i64
) -> Result<Vec<Poll>, sqlx::Error> {
    let poll_info = sqlx::query!(
        r#"
        SELECT 
            poll_id,
            question,
            created_by,
            created_at,
            is_active,
            more_than_one_vote
        FROM poll
        WHERE event_id = $1 AND is_active = true
        ORDER BY created_at DESC
        "#,
        event_id
    )
    .fetch_all(pool)
    .await?;

    let polls = poll_info.into_iter().map(|info| Poll {
        poll_id: info.poll_id,
        question: info.question,
        created_by: info.created_by,
        created_at: info.created_at,
        is_active: info.is_active,
        multiple_choice: info.more_than_one_vote
    }).collect();

    Ok(polls)
}

pub async fn get_poll_with_votes(
    pool: &PgPool,
    poll_id: i64,
    current_user_id: i64,
) -> Result<PollWithVotes, AppError> {
    // Получаем опции, сортируем по position
    let options_rows = sqlx::query!(
        r#"
        SELECT option_id, option_text, position
        FROM poll_option
        WHERE poll_id = $1
        ORDER BY position ASC
        "#,
        poll_id
    )
    .fetch_all(pool)
    .await?;
    
    let options: Vec<String> = options_rows
        .iter()
        .map(|row| row.option_text.clone())
        .collect();
    
    // Маппинг position -> не нужен, position и есть индекс
    // А вот option_id -> position понадобится для голосов
    let option_id_to_position: std::collections::HashMap<i64, i32> = options_rows
        .iter()
        .map(|row| (row.option_id, row.position))
        .collect();
    
    // Получаем все голоса
    let votes_rows = sqlx::query!(
        r#"
        SELECT pv.user_id, pv.option_id
        FROM poll_votes pv
        WHERE pv.poll_id = $1
        "#,
        poll_id
    )
    .fetch_all(pool)
    .await?;
    
    let mut votes: Vec<PollVote> = Vec::new();
    let mut votes_count = vec![0; options.len()];
    let mut own_vote = Vec::new();
    
    for row in votes_rows {
        if let Some(position) = option_id_to_position.get(&row.option_id) {
            let pos = *position as usize;
            votes_count[pos] += 1;
            
            votes.push(PollVote {
                option_index: *position,
                user_id: row.user_id.to_string(),
            });
            
            if row.user_id == current_user_id {
                own_vote.push(*position);
            }
        }
    }
    
    Ok(PollWithVotes {
        options,
        votes,
        votes_count,
        own_vote,
    })
}
//test
#[cfg(test)]
mod tests{
    use crate::permissions::EventPermissions;

    use super::*;
    use crate::data_base::event_db::create_event;
    use crate::data_base::user_db::create_user_db;
    use crate::data_base::event_db::add_member;

    use crate::test_utils::setup_test_db;

    #[tokio::test]
    async fn test_get_count_of_options_single_vote() {
        let pool = setup_test_db().await;
        
        // ✅ Создаем пользователя
        let user_id = create_user_db(
            &pool,
            "testuser1",
            "test1@example.com",
            "Test User 1",
            &None,
            &None,
        )
        .await
        .unwrap();
        
        // ✅ Создаем событие
        let event_id = create_event(
            &pool,
            "Test Event",
            None,
            None,
            None,
            Some("uiu".to_string()),
            "#123456".to_string(),
        )
        .await
        .unwrap();
        
        add_member(&pool, user_id, event_id, EventPermissions::OWNER).await.unwrap();

        let poll_id = create_poll(
            &pool,
            event_id,  // ← используем реальный event_id
            "Single vote".to_string(),
            user_id,   // ← используем реальный user_id
            vec!["A".to_string(), "B".to_string()],
            false,
        )
        .await
        .unwrap();

        let count = get_count_of_options(&pool, poll_id).await.unwrap();
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn test_get_count_of_options_multiple_vote() {
        let pool = setup_test_db().await;
        
        // ✅ Создаем пользователя
        let user_id = create_user_db(
            &pool,
            "testuser2",
            "test2@example.com",
            "Test User 2",
            &None,
            &None,
        )
        .await
        .unwrap();
        
        // ✅ Создаем событие
        let event_id = create_event(
            &pool,
            "Test Event 2",
            None,
            None,
            None,
            Some("uiu".to_string()),
            "#123456".to_string(),
        )
        .await
        .unwrap();
        
        add_member(&pool, user_id, event_id, EventPermissions::OWNER).await.unwrap();

        let poll_id = create_poll(
            &pool,
            event_id,
            "Multiple vote".to_string(),
            user_id,
            vec!["A".to_string(), "B".to_string()],
            true,
        )
        .await
        .unwrap();

        let count = get_count_of_options(&pool, poll_id).await.unwrap();
        assert_eq!(count, 100);
    }

    #[tokio::test]
    async fn test_vote_on_poll() {
        let pool = setup_test_db().await;
        
        let voter_id = create_user_db(
            &pool,
            "voter",
            "voter@example.com",
            "Voter",
            &None,
            &None,
        )
        .await
        .unwrap();
        
        let creator_id = create_user_db(
            &pool,
            "creator",
            "creator@example.com",
            "Creator",
            &None,
            &None,
        )
        .await
        .unwrap();
        
        let event_id = create_event(
            &pool,
            "Test Event",
            None,
            None,
            None,
            Some("uiu".to_string()),
            "#123456".to_string(),
        )
        .await
        .unwrap();
        
        add_member(&pool, creator_id, event_id, EventPermissions::OWNER).await.unwrap();
        add_member(&pool, voter_id, event_id, EventPermissions::ADMIN).await.unwrap();

        let poll_id = create_poll(
            &pool,
            event_id,
            "Vote test".to_string(),
            creator_id,
            vec!["A".to_string(), "B".to_string()],
            true,
        )
        .await
        .unwrap();

        // Получаем позиции (индексы), а не option_id
        let options = sqlx::query!(
            r#"
            SELECT position
            FROM poll_option
            WHERE poll_id = $1
            ORDER BY position ASC
            "#,
            poll_id
        )
        .fetch_all(&pool)
        .await
        .unwrap();

        // Теперь передаём индексы (position), а не option_id
        let option_indexes: Vec<i32> = options.into_iter().map(|o| o.position).collect();

        let result = vote_on_poll(
            &pool,
            poll_id,
            voter_id,
            vec![option_indexes[0]],  // 👈 теперь i32
        )
        .await
        .unwrap();

        assert!(result);

        // Проверяем, что голос сохранился (нужно проверить через get_poll_with_votes)
        let poll_with_votes = get_poll_with_votes(&pool, poll_id, voter_id).await.unwrap();
        
        assert_eq!(poll_with_votes.votes_count[0], 1);
        assert_eq!(poll_with_votes.own_vote, vec![0]);
    }

    #[tokio::test]
    async fn test_delete_poll() {
        let pool = setup_test_db().await;
        
        // ✅ Создаем пользователя и событие
        let user_id = create_user_db(
            &pool,
            "deleteuser",
            "delete@example.com",
            "Delete User",
            &None,
            &None,
        )
        .await
        .unwrap();
        
        let event_id = create_event(
            &pool,
            "Delete Event",
            None,
            None,
            None,
            Some("uiu".to_string()),
            "#123456".to_string(),
        )
        .await
        .unwrap();
        
        add_member(&pool, user_id, event_id, EventPermissions::OWNER).await.unwrap();

        let poll_id = create_poll(
            &pool,
            event_id,
            "Delete test".to_string(),
            user_id,
            vec!["A".to_string(), "B".to_string()],
            false,
        )
        .await
        .unwrap();

        let deleted = delete_poll(&pool, poll_id).await.unwrap();
        assert!(deleted);

        let deleted_poll = sqlx::query!(
            r#"
            SELECT poll_id
            FROM poll
            WHERE poll_id = $1
            "#,
            poll_id
        )
        .fetch_optional(&pool)
        .await
        .unwrap();

        assert!(deleted_poll.is_none());
    }

    #[tokio::test]
    async fn test_edit_poll_question() {
        let pool = setup_test_db().await;
        
        // ✅ Создаем пользователя и событие
        let user_id = create_user_db(
            &pool,
            "edituser",
            "edit@example.com",
            "Edit User",
            &None,
            &None,
        )
        .await
        .unwrap();
        
        let event_id = create_event(
            &pool,
            "Edit Event",
            None,
            None,
            None,
            Some("uiu".to_string()),
            "#123456".to_string(),
        )
        .await
        .unwrap();
        
        add_member(&pool, user_id, event_id, EventPermissions::OWNER).await.unwrap();

        let poll_id = create_poll(
            &pool,
            event_id,
            "Old question".to_string(),
            user_id,
            vec!["A".to_string(), "B".to_string()],
            false,
        )
        .await
        .unwrap();

        let updated = edit_pool_question(&pool, poll_id, "New question".to_string())
            .await
            .unwrap();

        assert!(updated);

        let poll = sqlx::query!(
            r#"
            SELECT question
            FROM poll
            WHERE poll_id = $1
            "#,
            poll_id
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(poll.question, "New question");
    }

    #[tokio::test]
    async fn test_get_event_polls() {
        let pool = setup_test_db().await;
        
        // ✅ Создаем пользователя и событие
        let user_id = create_user_db(
            &pool,
            "eventpolls",
            "eventpolls@example.com",
            "Event Polls User",
            &None,
            &None,
        )
        .await
        .unwrap();
        
        let event_id = create_event(
            &pool,
            "Polls Event",
            None,
            None,
            None,
            Some("uiu".to_string()),
            "#123456".to_string(),
        )
        .await
        .unwrap();
        
        add_member(&pool, user_id, event_id, EventPermissions::OWNER).await.unwrap();

        create_poll(
            &pool,
            event_id,  // ← используем реальный event_id
            "Poll 1".to_string(),
            user_id,
            vec!["A".to_string(), "B".to_string()],  // ← минимум 2 опции
            false,
        )
        .await
        .unwrap();

        create_poll(
            &pool,
            event_id,
            "Poll 2".to_string(),
            user_id,
            vec!["C".to_string(), "D".to_string()],  // ← минимум 2 опции
            true,
        )
        .await
        .unwrap();

        let polls = get_event_polls(&pool, event_id).await.unwrap();

        assert_eq!(polls.len(), 2);
        assert!(polls.iter().any(|p| p.question == "Poll 1"));
        assert!(polls.iter().any(|p| p.question == "Poll 2"));
    }
}
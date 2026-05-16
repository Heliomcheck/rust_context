use sqlx::PgPool;
use std::str;

use crate::{
    errors::AppError,
    structs::*,
};

pub async fn create_poll(
    pool: &PgPool, 
    event_id: i64, 
    question: String,
    user_id: i64,
    options: Vec<String>,
    more_than_one_vote: bool,
) -> Result<i64, sqlx::Error> {
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
    ).fetch_one(pool)
    .await?;

    let poll_id = create_poll.poll_id;

    let add_options = sqlx::query!(
        r#"
        INSERT INTO poll_option (poll_id, option_text)
        SELECT $1, unnest($2::text[])
        "#,
        poll_id,
        &options
    ).execute(pool)
    .await?;

    Ok(create_poll.poll_id)
} 

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

    let _ = match poll_info {
        Some(info) => {
            if info.more_than_one_vote {
                return Ok(100); // max count of options
            } else {
                return Ok(1); // min count of options
            }
        },
        None => return Err(sqlx::Error::RowNotFound)
    };
}

pub async fn vote_on_poll(
    pool: &PgPool,
    poll_id: i64,
    user_id: i64,
    option_ids: Vec<i64>,
) -> Result<bool, sqlx::Error> {
    // 3. Удаляем все старые голоса пользователя
    sqlx::query!(
        r#"
        DELETE FROM poll_votes
        WHERE poll_id = $1 AND user_id = $2
        "#,
        poll_id,
        user_id
    )
    .execute(pool)
    .await?;

    // 4. Добавляем новые голоса
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
        .execute(pool)
        .await?;
    }

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
        more_than_one_vote: info.more_than_one_vote
    }).collect();

    Ok(polls)
}
//test
#[cfg(test)]
mod tests{
    use crate::data_base;
    use crate::permissions::EventPermissions;

    use super::*;
    use sqlx::{PgPool, Executor};
    use crate::data_base::event_db::create_event;
    use crate::data_base::user_db::create_user_db;
    use crate::data_base::event_db::add_member;

    use crate::test_utils::setup_test_db;

    #[tokio::test]
    async fn test_create_poll() {
        let pool = setup_test_db().await;
        let user_id = create_user_db(
            &pool,
            "testuser",
            "test@example.com",
            "Test User",
            &None,
            &None,
            &None,
        )
        .await
        .unwrap();
        
        // 2. Создаём событие
        let event_id = create_event(
            &pool,
            "Test Event",
            None,
            None,
            None,
            "#123456".to_string(),
        )
        .await
        .unwrap();

        add_member(&pool, user_id, event_id, EventPermissions::CREATE_MODULE, 2).await.unwrap();

        let poll_id = create_poll(
            &pool, 
            event_id,
            "What is your favorite color?".to_string(),
            user_id, 
            vec![
                "Red".to_string(), 
                "Blue".to_string()
                ], 
            false
        )
        .await
        .unwrap();
        assert!(poll_id > 0);

        let poll = sqlx::query!(
            r#"
            SELECT question, created_by, more_than_one_vote
            FROM poll
            WHERE poll_id = $1
            "#,
            poll_id
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(poll.question, "What is your favorite color?");
        assert_eq!(poll.created_by, 1);
        assert_eq!(poll.more_than_one_vote, false);
    }
    
    #[tokio::test]
    async fn test_get_count_of_options_single_vote() {
        let pool = setup_test_db().await;

        let poll_id = create_poll(
            &pool,
            1,
            "Single vote".to_string(),
            1,
            vec!["A".to_string(), "B".to_string()],
            false,
        )
        .await
        .unwrap();

        let count = get_count_of_options(&pool, poll_id)
            .await
            .unwrap();

        assert_eq!(count, 1);
    }
    #[tokio::test]
    async fn test_get_count_of_options_multiple_vote() {
        let pool = setup_test_db().await;

        let poll_id = create_poll(
            &pool,
            1,
            "Multiple vote".to_string(),
            1,
            vec!["A".to_string(), "B".to_string()],
            true,
        )
        .await
        .unwrap();

        let count = get_count_of_options(&pool, poll_id)
            .await
            .unwrap();

        assert_eq!(count, 100);
    }
    #[tokio::test]
    async fn test_vote_on_poll() {
        let pool = setup_test_db().await;

        let poll_id = create_poll(
            &pool,
            1,
            "Vote test".to_string(),
            1,
            vec!["A".to_string(), "B".to_string()],
            true,
        )
        .await
        .unwrap();

        let option_id = sqlx::query!(
            r#"
            SELECT option_id
            FROM poll_option
            WHERE poll_id = $1
            ORDER BY option_id
            "#,
            poll_id
        )
        .fetch_all(&pool)
        .await
        .unwrap();

        let option_ids: Vec<i64> = option_id.into_iter().map(|o| o.option_id).collect();

        // Голосуем
        let result = vote_on_poll(
            &pool,
            poll_id,
            42,           // ← запятая была пропущена!
            vec![option_ids[0]],
        )
        .await
        .unwrap();

        assert!(result);

        // Проверяем, что голос сохранился
        let votes = sqlx::query!(
            r#"
            SELECT option_id
            FROM poll_votes
            WHERE poll_id = $1 AND user_id = $2
            "#,
            poll_id,
            42
        )
        .fetch_all(&pool)
        .await
        .unwrap();

        assert_eq!(votes.len(), 1);
        assert_eq!(votes[0].option_id, option_ids[0]);
    }

    #[tokio::test]
    async fn test_delete_poll() {
        let pool = setup_test_db().await;

        let poll_id = create_poll(
            &pool,
            1,
            "Delete test".to_string(),
            1,
            vec!["A".to_string()],
            false,
        )
        .await
        .unwrap();

        let deleted = delete_poll(&pool, poll_id)
            .await
            .unwrap();

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

        let poll_id = create_poll(
            &pool,
            1,
            "Old question".to_string(),
            1,
            vec!["A".to_string()],
            false,
        )
        .await
        .unwrap();

        let updated = edit_pool_question(
            &pool,
            poll_id,
            "New question".to_string(),
        )
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

        create_poll(
            &pool,
            777,
            "Poll 1".to_string(),
            1,
            vec!["A".to_string()],
            false,
        )
        .await
        .unwrap();

        create_poll(
            &pool,
            777,
            "Poll 2".to_string(),
            2,
            vec!["B".to_string()],
            true,
        )
        .await
        .unwrap();

        let polls = get_event_polls(&pool, 777)
            .await
            .unwrap();

        assert_eq!(polls.len(), 2);

        assert!(polls.iter().any(|p| p.question == "Poll 1"));
        assert!(polls.iter().any(|p| p.question == "Poll 2"));
    }
}
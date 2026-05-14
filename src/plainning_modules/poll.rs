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
    ).fetch_one(pool)
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
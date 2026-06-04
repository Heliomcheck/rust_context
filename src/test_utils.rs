#[allow(unused_imports)]
use sqlx::postgres::{PgPoolOptions, PgPool};
#[allow(unused_imports)]
use sqlx::Executor;
use std::sync::OnceLock;
use tokio::sync::Mutex;
use crate::{
    data_base::user_db::create_token,
    data_base::user_db::create_user_db,
    data_base::event_db::create_event
};
use chrono::Utc;

#[cfg(test)]
pub async fn setup_test_db() -> PgPool {
    let _guard = lock_db().await;
    dotenvy::dotenv().ok();
    
    let database_url = std::env::var("DATABASE_URL_TEST")
        .expect("DATABASE_URL_TEST must be set");
    
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to connect to test database");
    
    sqlx::migrate!().run(&pool).await.expect("Failed to run migrations");
    
    clear_db(&pool).await;
    
    pool
}

#[cfg(test)]
pub async fn clear_db(pool: &PgPool) {
    // Отключаем проверку внешних ключей временно для чистого удаления
    let _ = pool.execute("ALTER TABLE IF EXISTS poll_votes DISABLE TRIGGER ALL").await;
    let _ = pool.execute("ALTER TABLE IF EXISTS poll_option DISABLE TRIGGER ALL").await;
    let _ = pool.execute("ALTER TABLE IF EXISTS poll DISABLE TRIGGER ALL").await;
    let _ = pool.execute("ALTER TABLE IF EXISTS event_user DISABLE TRIGGER ALL").await;
    let _ = pool.execute("ALTER TABLE IF EXISTS event_token DISABLE TRIGGER ALL").await;
    let _ = pool.execute("ALTER TABLE IF EXISTS token_store DISABLE TRIGGER ALL").await;
    let _ = pool.execute("ALTER TABLE IF EXISTS events DISABLE TRIGGER ALL").await;
    let _ = pool.execute("ALTER TABLE IF EXISTS users DISABLE TRIGGER ALL").await;
    
    // Очищаем таблицы в порядке от зависимых к основным
    let _ = pool.execute("TRUNCATE TABLE poll_votes CASCADE").await;
    let _ = pool.execute("TRUNCATE TABLE poll_option CASCADE").await;
    let _ = pool.execute("TRUNCATE TABLE poll CASCADE").await;
    let _ = pool.execute("TRUNCATE TABLE event_user CASCADE").await;
    let _ = pool.execute("TRUNCATE TABLE event_token CASCADE").await;
    let _ = pool.execute("TRUNCATE TABLE token_store CASCADE").await;
    let _ = pool.execute("TRUNCATE TABLE events CASCADE").await;
    let _ = pool.execute("TRUNCATE TABLE users CASCADE").await;
    
    // Сбрасываем последовательности (sequences) для BIGSERIAL полей
    let _ = pool.execute("ALTER SEQUENCE users_user_id_seq RESTART WITH 1").await;
    let _ = pool.execute("ALTER SEQUENCE events_event_id_seq RESTART WITH 1").await;
    let _ = pool.execute("ALTER SEQUENCE token_store_token_id_seq RESTART WITH 1").await;
    let _ = pool.execute("ALTER SEQUENCE poll_poll_id_seq RESTART WITH 1").await;
    let _ = pool.execute("ALTER SEQUENCE poll_option_option_id_seq RESTART WITH 1").await;
    
    // Включаем обратно триггеры
    let _ = pool.execute("ALTER TABLE IF EXISTS poll_votes ENABLE TRIGGER ALL").await;
    let _ = pool.execute("ALTER TABLE IF EXISTS poll_option ENABLE TRIGGER ALL").await;
    let _ = pool.execute("ALTER TABLE IF EXISTS poll ENABLE TRIGGER ALL").await;
    let _ = pool.execute("ALTER TABLE IF EXISTS event_user ENABLE TRIGGER ALL").await;
    let _ = pool.execute("ALTER TABLE IF EXISTS event_token ENABLE TRIGGER ALL").await;
    let _ = pool.execute("ALTER TABLE IF EXISTS token_store ENABLE TRIGGER ALL").await;
    let _ = pool.execute("ALTER TABLE IF EXISTS events ENABLE TRIGGER ALL").await;
    let _ = pool.execute("ALTER TABLE IF EXISTS users ENABLE TRIGGER ALL").await;
}

#[cfg(test)]
static DB_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

#[cfg(test)]
pub async fn lock_db() -> tokio::sync::MutexGuard<'static, ()> {
    DB_LOCK.get_or_init(|| Mutex::new(())).lock().await
}

pub async fn new_user_and_token(pool: &sqlx::PgPool, name: &str, email: &str, token_str: &str) -> (i64, String) {
    let uid = create_user_db(
        pool, 
        name, 
        email, 
        name, 
        &None, 
        &None
    ).await.unwrap();
    create_token(
        pool, 
        uid, 
        token_str, 
        Utc::now() + chrono::Duration::hours(1)
    ).await.unwrap();

    (uid, token_str.to_string())
}

#[cfg(test)]
pub async fn new_event(pool: &sqlx::PgPool) -> i64 {
    create_event(
        pool, "Test Event", 
        Some("Desc".into()),
        None,
        None,
        Some("Room".into()),
        "#123456".into()
        ).await.unwrap()
}

#[utoipa::path(
    get,
    path = "/health",
    tag = "test_server",
    responses(
        (status = 200, description = "Server start successfull")
    )
)]
pub async fn health_handler() -> &'static str {
    "OK"
}
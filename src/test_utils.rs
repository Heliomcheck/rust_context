use sqlx::postgres::{PgPoolOptions, PgPool};
use sqlx::Executor;

pub async fn setup_test_db() -> PgPool {
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

pub async fn clear_db(pool: &PgPool) {
    // Порядок важен из-за внешних ключей
    let _ = pool.execute("DELETE FROM tokenstore").await;
    let _ = pool.execute("DELETE FROM users").await;
}
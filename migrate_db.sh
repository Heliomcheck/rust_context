docker-compose restart

sqlx db drop
sqlx db create
sqlx migrate run

sqlx db drop -y --database-url=postgres://postgres:postgres@localhost:5433/rust_context_test
sqlx db create --database-url=postgres://postgres:postgres@localhost:5433/rust_context_test
sqlx migrate run --database-url=postgres://postgres:postgres@localhost:5433/rust_context_test

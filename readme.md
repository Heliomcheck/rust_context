Сервер на rust для управления событиями, чатом, пользователями
---
Client: https://github.com/CQuann/Krug.git

## Технологии
- Rust + Tokio (асинхронный рантайм)
- Axum (веб‑фреймворк)
- SQLx (асинхронный драйвер PostgreSQL)
- PostgreSQL (основная БД)
- Docker / Docker Compose
- WebSocket (чат в реальном времени)

## Install:
```bash
docker-compose up -d  # for database
mv env .env # for set .env file
cargo build
cargo install --path .
```

## Usage:
```bash
rust_context ip:port
```

## Tests:
```bash
cargo test -- --test-threads=1
```

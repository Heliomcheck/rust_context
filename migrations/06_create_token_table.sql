-- Add migration script here
CREATE TABLE IF NOT EXISTS token_store (
    token_id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    token VARCHAR(32) UNIQUE NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL,
    is_active BOOLEAN DEFAULT TRUE
);

-- Индексы для быстрого поиска
CREATE INDEX IF NOT EXISTS idx_token_store_user_id ON token_store(user_id);
CREATE INDEX IF NOT EXISTS idx_token_store_token ON token_store(token);
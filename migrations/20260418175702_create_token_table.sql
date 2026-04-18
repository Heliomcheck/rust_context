-- Add migration script here
CREATE TABLE IF NOT EXISTS tokenstore (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token VARCHAR(32) UNIQUE NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL,
    is_active BOOLEAN DEFAULT TRUE
);

-- Индексы для быстрого поиска
CREATE INDEX IF NOT EXISTS idx_tokenstore_user_id ON tokenstore(user_id);
CREATE INDEX IF NOT EXISTS idx_tokenstore_token ON tokenstore(token);
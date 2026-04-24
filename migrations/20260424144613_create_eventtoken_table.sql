-- Add migration script here
CREATE TABLE IF NOT EXISTS eventtoken(
    eventtoken_id BIGSERIAL PRIMARY KEY,
    event_id BIGINT NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    event_token VARCHAR(32) UNIQUE NOT NULL,
    expires_ad TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
)

CREATE INDEX IF NOT EXISTS idx_eventtoken_event_id ON eventtoken(event_id);
CREATE INDEX IF NOT EXISTS idx_eventtoken_even_token ON eventtoken(event_token);
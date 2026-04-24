-- Add migration script here
CREATE TABLE event_token (
    event_token VARCHAR(64) PRIMARY KEY,
    event_id BIGINT NOT NULL REFERENCES events(event_id) ON DELETE CASCADE,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_event_tokens_event_id ON event_token(event_id);
CREATE INDEX idx_event_tokens_expires_at ON event_token(expires_at);
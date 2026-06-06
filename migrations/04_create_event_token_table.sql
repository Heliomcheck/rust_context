-- Add migration script here
CREATE TABLE event_token (
    event_token VARCHAR(200) PRIMARY KEY,
    event_id BIGINT NOT NULL REFERENCES events(event_id) ON DELETE CASCADE,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

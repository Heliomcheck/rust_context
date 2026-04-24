-- Add migration script here
CREATE TABLE IF NOT EXISTS events (
    event_id BIGSERIAL PRIMARY KEY,
    event_name VARCHAR(100) UNIQUE NOT NULL,
    start_data TIMESTAMPTZ,
    end_data TIMESTAMPTZ,
    event_avatar TEXT,
    descriptions TEXT
) 

CREATE INDEX IF NOT EXISTS idx_event_eventname ON events(event_name);
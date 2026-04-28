-- Add migration script here
CREATE TABLE events (
    event_name VARCHAR(200) NOT NULL,
    event_id BIGSERIAL PRIMARY KEY,
    description_profile TEXT DEFAULT NULL,
    start_date TIMESTAMPTZ DEFAULT NULL,
    end_date TIMESTAMPTZ DEFAULT NULL,
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    status_id SMALLINT NOT NULL REFERENCES event_statuses(status_id) DEFAULT 2
);

CREATE INDEX idx_events_status_id ON events(status_id);
CREATE INDEX idx_events_start_date ON events(start_date);
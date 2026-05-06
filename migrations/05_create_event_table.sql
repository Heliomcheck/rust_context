-- Add migration script here
CREATE TABLE events (
    event_name VARCHAR(200) NOT NULL,
    event_id BIGSERIAL PRIMARY KEY,
    description_event VARCHAR(200) DEFAULT NULL,
    start_date TIMESTAMPTZ DEFAULT NULL,
    end_date TIMESTAMPTZ DEFAULT NULL,
    color VARCHAR(7) NOT NULL DEFAULT '#123456' ,
    is_active BOOLEAN DEFAULT TRUE NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW() NOT NULL,
    status_id SMALLINT NOT NULL REFERENCES event_statuses(status_id) DEFAULT 2
);

CREATE INDEX idx_events_status_id ON events(status_id);
CREATE INDEX idx_events_start_date ON events(start_date);
-- Add migration script here
CREATE TABLE events (
    event_name VARCHAR(200) NOT NULL,
    event_id BIGSERIAL PRIMARY KEY,
    description_event VARCHAR(200) DEFAULT NULL,
    start_date TIMESTAMPTZ DEFAULT NULL,
    end_date TIMESTAMPTZ DEFAULT NULL,
    color VARCHAR(7) NOT NULL DEFAULT '#123456' ,
    location VARCHAR(200),
    created_at TIMESTAMPTZ DEFAULT NOW() NOT NULL,
    status_event VARCHAR(30) NOT NULL DEFAULT 'OPEN'
);

CREATE INDEX idx_events_status_event ON events(status_event);
CREATE INDEX idx_events_start_date ON events(start_date);
-- Add migration script here
CREATE TABLE events (
    title VARCHAR(200) NOT NULL,
    event_id BIGSERIAL PRIMARY KEY,
    description_event VARCHAR(200) DEFAULT NULL,
    start_date_time TIMESTAMPTZ DEFAULT NULL,
    end_date_time TIMESTAMPTZ DEFAULT NULL,
    color VARCHAR(7) NOT NULL DEFAULT '#123456' ,
    location VARCHAR(200),
    created_at TIMESTAMPTZ DEFAULT NOW() NOT NULL,
    status_event VARCHAR(30) NOT NULL DEFAULT 'active',
    album_updated_at TIMESTAMPTZ DEFAULT NOW(),
    album_size BIGINT DEFAULT 0,
    album_photos_count INT DEFAULT 0
);

CREATE INDEX idx_events_status_event ON events(status_event);
CREATE INDEX idx_events_start_date ON events(start_date_time);
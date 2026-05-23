CREATE TABLE IF NOT EXISTS items (
    item_id BIGSERIAL PRIMARY KEY,
    event_id BIGINT NOT NULL REFERENCES events(event_id) ON DELETE CASCADE,
    content TEXT NOT NULL,
    done BOOLEAN NOT NULL DEFAULT FALSE,
    created_by BIGINT NOT NULL REFERENCES users(user_id),
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_items_event_id ON items(event_id);
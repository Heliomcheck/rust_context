-- Add migration script here
CREATE TABLE item_list (
    item_list_id BIGSERIAL PRIMARY KEY,
    event_id BIGINT NOT NULL REFERENCES events(event_id) ON DELETE CASCADE,
    title VARCHAR(200) NOT NULL,
    created_by BIGINT NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    is_active BOOLEAN NOT NULL DEFAULT TRUE
);

CREATE TABLE item_list_item (
    item_id BIGSERIAL PRIMARY KEY,
    item_list_id BIGINT NOT NULL REFERENCES item_list(item_list_id) ON DELETE CASCADE,
    item_text TEXT NOT NULL,
    assigned_user_id BIGINT REFERENCES users(user_id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_item_list_event_id ON item_list(event_id);
CREATE INDEX idx_item_list_item_list_id ON item_list_item(item_list_id);
CREATE INDEX idx_item_list_assigned_user ON item_list_item(assigned_user_id);
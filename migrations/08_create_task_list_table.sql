-- Add migration script here
CREATE TABLE IF NOT EXISTS task_list (
    task_list_id BIGSERIAL PRIMARY KEY,
    event_id BIGINT NOT NULL REFERENCES events(event_id) ON DELETE CASCADE,
    title VARCHAR(200) NOT NULL,
    created_by BIGINT NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    is_active BOOLEAN NOT NULL DEFAULT TRUE
);

CREATE TABLE IF NOT EXISTS task_list_item (
    task_id BIGSERIAL PRIMARY KEY,
    task_list_id BIGINT NOT NULL REFERENCES task_list(task_list_id) ON DELETE CASCADE,
    task_text TEXT NOT NULL,
    assigned_user_id BIGINT REFERENCES users(user_id) ON DELETE SET NULL,
    is_completed BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_task_list_event_id ON task_list(event_id);
CREATE INDEX IF NOT EXISTS idx_task_list_item_list_id ON task_list_item(task_list_id);
CREATE INDEX IF NOT EXISTS idx_task_list_assigned_user ON task_list_item(assigned_user_id);
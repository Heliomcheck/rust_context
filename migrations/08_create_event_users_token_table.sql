-- Add migration script here
CREATE TABLE event_user (
    user_id BIGINT NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    event_id BIGINT NOT NULL REFERENCES events(event_id) ON DELETE CASCADE,
    role_id SMALLINT NOT NULL REFERENCES event_roles(role_id) DEFAULT 3,
    status_id SMALLINT NOT NULL REFERENCES participant_statuses(status_id) DEFAULT 1,
    joined_at TIMESTAMPTZ DEFAULT NOW(),
    PRIMARY KEY (user_id, event_id)
);

CREATE INDEX idx_event_user_user_id ON event_user(user_id);
CREATE INDEX idx_event_user_event_id ON event_user(event_id);
CREATE INDEX idx_event_user_role_id ON event_user(role_id);
CREATE INDEX idx_event_user_status_id ON event_user(status_id);
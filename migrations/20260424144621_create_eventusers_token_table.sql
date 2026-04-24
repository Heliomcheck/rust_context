-- Add migration script here
CREATE TYPE roleinevent AS ENUM('Master', 'Slave');

CREATE TABLE IF NOT EXISTS eventusers(
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    event_id BIGINT NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    roles roleinevent DEFAULT 'Slave',
    entry_date TIMESTAMPTZ DEFAULT NOW()
)

CREATE INDEX IF NOT EXISTS idx_eventusers_user_id ON eventusers(user_id);
CREATE INDEX IF NOT EXISTS idx_eventusers_event_id ON eventusers(event_id);
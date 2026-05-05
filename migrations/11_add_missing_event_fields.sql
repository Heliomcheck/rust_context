-- Добавляем отсутствующие поля для событий
ALTER TABLE events 
ADD COLUMN IF NOT EXISTS color VARCHAR(7),
ADD COLUMN IF NOT EXISTS avatar_uploaded BOOLEAN DEFAULT FALSE;

-- Добавляем индекс для быстрого поиска по пользователю-владельцу
CREATE INDEX IF NOT EXISTS idx_event_user_owner ON event_user(event_id, role_id) WHERE role_id = 1;

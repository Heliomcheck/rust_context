CREATE TABLE IF NOT EXISTS event_photos (
    photo_id BIGSERIAL PRIMARY KEY,
    event_id BIGINT NOT NULL REFERENCES events(event_id) ON DELETE CASCADE,
    etag TEXT,
    file_name VARCHAR(255) NOT NULL,
    original_name VARCHAR(255),
    mime_type VARCHAR(100),
    file_size BIGINT NOT NULL,
    uploaded_by BIGINT NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    is_active BOOLEAN NOT NULL DEFAULT TRUE
);

CREATE INDEX IF NOT EXISTS idx_event_photos_event_id ON event_photos(event_id);
CREATE INDEX IF NOT EXISTS idx_event_photos_uploaded_by ON event_photos(uploaded_by);
CREATE INDEX IF NOT EXISTS idx_event_photos_active ON event_photos(event_id, is_active);
CREATE INDEX IF NOT EXISTS idx_event_photos_etag ON event_photos(event_id);

CREATE OR REPLACE FUNCTION update_photo_etag()
RETURNS TRIGGER AS $$
BEGIN
    NEW.etag := md5(random()::text || now()::text || NEW.photo_id::text);
    NEW.updated_at := NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_update_photo_etag
BEFORE INSERT OR UPDATE ON event_photos
FOR EACH ROW
EXECUTE FUNCTION update_photo_etag();
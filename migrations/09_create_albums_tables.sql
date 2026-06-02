CREATE TABLE IF NOT EXISTS albums (
    album_id BIGSERIAL PRIMARY KEY,
    event_id BIGINT NOT NULL REFERENCES events(event_id) ON DELETE CASCADE,
    title VARCHAR(200) NOT NULL,
    description TEXT,
    created_by BIGINT NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    is_active BOOLEAN NOT NULL DEFAULT TRUE
);

CREATE INDEX idx_albums_event_id ON albums(event_id);
CREATE INDEX idx_albums_created_by ON albums(created_by);

CREATE TABLE IF NOT EXISTS album_photos (
    photo_id BIGSERIAL PRIMARY KEY,
    album_id BIGINT NOT NULL REFERENCES albums(album_id) ON DELETE CASCADE,
    file_name VARCHAR(255) NOT NULL,       -- уникальное имя файла на диске (например, "abc123.jpg")
    original_name VARCHAR(255),            -- оригинальное имя при загрузке
    mime_type VARCHAR(100),
    file_size BIGINT,
    uploaded_by BIGINT NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    is_active BOOLEAN NOT NULL DEFAULT TRUE
);

CREATE INDEX idx_album_photos_album_id ON album_photos(album_id);
CREATE INDEX idx_album_photos_uploaded_by ON album_photos(uploaded_by);
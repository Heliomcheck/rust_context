CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

ALTER TABLE events
DROP CONSTRAINT events_pkey;

ALTER TABLE events
ALTER COLUMN event_id
TYPE UUID
USING uuid_generate_v4();

ALTER TABLE events
ADD PRIMARY KEY(event_id);
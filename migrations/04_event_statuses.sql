-- Add migration script here
CREATE TABLE IF NOT EXISTS event_statuses (
    status_id SMALLINT PRIMARY KEY,
    status_name VARCHAR(20) UNIQUE NOT NULL,
    is_editable BOOLEAN DEFAULT TRUE,
    can_join BOOLEAN DEFAULT TRUE
);

INSERT INTO event_statuses (status_id, status_name, is_editable, can_join) VALUES
(1, 'draft', TRUE, FALSE),
(2, 'open', TRUE, TRUE),
(3, 'in_progress', FALSE, FALSE),
(4, 'completed', FALSE, FALSE),
(5, 'cancelled', FALSE, FALSE),
(6, 'archived', FALSE, FALSE);
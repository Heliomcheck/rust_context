-- Add migration script here
CREATE TABLE IF NOT EXISTS event_statuses (
    status_id SMALLINT PRIMARY KEY,
    status_name VARCHAR(20) UNIQUE NOT NULL,
    description_profile TEXT,
    is_editable BOOLEAN DEFAULT TRUE,
    can_join BOOLEAN DEFAULT TRUE
);

INSERT INTO event_statuses (status_id, status_name, description_profile, is_editable, can_join) VALUES
(1, 'draft', 'Черновик', TRUE, FALSE),
(2, 'open', 'Открыт для участников', TRUE, TRUE),
(3, 'in_progress', 'В процессе', FALSE, FALSE),
(4, 'completed', 'Завершён', FALSE, FALSE),
(5, 'cancelled', 'Отменён', FALSE, FALSE),
(6, 'archived', 'В архиве', FALSE, FALSE);
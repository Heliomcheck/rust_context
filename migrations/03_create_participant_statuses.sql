-- Add migration script here
CREATE TABLE IF NOT EXISTS participant_statuses (
    status_id SMALLINT PRIMARY KEY,
    status_name VARCHAR(20) UNIQUE NOT NULL,
    description TEXT,
    is_active BOOLEAN DEFAULT TRUE
);

INSERT INTO participant_statuses (status_id, status_name, description, is_active) VALUES
(1, 'pending', 'Wait approve', TRUE),
(2, 'approved', 'Approved', TRUE),
(3, 'rejected', 'rejected', FALSE)
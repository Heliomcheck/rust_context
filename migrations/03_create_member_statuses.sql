-- Add migration script here
CREATE TABLE IF NOT EXISTS member_statuses (
    status_id SMALLINT PRIMARY KEY,
    status_name VARCHAR(20) UNIQUE NOT NULL,
    is_active BOOLEAN DEFAULT TRUE
);

INSERT INTO member_statuses (status_id, status_name, is_active) VALUES
(1, 'pending', TRUE),
(2, 'approved', TRUE),
(3, 'rejected', FALSE)
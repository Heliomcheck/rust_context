-- Add migration script here
CREATE TABLE IF NOT EXISTS event_roles (
    role_id SMALLINT PRIMARY KEY,
    role_name VARCHAR(20) UNIQUE NOT NULL,
    priority SMALLINT NOT NULL,
    can_manage_participants BOOLEAN DEFAULT FALSE,
    can_edit_event BOOLEAN DEFAULT FALSE,
    can_delete_event BOOLEAN DEFAULT FALSE,
    role_order INT DEFAULT 0
);

INSERT INTO event_roles (role_id, role_name, priority, can_manage_participants, can_edit_event, can_delete_event, role_order) VALUES
(1, 'Dungeon Master', 1, TRUE, TRUE, TRUE, 1),
(2, 'Master', 2, TRUE, TRUE, FALSE, 2),
(3, 'Slave', 3, FALSE, FALSE, FALSE, 3),
(4, 'Banned', 4, FALSE, FALSE, FALSE, 4)
-- Add migration script here
CREATE TABLE IF NOT EXISTS event_roles (
    role_id SMALLINT PRIMARY KEY,
    role_name VARCHAR(20) UNIQUE NOT NULL,
    description TEXT,
    priority SMALLINT NOT NULL,
    can_manage_participants BOOLEAN DEFAULT FALSE,
    can_edit_event BOOLEAN DEFAULT FALSE,
    can_delete_event BOOLEAN DEFAULT FALSE,
    role_order INT DEFAULT 0
);

INSERT INTO event_roles (role_id, role_name, description, priority, can_manage_participants, can_edit_event, can_delete_event, role_order) VALUES
(1, 'Dungeon Master', 'Owner', 1, TRUE, TRUE, TRUE, 1),
(2, 'Master', 'Admin', 2, TRUE, TRUE, FALSE, 2),
(3, 'Slave', 'Member', 3, FALSE, FALSE, FALSE, 3),
(4, 'Banned', 'Banned user', 4, FALSE, FALSE, FALSE, 4)
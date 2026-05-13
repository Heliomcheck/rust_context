-- Add migration script here
CREATE TABLE poll (
    poll_id BIGSERIAL PRIMARY KEY,
    event_id BIGINT NOT NULL REFERENCES events(event_id) ON DELETE CASCADE,
    question TEXT NOT NULL,
    created_by BIGINT NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    more_than_one_vote BOOLEAN NOT NULL
);

CREATE TABLE poll_option (
    option_id BIGSERIAL PRIMARY KEY,
    poll_id BIGINT NOT NULL REFERENCES poll(poll_id) ON DELETE CASCADE,
    option_text TEXT NOT NULL
);

CREATE TABLE poll_votes (
    poll_id BIGINT NOT NULL REFERENCES poll(poll_id) ON DELETE CASCADE,
    user_id BIGINT NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    option_id BIGINT NOT NULL REFERENCES poll_option(option_id) ON DELETE CASCADE,
    PRIMARY KEY (poll_id, option_id, user_id)
);

-- CREATE TABLE Hehe (
--     hehe_id int identity primary key, 
--     haha text not null,
--     is_hehe boolean not null,
--     is_haha boolean not null
--     time_hehe timestamp default now(),
--     time_haha timestamp default now(),
-- )
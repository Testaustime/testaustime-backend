CREATE TABLE leaderboards(
    id SERIAL PRIMARY KEY,
    name VARCHAR(32) NOT NULL UNIQUE,
    invite_code VARCHAR(32) NOT NULL UNIQUE,
    creation_time TIMESTAMP NOT NULL
);

CREATE TABLE leaderboard_members(
    id SERIAL PRIMARY KEY,
    leaderboard_id INTEGER NOT NULL,
    user_id INTEGER NOT NULL,
    admin BOOLEAN NOT NULL DEFAULT FALSE,
    FOREIGN KEY(leaderboard_id)
        REFERENCES leaderboards(id)
            ON DELETE CASCADE,
    FOREIGN KEY(user_id)
        REFERENCES registered_users(id)
            ON DELETE CASCADE,
    UNIQUE(leaderboard_id, user_id)
);

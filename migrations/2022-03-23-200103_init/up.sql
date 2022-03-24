CREATE TABLE registered_users(
    id SERIAL PRIMARY KEY,
    auth_token VARCHAR(32) NOT NULL UNIQUE,
    friend_code VARCHAR(24) NOT NULL UNIQUE,
    username VARCHAR(32) NOT NULL UNIQUE,
    password BYTEA NOT NULL,
    salt BYTEA NOT NULL,
    registration_time TIMESTAMP NOT NULL
);

CREATE TABLE friend_relations(
    id SERIAL PRIMARY KEY,
    lesser_id INTEGER NOT NULL,
    greater_id INTEGER NOT NULL,
    FOREIGN KEY(lesser_id)
        REFERENCES registered_users(id)
            ON DELETE CASCADE,
    FOREIGN KEY(greater_id)
        REFERENCES registered_users(id)
            ON DELETE CASCADE,
    UNIQUE(lesser_id, greater_id)
);

CREATE TABLE coding_activities(
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL,
    start_time TIMESTAMP NOT NULL,
    duration INTEGER NOT NULL,
    project_name VARCHAR(32),
    language VARCHAR(32),
    editor_name VARCHAR(32),
    hostname VARCHAR(32),
    FOREIGN KEY(user_id)
        REFERENCES registered_users(id)
            ON DELETE CASCADE
);

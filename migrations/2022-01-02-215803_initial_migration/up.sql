CREATE TABLE RegisteredUsers(
    id INTEGER NOT NULL UNIQUE,
    auth_token TEXT NOT NULL,
    user_name TEXT NOT NULL,
    discord_id BIGINT NOT NULL,
    registration_time DATETIME NOT NULL,
    PRIMARY KEY (id)
);

CREATE TABLE CodingActivities(
    id INTEGER NOT NULL UNIQUE,
    user_id INTEGER NOT NULL,
    start_time DATETIME NOT NULL,
    duration INTEGER NOT NULL,
    project_name TEXT,
    language TEXT,
    editor_name TEXT,
    hostname TEXT,
    PRIMARY KEY (id),
    FOREIGN KEY (user_id) REFERENCES RegisteredUsers(id)
);

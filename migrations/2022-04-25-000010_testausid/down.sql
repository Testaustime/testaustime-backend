ALTER TABLE testaustime_users
ADD COLUMN auth_token CHAR(32),
ADD COLUMN friend_code CHAR(24),
ADD COLUMN registration_time TIMESTAMP;

CREATE TABLE registered_users(
    id SERIAL PRIMARY KEY,
    username VARCHAR(32) NOT NULL,
    password BYTEA NOT NULL,
    salt BYTEA NOT NULL,
    auth_token CHAR(32) NOT NULL,
    friend_code CHAR(24) NOT NULL,
    registration_time TIMESTAMP NOT NULL
);

INSERT INTO registered_users(id, username, auth_token, friend_code, registration_time, password, salt)
SELECT
    u.id, u.username, u.auth_token, u.friend_code, u.registration_time, t.password, t.salt
FROM
    testaustime_users as t
JOIN
    user_identities as u
ON
    t.identity = u.id;

DROP TABLE testaustime_users cascade;

ALTER TABLE coding_activities
DROP CONSTRAINT coding_activities_user_id_fkey,
ADD CONSTRAINT coding_activities_user_id_fkey FOREIGN KEY (user_id) REFERENCES registered_users(id) ON DELETE CASCADE;

ALTER TABLE friend_relations
DROP CONSTRAINT friend_relations_greater_id_fkey,
ADD CONSTRAINT friend_relations_greater_id_fkey FOREIGN KEY (greater_id) REFERENCES registered_users(id) ON DELETE CASCADE,
DROP CONSTRAINT friend_relations_lesser_id_fkey,
ADD CONSTRAINT friend_relations_lesser_id_fkey FOREIGN KEY (lesser_id) REFERENCES registered_users(id) ON DELETE CASCADE;

ALTER TABLE leaderboard_members
DROP CONSTRAINT leaderboard_members_user_id_fkey,
ADD CONSTRAINT leaderboard_members_user_id_fkey FOREIGN KEY (user_id) REFERENCES registered_users(id) ON DELETE CASCADE;

DROP TABLE user_identities cascade;
DROP TABLE testausid_users;

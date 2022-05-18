CREATE TABLE user_identities (LIKE registered_users INCLUDING ALL);

INSERT INTO user_identities SELECT * FROM registered_users;

ALTER TABLE user_identities
DROP COLUMN password,
DROP COLUMN salt;

ALTER TABLE coding_activities
DROP CONSTRAINT coding_activities_user_id_fkey,
ADD CONSTRAINT coding_activities_user_id_fkey FOREIGN KEY (user_id) REFERENCES user_identities(id) ON DELETE CASCADE;

ALTER TABLE friend_relations
DROP CONSTRAINT friend_relations_greater_id_fkey,
ADD CONSTRAINT friend_relations_greater_id_fkey FOREIGN KEY (greater_id) REFERENCES user_identities(id) ON DELETE CASCADE,
DROP CONSTRAINT friend_relations_lesser_id_fkey,
ADD CONSTRAINT friend_relations_lesser_id_fkey FOREIGN KEY (lesser_id) REFERENCES user_identities(id) ON DELETE CASCADE;

ALTER TABLE leaderboard_members
DROP CONSTRAINT leaderboard_members_user_id_fkey,
ADD CONSTRAINT leaderboard_members_user_id_fkey FOREIGN KEY (user_id) REFERENCES user_identities(id) ON DELETE CASCADE;

ALTER TABLE registered_users
RENAME TO testaustime_users;

ALTER TABLE testaustime_users
DROP COLUMN auth_token,
DROP COLUMN friend_code,
DROP COLUMN registration_time,
DROP COLUMN username,
ADD COLUMN identity INT;

UPDATE testaustime_users
SET identity = id;

ALTER TABLE testaustime_users
ALTER COLUMN identity SET NOT NULL,
ADD CONSTRAINT testaustime_users_identity_fkey FOREIGN KEY (identity) REFERENCES user_identities(id) ON DELETE CASCADE;

CREATE TABLE testausid_users(
    id SERIAL PRIMARY KEY,
    user_id INT NOT NULL,
    service_id INT NOT NULL,
    identity INT NOT NULL,
    CONSTRAINT testausid_users_identity_fkey
        FOREIGN KEY (identity)
        REFERENCES user_identities(id)
);

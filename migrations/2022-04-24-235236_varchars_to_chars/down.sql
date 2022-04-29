ALTER TABLE registered_users
ALTER COLUMN auth_token TYPE VARCHAR(32);
ALTER TABLE registered_users
ALTER COLUMN friend_code TYPE VARCHAR(24);

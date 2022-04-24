ALTER TABLE registered_users
ALTER COLUMN auth_token TYPE CHAR(32);
ALTER TABLE registered_users
ALTER COLUMN friend_code TYPE CHAR(24);

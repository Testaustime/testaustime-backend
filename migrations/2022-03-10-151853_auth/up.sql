ALTER TABLE RegisteredUsers
ADD COLUMN password BINARY(32) NOT NULL AFTER user_name,
ADD COLUMN salt BINARY(22) NOT NULL AFTER password,
ADD CONSTRAINT unique_usernames UNIQUE (user_name);

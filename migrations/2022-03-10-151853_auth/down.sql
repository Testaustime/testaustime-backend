ALTER TABLE RegisteredUsers
DROP COLUMN password,
DROP COLUMN salt,
DROP INDEX unique_usernames;

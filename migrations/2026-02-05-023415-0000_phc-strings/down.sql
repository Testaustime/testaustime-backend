-- This file should undo anything in `up.sql`
ALTER TABLE testaustime_users 
ADD raw_password bytea,
ADD salt bytea;

UPDATE testaustime_users
SET salt = decode(SPLIT_PART(password, '$', 5), 'escape'),
raw_password = decode(RPAD(SPLIT_PART(password, '$', 6), 44, '='), 'base64');

ALTER TABLE testaustime_users
DROP COLUMN password;

ALTER TABLE testaustime_users
RENAME COLUMN raw_password to password;

ALTER TABLE testaustime_users 
RENAME COLUMN password TO raw_password;
ALTER TABLE testaustime_users 
ADD password TEXT;

UPDATE testaustime_users AS t
SET password = CONCAT(
    '$argon2id$v=19$m=4096,t=3,p=1$',
    convert_from(t.salt, 'UTF-8'),
    '$',
    TRIM(TRAILING '=' FROM encode(t.raw_password, 'base64'))
);

ALTER TABLE testaustime_users 
ALTER COLUMN password SET NOT NULL;

ALTER TABLE testaustime_users 
DROP COLUMN raw_password,
DROP COLUMN salt;

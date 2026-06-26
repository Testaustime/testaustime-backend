-- This file should undo anything in `up.sql`
ALTER TABLE user_identities
DROP COLUMN email VARCHAR UNIQUE;

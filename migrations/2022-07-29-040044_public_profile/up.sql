ALTER TABLE user_identities
ADD COLUMN is_public BOOLEAN NOT NULL DEFAULT false;

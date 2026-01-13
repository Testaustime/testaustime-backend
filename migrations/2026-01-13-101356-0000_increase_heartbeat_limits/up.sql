-- Your SQL goes here
ALTER TABLE coding_activities
    ALTER COLUMN language TYPE VARCHAR(64),
    ALTER COLUMN editor_name TYPE VARCHAR(64),
    ALTER COLUMN hostname TYPE VARCHAR(64);

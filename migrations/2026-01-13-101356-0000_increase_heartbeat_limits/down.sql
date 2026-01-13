-- This file should undo anything in `up.sql`
ALTER TABLE coding_activities
    ALTER COLUMN language TYPE VARCHAR(32),
    ALTER COLUMN editor_name TYPE VARCHAR(32),
    ALTER COLUMN hostname TYPE VARCHAR(32);

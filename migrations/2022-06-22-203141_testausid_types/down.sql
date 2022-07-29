ALTER TABLE testausid_users
ALTER COLUMN user_id TYPE INT USING user_id::integer,
ALTER COLUMN service_id TYPE INT USING service_id::integer;

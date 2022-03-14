ALTER TABLE FriendRelations
RENAME COLUMN lesser_id to adder,
RENAME COLUMN greater_id to friend;

DROP INDEX friends ON FriendRelations;

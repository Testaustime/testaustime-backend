ALTER TABLE FriendRelations
RENAME COLUMN adder TO lesser_id,
RENAME COLUMN friend TO greater_id;

CREATE UNIQUE INDEX friends ON FriendRelations(lesser_id,greater_id);

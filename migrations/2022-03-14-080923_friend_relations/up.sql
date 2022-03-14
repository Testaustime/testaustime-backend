CREATE TABLE FriendRelations (
    id INTEGER NOT NULL UNIQUE AUTO_INCREMENT,
    adder INTEGER NOT NULL,
    friend INTEGER NOT NULL,
    PRIMARY KEY (id),
    FOREIGN KEY (adder) REFERENCES RegisteredUsers(id),
    FOREIGN KEY (friend) REFERENCES RegisteredUsers(id)
);

ALTER TABLE RegisteredUsers
ADD COLUMN friend_code VARCHAR(24) UNIQUE AFTER user_name;

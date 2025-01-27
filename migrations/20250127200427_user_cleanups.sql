-- Add migration script here
CREATE TABLE user_cleanups
(
    guild_id   INT8 NOT NULL,
    user_id    INT8 NOT NULL,
    removed_at TIMESTAMP NOT NULL,
    PRIMARY KEY(user_id, guild_id)
);
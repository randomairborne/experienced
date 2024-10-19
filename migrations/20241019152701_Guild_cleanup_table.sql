-- Add migration script here
CREATE TABLE guild_cleanups
(
    guild      INT8 PRIMARY KEY,
    removed_at TIMESTAMP NOT NULL
);
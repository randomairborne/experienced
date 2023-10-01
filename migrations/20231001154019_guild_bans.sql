-- Add migration script here
CREATE TABLE guild_bans (
    id BIGINT NOT NULL,
    expires TIMESTAMP
);
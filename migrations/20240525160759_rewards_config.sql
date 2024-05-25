-- Add migration script here
CREATE TABLE guild_configs (
    id BIGINT PRIMARY KEY,
    one_at_a_time BOOLEAN
);
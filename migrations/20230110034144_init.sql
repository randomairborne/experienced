-- Add migration script here

CREATE TABLE levels (
    id BIGINT NOT NULL,
    xp BIGINT NOT NULL,
    guild BIGINT NOT NULL,
    PRIMARY KEY (id, guild)
);

CREATE TABLE custom_colors (
    important VARCHAR(7),
    secondary VARCHAR(7),
    rank VARCHAR(7),
    level VARCHAR(7),
    border VARCHAR(7),
    background VARCHAR(7),
    progress_foreground VARCHAR(7),
    progress_background VARCHAR(7),
    id BIGINT PRIMARY KEY
);

CREATE TABLE role_rewards (
    id BIGINT NOT NULL,
    requirement BIGINT NOT NULL,
    guild BIGINT NOT NULL,
    UNIQUE (guild, id),
    UNIQUE (guild, requirement)
);
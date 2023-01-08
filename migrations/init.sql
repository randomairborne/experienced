USE experienced;

CREATE TABLE IF NOT EXISTS levels (
    id BIGINT UNSIGNED NOT NULL,
    xp BIGINT UNSIGNED NOT NULL,
    guild BIGINT UNSIGNED NOT NULL,
    PRIMARY KEY (id, guild)
);

CREATE TABLE IF NOT EXISTS custom_colors (
    important VARCHAR(7),
    secondary VARCHAR(7),
    `rank` VARCHAR(7),
    level VARCHAR(7),
    border VARCHAR(7),
    background VARCHAR(7),
    progress_foreground VARCHAR(7),
    progress_background VARCHAR(7),
    id BIGINT UNSIGNED PRIMARY KEY
);

CREATE TABLE IF NOT EXISTS role_rewards (
    id BIGINT UNSIGNED NOT NULL,
    requirement BIGINT UNSIGNED NOT NULL,
    guild BIGINT UNSIGNED NOT NULL,
    UNIQUE (guild, id),
    UNIQUE (guild, requirement)
);

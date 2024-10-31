-- Add migration script here
CREATE TABLE cooldowns
(
    guild_id     INT8 NOT NULL,
    user_id      INT8 NOT NULL,
    last_message INT8 NOT NULL,
    PRIMARY KEY (guild_id, user_id)
);

CREATE INDEX ON cooldowns (last_message);
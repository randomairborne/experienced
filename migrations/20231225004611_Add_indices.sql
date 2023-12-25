-- Add migration script here

CREATE INDEX ON levels (guild);
CREATE INDEX ON levels (xp);

CREATE INDEX ON role_rewards USING HASH (guild);

CREATE INDEX ON guild_bans USING HASH (id);
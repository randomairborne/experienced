-- Add migration script here
CREATE TABLE audit_logs (
    guild_id INT8 NOT NULL,
    user_id INT8 NOT NULL,
    moderator INT8 NOT NULL,
    timestamp INT8 NOT NULL,
    previous INT8 NOT NULL,
    delta INT8 NOT NULL,
    reset BOOLEAN NOT NULL,
    set BOOLEAN NOT NULL
);

CREATE INDEX ON audit_logs(guild_id);
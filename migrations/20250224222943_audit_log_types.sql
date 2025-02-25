-- Add migration script here
CREATE TABLE new_audit_logs (
    guild INT8 NOT NULL,
    target INT8 NOT NULL,
    moderator INT8 NOT NULL,
    timestamp INT8 NOT NULL,
    previous INT8 NOT NULL,
    delta INT8 NOT NULL,
    kind INT8 NOT NULL
);

INSERT INTO new_audit_logs SELECT guild_id as guild, user_id as target, moderator, timestamp, previous, delta,
CASE
    /* set */
    WHEN set THEN 2
    /* reset */
    WHEN reset THEN 1
    /* add/sub */
    ELSE 0
END as kind
FROM audit_logs;

CREATE INDEX ON new_audit_logs(guild);
DROP TABLE audit_logs;
ALTER TABLE new_audit_logs RENAME TO audit_logs;

-- Add migration script here
ALTER TABLE guild_configs
    ADD COLUMN ping_on_level_up BOOLEAN;
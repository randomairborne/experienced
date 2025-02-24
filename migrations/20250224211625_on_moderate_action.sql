-- Add migration script here
ALTER TABLE guild_configs
    ADD COLUMN set_on_kick INT8;
ALTER TABLE guild_configs
    ADD COLUMN set_on_ban INT8;
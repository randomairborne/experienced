-- Add migration script here
ALTER TABLE guild_configs
    ADD COLUMN guild_card_default_show_off BOOLEAN NOT NULL DEFAULT FALSE;
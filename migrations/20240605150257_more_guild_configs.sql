-- Add migration script here
ALTER TABLE guild_configs
    ADD COLUMN level_up_message VARCHAR(512);
ALTER TABLE guild_configs
    ADD COLUMN level_up_channel INT8;
ALTER TABLE guild_configs
    ADD COLUMN min_xp_per_message INT2;
ALTER TABLE guild_configs
    ADD COLUMN max_xp_per_message INT2;
ALTER TABLE guild_configs
    ADD COLUMN message_cooldown INT2;
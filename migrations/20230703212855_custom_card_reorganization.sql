-- Add migration script here
ALTER TABLE custom_card RENAME COLUMN important TO username;
ALTER TABLE custom_card RENAME COLUMN secondary TO background_xp_count;
ALTER TABLE custom_card ADD COLUMN foreground_xp_count VARCHAR(7);
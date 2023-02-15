-- Add migration script here
ALTER TABLE custom_colors ADD COLUMN font VARCHAR(64);
ALTER TABLE custom_colors RENAME TO custom_card;
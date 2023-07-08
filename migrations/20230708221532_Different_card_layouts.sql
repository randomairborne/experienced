-- Add migration script here
ALTER TABLE custom_card ADD COLUMN card_layout VARCHAR(32) NOT NULL DEFAULT 'classic.svg';
-- Add down migration script here
ALTER TABLE addresses DROP COLUMN label;

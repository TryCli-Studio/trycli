-- Add down migration script here
ALTER TABLE projects DROP COLUMN IF EXISTS embed_token;
-- Add up migration script here
ALTER TABLE projects
ADD COLUMN is_public BOOLEAN NOT NULL DEFAULT FALSE;
-- Add down migration script here
ALTER TABLE projects
DROP COLUMN is_public;
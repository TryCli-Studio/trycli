-- Add up migration script here
ALTER TABLE projects ADD COLUMN IF NOT EXISTS view_count BIGINT NOT NULL DEFAULT 0;
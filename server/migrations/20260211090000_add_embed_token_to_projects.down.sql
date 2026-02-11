DROP INDEX IF EXISTS projects_embed_token_unique;

ALTER TABLE projects
DROP COLUMN IF EXISTS embed_token;

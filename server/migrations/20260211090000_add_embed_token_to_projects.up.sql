ALTER TABLE projects
ADD COLUMN IF NOT EXISTS embed_token text;

CREATE UNIQUE INDEX IF NOT EXISTS projects_embed_token_unique
ON projects (embed_token);

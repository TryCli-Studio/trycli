ALTER TABLE projects 
DROP COLUMN IF EXISTS is_protected,
DROP COLUMN IF EXISTS allowed_origins,
DROP COLUMN IF EXISTS embed_token;

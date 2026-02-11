-- 1. Add the secret VIP Pass key to projects
ALTER TABLE projects 
ADD COLUMN IF NOT EXISTS embed_key TEXT UNIQUE;

-- 2. Backfill existing projects with a unique secure key
-- Using pgcrypto which you already enabled in a previous migration
UPDATE projects 
SET embed_key = encode(gen_random_bytes(24), 'base64') 
WHERE embed_key IS NULL;

-- 3. Create the Whitelist (Guest List) table
CREATE TABLE IF NOT EXISTS project_whitelists (
    id SERIAL PRIMARY KEY,
    project_id BIGINT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    allowed_url TEXT NOT NULL CHECK (length(allowed_url) <= 2048),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(project_id, allowed_url)
);

-- Index for high-performance Referer lookups
CREATE INDEX IF NOT EXISTS idx_whitelist_lookup ON project_whitelists(project_id, allowed_url);
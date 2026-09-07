-- 1. Enable pgcrypto (Safe: checks existence)
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- 2. Add column (Safe: checks existence)
ALTER TABLE projects ADD COLUMN IF NOT EXISTS embed_token TEXT;

-- 3. Backfill Data 
-- (Safe: only updates rows that are currently NULL)
UPDATE projects 
SET embed_token = gen_random_uuid()::text 
WHERE embed_token IS NULL;

-- 4. Enforce Non-Null (Safe: idempotent)
ALTER TABLE projects ALTER COLUMN embed_token SET NOT NULL;

-- 5. Add Unique Constraint (Safe: Wrapped in a DO block to check existence)
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'unique_embed_token') THEN
        ALTER TABLE projects ADD CONSTRAINT unique_embed_token UNIQUE (embed_token);
    END IF;
END $$;
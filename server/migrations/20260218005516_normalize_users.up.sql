-- 1. Create the new users table
CREATE TABLE IF NOT EXISTS users (
    id BIGINT PRIMARY KEY, 
    username TEXT NOT NULL,
    avatar_url TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_login TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 2. Backfill user data (Safe: handles conflicts if data exists)
INSERT INTO users (id, username, last_login)
SELECT DISTINCT owner_id, owner_username, NOW()
FROM projects
WHERE owner_id IS NOT NULL AND owner_username IS NOT NULL
ON CONFLICT (id) DO UPDATE 
SET username = EXCLUDED.username;

-- 3. Enforce Foreign Key Constraint (Safe: drops first if exists)
ALTER TABLE projects DROP CONSTRAINT IF EXISTS fk_projects_owner;
ALTER TABLE projects 
ADD CONSTRAINT fk_projects_owner 
FOREIGN KEY (owner_id) REFERENCES users(id) ON DELETE CASCADE;

-- 4. Drop the redundant column (Safe: checks existence)
ALTER TABLE projects DROP COLUMN IF EXISTS owner_username;

-- 5. Fix Primary Keys
-- FIX: Use IF EXISTS to prevent "constraint does not exist" errors
ALTER TABLE projects DROP CONSTRAINT IF EXISTS projects_pkey;

-- Drop the unique constraint if it exists (so we can re-add it cleanly)
ALTER TABLE projects DROP CONSTRAINT IF EXISTS projects_owner_slug_unique;

-- Add the logical unique constraint for projects
ALTER TABLE projects ADD CONSTRAINT projects_owner_slug_unique UNIQUE (owner_id, slug);

-- Set 'id' as the new Primary Key (safely)
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'projects_pkey') THEN
        ALTER TABLE projects ADD PRIMARY KEY (id);
    END IF;
END $$;
-- Add the column back
ALTER TABLE projects ADD COLUMN IF NOT EXISTS owner_username TEXT;

-- Restore data from users table
UPDATE projects p
SET owner_username = u.username
FROM users u
WHERE p.owner_id = u.id;

-- Drop the new constraints
ALTER TABLE projects DROP CONSTRAINT IF EXISTS fk_projects_owner;
ALTER TABLE projects DROP CONSTRAINT IF EXISTS projects_owner_slug_unique;

-- Restore the old Primary Key (Composite)
-- First drop the ID primary key if it exists
ALTER TABLE projects DROP CONSTRAINT IF EXISTS projects_pkey;
-- Add the old composite primary key
ALTER TABLE projects ADD PRIMARY KEY (owner_username, slug);

-- Drop users table
DROP TABLE IF EXISTS users;
-- Add analytics events table and project IDs
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'analytics_event_type') THEN
        CREATE TYPE analytics_event_type AS ENUM ('view', 'session_end', 'error');
    END IF;
END$$;

ALTER TABLE projects ADD COLUMN IF NOT EXISTS id BIGSERIAL;
UPDATE projects
SET id = nextval(pg_get_serial_sequence('projects', 'id'))
WHERE id IS NULL;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'projects_id_unique'
    ) THEN
        ALTER TABLE projects ADD CONSTRAINT projects_id_unique UNIQUE (id);
    END IF;
END$$;

CREATE TABLE IF NOT EXISTS analytics_events (
    id BIGSERIAL PRIMARY KEY,
    project_id BIGINT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    event_type analytics_event_type NOT NULL,
    duration_seconds BIGINT,
    error_type TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS analytics_events_project_id_idx ON analytics_events(project_id);
CREATE INDEX IF NOT EXISTS analytics_events_created_at_idx ON analytics_events(created_at);
CREATE INDEX IF NOT EXISTS analytics_events_event_type_idx ON analytics_events(event_type);

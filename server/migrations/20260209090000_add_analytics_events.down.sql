DROP TABLE IF EXISTS analytics_events;
DROP TYPE IF EXISTS analytics_event_type;

DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'projects_id_unique'
    ) THEN
        ALTER TABLE projects DROP CONSTRAINT projects_id_unique;
    END IF;
END$$;

ALTER TABLE projects DROP COLUMN IF EXISTS id;

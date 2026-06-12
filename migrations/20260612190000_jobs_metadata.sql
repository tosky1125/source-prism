ALTER TABLE jobs
    ADD COLUMN metadata JSONB NOT NULL DEFAULT '{}'::jsonb;

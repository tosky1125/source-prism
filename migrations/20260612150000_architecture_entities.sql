CREATE TABLE architecture_entities (
    architecture_entity_id TEXT PRIMARY KEY,
    stable_entity_id TEXT NOT NULL,
    repo_id TEXT NOT NULL,
    commit_sha TEXT NOT NULL,
    generation_id TEXT NOT NULL REFERENCES index_generations (generation_id) ON DELETE RESTRICT,
    entity_type TEXT NOT NULL,
    name TEXT NOT NULL,
    file_path TEXT NOT NULL,
    start_line INTEGER NOT NULL,
    end_line INTEGER NOT NULL,
    content_hash TEXT NOT NULL,
    confidence TEXT NOT NULL DEFAULT 'high',
    created_by TEXT NOT NULL DEFAULT 'ri-architecture',
    evidence JSONB NOT NULL DEFAULT '{}'::jsonb,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    stale_at TIMESTAMPTZ,
    FOREIGN KEY (repo_id, commit_sha)
        REFERENCES commits (repo_id, commit_sha)
        ON DELETE RESTRICT,
    CHECK (start_line >= 1),
    CHECK (end_line >= start_line),
    CHECK (confidence IN ('exact', 'high', 'medium', 'low'))
);

CREATE UNIQUE INDEX architecture_entities_active_identity_idx
    ON architecture_entities (repo_id, commit_sha, entity_type, file_path)
    WHERE stale_at IS NULL;

CREATE INDEX architecture_entities_repo_type_idx
    ON architecture_entities (repo_id, entity_type)
    WHERE stale_at IS NULL;

CREATE INDEX architecture_entities_generation_idx
    ON architecture_entities (generation_id);

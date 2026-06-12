CREATE TABLE file_overlays (
    file_overlay_id TEXT PRIMARY KEY,
    repo_id TEXT NOT NULL,
    base_generation_id TEXT NOT NULL REFERENCES index_generations (generation_id) ON DELETE CASCADE,
    base_commit_sha TEXT NOT NULL,
    head_sha TEXT NOT NULL,
    file_path TEXT NOT NULL,
    previous_file_path TEXT,
    status TEXT NOT NULL,
    language TEXT NOT NULL DEFAULT 'unknown',
    content_sha256 TEXT,
    size_bytes BIGINT,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (status IN ('added', 'modified', 'deleted', 'renamed', 'mode_only')),
    CHECK (size_bytes IS NULL OR size_bytes >= 0)
);

CREATE UNIQUE INDEX file_overlays_identity_idx
    ON file_overlays (repo_id, base_generation_id, head_sha, file_path);

CREATE INDEX file_overlays_repo_base_idx
    ON file_overlays (repo_id, base_generation_id, head_sha);

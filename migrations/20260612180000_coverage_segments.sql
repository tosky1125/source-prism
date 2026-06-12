CREATE TABLE coverage_segments (
    coverage_segment_id TEXT PRIMARY KEY,
    repo_id TEXT NOT NULL,
    commit_sha TEXT NOT NULL,
    generation_id TEXT NOT NULL REFERENCES index_generations (generation_id) ON DELETE RESTRICT,
    source_path TEXT NOT NULL,
    file_path TEXT NOT NULL,
    start_line INTEGER NOT NULL,
    end_line INTEGER NOT NULL,
    hit_count INTEGER NOT NULL,
    format TEXT NOT NULL,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    stale_at TIMESTAMPTZ,
    FOREIGN KEY (repo_id, commit_sha)
        REFERENCES commits (repo_id, commit_sha)
        ON DELETE RESTRICT,
    CHECK (start_line >= 1),
    CHECK (end_line >= start_line),
    CHECK (hit_count >= 0)
);

CREATE UNIQUE INDEX coverage_segments_active_identity_idx
    ON coverage_segments (repo_id, commit_sha, source_path, file_path, start_line, end_line)
    WHERE stale_at IS NULL;

CREATE INDEX coverage_segments_repo_file_idx
    ON coverage_segments (repo_id, file_path)
    WHERE stale_at IS NULL;

CREATE INDEX coverage_segments_generation_idx
    ON coverage_segments (generation_id);

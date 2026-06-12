CREATE TABLE test_runs (
    test_run_id TEXT PRIMARY KEY,
    repo_id TEXT NOT NULL,
    commit_sha TEXT NOT NULL,
    generation_id TEXT NOT NULL REFERENCES index_generations (generation_id) ON DELETE RESTRICT,
    source_path TEXT NOT NULL,
    framework TEXT NOT NULL,
    status TEXT NOT NULL,
    total_count INTEGER NOT NULL,
    passed_count INTEGER NOT NULL,
    failed_count INTEGER NOT NULL,
    error_count INTEGER NOT NULL,
    skipped_count INTEGER NOT NULL,
    duration_ms BIGINT,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    stale_at TIMESTAMPTZ,
    FOREIGN KEY (repo_id, commit_sha)
        REFERENCES commits (repo_id, commit_sha)
        ON DELETE RESTRICT,
    CHECK (status IN ('passed', 'failed', 'error', 'skipped')),
    CHECK (total_count >= 0),
    CHECK (passed_count >= 0),
    CHECK (failed_count >= 0),
    CHECK (error_count >= 0),
    CHECK (skipped_count >= 0),
    CHECK (duration_ms IS NULL OR duration_ms >= 0)
);

CREATE UNIQUE INDEX test_runs_active_source_idx
    ON test_runs (repo_id, commit_sha, source_path)
    WHERE stale_at IS NULL;

CREATE INDEX test_runs_repo_commit_idx
    ON test_runs (repo_id, commit_sha)
    WHERE stale_at IS NULL;

CREATE INDEX test_runs_generation_idx
    ON test_runs (generation_id);

CREATE TABLE test_results (
    test_result_id TEXT PRIMARY KEY,
    test_run_id TEXT NOT NULL REFERENCES test_runs (test_run_id) ON DELETE RESTRICT,
    repo_id TEXT NOT NULL,
    commit_sha TEXT NOT NULL,
    generation_id TEXT NOT NULL REFERENCES index_generations (generation_id) ON DELETE RESTRICT,
    suite_name TEXT NOT NULL,
    class_name TEXT,
    name TEXT NOT NULL,
    fqn TEXT NOT NULL,
    file_path TEXT,
    status TEXT NOT NULL,
    duration_ms BIGINT,
    message TEXT,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    stale_at TIMESTAMPTZ,
    FOREIGN KEY (repo_id, commit_sha)
        REFERENCES commits (repo_id, commit_sha)
        ON DELETE RESTRICT,
    CHECK (status IN ('passed', 'failed', 'error', 'skipped')),
    CHECK (duration_ms IS NULL OR duration_ms >= 0)
);

CREATE UNIQUE INDEX test_results_active_identity_idx
    ON test_results (test_run_id, suite_name, fqn)
    WHERE stale_at IS NULL;

CREATE INDEX test_results_repo_status_idx
    ON test_results (repo_id, status)
    WHERE stale_at IS NULL;

CREATE INDEX test_results_generation_idx
    ON test_results (generation_id);

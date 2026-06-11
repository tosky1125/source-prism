CREATE TABLE test_cases (
    test_case_id TEXT PRIMARY KEY,
    stable_test_id TEXT NOT NULL,
    symbol_id TEXT REFERENCES symbols (symbol_id) ON DELETE RESTRICT,
    repo_id TEXT NOT NULL,
    commit_sha TEXT NOT NULL,
    generation_id TEXT NOT NULL REFERENCES index_generations (generation_id) ON DELETE RESTRICT,
    file_path TEXT NOT NULL,
    language TEXT NOT NULL,
    name TEXT NOT NULL,
    fqn TEXT NOT NULL,
    start_line INTEGER NOT NULL,
    start_col INTEGER NOT NULL,
    end_line INTEGER NOT NULL,
    end_col INTEGER NOT NULL,
    framework TEXT,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    stale_at TIMESTAMPTZ,
    FOREIGN KEY (repo_id, commit_sha)
        REFERENCES commits (repo_id, commit_sha)
        ON DELETE RESTRICT,
    CHECK (start_line >= 1),
    CHECK (end_line >= start_line),
    CHECK (start_col >= 0),
    CHECK (end_col >= 0)
);

CREATE UNIQUE INDEX test_cases_active_identity_idx
    ON test_cases (repo_id, commit_sha, file_path, fqn, start_line)
    WHERE stale_at IS NULL;

CREATE INDEX test_cases_repo_commit_idx
    ON test_cases (repo_id, commit_sha)
    WHERE stale_at IS NULL;

CREATE INDEX test_cases_generation_idx
    ON test_cases (generation_id);

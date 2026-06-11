CREATE TABLE repos (
    repo_id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    origin_url TEXT,
    default_branch TEXT,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX repos_origin_url_idx
    ON repos (origin_url)
    WHERE origin_url IS NOT NULL;

CREATE TABLE commits (
    repo_id TEXT NOT NULL REFERENCES repos (repo_id) ON DELETE RESTRICT,
    commit_sha TEXT NOT NULL,
    parent_shas TEXT[] NOT NULL DEFAULT ARRAY[]::TEXT[],
    tree_sha TEXT,
    author_time TIMESTAMPTZ,
    committer_time TIMESTAMPTZ,
    message_hash TEXT,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (repo_id, commit_sha)
);

CREATE INDEX commits_repo_created_idx
    ON commits (repo_id, created_at DESC);

CREATE TABLE index_generations (
    generation_id TEXT PRIMARY KEY,
    repo_id TEXT NOT NULL,
    commit_sha TEXT NOT NULL,
    index_kind TEXT NOT NULL,
    status TEXT NOT NULL,
    extractor_version TEXT,
    started_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    finished_at TIMESTAMPTZ,
    failed_at TIMESTAMPTZ,
    error TEXT,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    FOREIGN KEY (repo_id, commit_sha)
        REFERENCES commits (repo_id, commit_sha)
        ON DELETE RESTRICT,
    CHECK (status IN ('started', 'succeeded', 'failed', 'cancelled')),
    CHECK (finished_at IS NULL OR finished_at >= started_at),
    CHECK (failed_at IS NULL OR failed_at >= started_at)
);

CREATE INDEX index_generations_repo_commit_started_idx
    ON index_generations (repo_id, commit_sha, started_at DESC);

CREATE INDEX index_generations_status_started_idx
    ON index_generations (status, started_at);

CREATE TABLE file_manifests (
    file_manifest_id TEXT PRIMARY KEY,
    repo_id TEXT NOT NULL,
    commit_sha TEXT NOT NULL,
    generation_id TEXT NOT NULL REFERENCES index_generations (generation_id) ON DELETE RESTRICT,
    file_path TEXT NOT NULL,
    language TEXT NOT NULL DEFAULT 'unknown',
    content_sha256 TEXT NOT NULL,
    size_bytes BIGINT NOT NULL,
    mode TEXT,
    is_binary BOOLEAN NOT NULL DEFAULT false,
    is_generated BOOLEAN NOT NULL DEFAULT false,
    is_vendor BOOLEAN NOT NULL DEFAULT false,
    is_test BOOLEAN NOT NULL DEFAULT false,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    stale_at TIMESTAMPTZ,
    FOREIGN KEY (repo_id, commit_sha)
        REFERENCES commits (repo_id, commit_sha)
        ON DELETE RESTRICT,
    CHECK (size_bytes >= 0)
);

CREATE UNIQUE INDEX file_manifests_active_path_idx
    ON file_manifests (repo_id, commit_sha, file_path)
    WHERE stale_at IS NULL;

CREATE INDEX file_manifests_repo_commit_idx
    ON file_manifests (repo_id, commit_sha);

CREATE INDEX file_manifests_generation_idx
    ON file_manifests (generation_id);

CREATE TABLE symbols (
    symbol_id TEXT PRIMARY KEY,
    stable_symbol_id TEXT NOT NULL,
    repo_id TEXT NOT NULL,
    commit_sha TEXT NOT NULL,
    generation_id TEXT NOT NULL REFERENCES index_generations (generation_id) ON DELETE RESTRICT,
    file_manifest_id TEXT REFERENCES file_manifests (file_manifest_id) ON DELETE RESTRICT,
    file_path TEXT NOT NULL,
    language TEXT NOT NULL,
    kind TEXT NOT NULL,
    name TEXT NOT NULL,
    fqn TEXT,
    signature TEXT,
    signature_hash TEXT,
    start_line INTEGER NOT NULL,
    start_col INTEGER NOT NULL,
    end_line INTEGER NOT NULL,
    end_col INTEGER NOT NULL,
    parent_symbol_id TEXT REFERENCES symbols (symbol_id) ON DELETE RESTRICT,
    visibility TEXT,
    content_hash TEXT NOT NULL,
    confidence TEXT NOT NULL DEFAULT 'medium',
    doc_comment TEXT,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    stale_at TIMESTAMPTZ,
    FOREIGN KEY (repo_id, commit_sha)
        REFERENCES commits (repo_id, commit_sha)
        ON DELETE RESTRICT,
    CHECK (start_line >= 1),
    CHECK (end_line >= start_line),
    CHECK (start_col >= 0),
    CHECK (end_col >= 0),
    CHECK (confidence IN ('exact', 'high', 'medium', 'low'))
);

CREATE UNIQUE INDEX symbols_active_identity_idx
    ON symbols (repo_id, commit_sha, file_path, kind, fqn, start_line)
    WHERE stale_at IS NULL;

CREATE INDEX symbols_lookup_line_idx
    ON symbols (repo_id, commit_sha, file_path, start_line, end_line)
    WHERE stale_at IS NULL;

CREATE INDEX symbols_stable_idx
    ON symbols (repo_id, stable_symbol_id);

CREATE INDEX symbols_generation_idx
    ON symbols (generation_id);

CREATE TABLE graph_nodes (
    graph_node_id TEXT PRIMARY KEY,
    repo_id TEXT NOT NULL,
    commit_sha TEXT NOT NULL,
    generation_id TEXT NOT NULL REFERENCES index_generations (generation_id) ON DELETE RESTRICT,
    node_type TEXT NOT NULL,
    subject_id TEXT,
    stable_subject_id TEXT,
    display_name TEXT NOT NULL,
    file_path TEXT,
    start_line INTEGER,
    end_line INTEGER,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    stale_at TIMESTAMPTZ,
    FOREIGN KEY (repo_id, commit_sha)
        REFERENCES commits (repo_id, commit_sha)
        ON DELETE RESTRICT,
    CHECK (start_line IS NULL OR start_line >= 1),
    CHECK (end_line IS NULL OR end_line >= start_line)
);

CREATE UNIQUE INDEX graph_nodes_active_subject_idx
    ON graph_nodes (repo_id, commit_sha, node_type, subject_id)
    WHERE stale_at IS NULL AND subject_id IS NOT NULL;

CREATE INDEX graph_nodes_repo_commit_idx
    ON graph_nodes (repo_id, commit_sha)
    WHERE stale_at IS NULL;

CREATE INDEX graph_nodes_generation_idx
    ON graph_nodes (generation_id);

CREATE TABLE graph_edges (
    edge_id TEXT PRIMARY KEY,
    repo_id TEXT NOT NULL,
    commit_sha TEXT NOT NULL,
    generation_id TEXT NOT NULL REFERENCES index_generations (generation_id) ON DELETE RESTRICT,
    source_node_id TEXT NOT NULL REFERENCES graph_nodes (graph_node_id) ON DELETE RESTRICT,
    target_node_id TEXT NOT NULL REFERENCES graph_nodes (graph_node_id) ON DELETE RESTRICT,
    edge_type TEXT NOT NULL,
    confidence NUMERIC(4, 3) NOT NULL DEFAULT 1.0,
    resolution_method TEXT NOT NULL,
    evidence_file_path TEXT,
    evidence_start_line INTEGER,
    evidence_start_col INTEGER,
    evidence_end_line INTEGER,
    evidence_end_col INTEGER,
    evidence JSONB NOT NULL DEFAULT '{}'::jsonb,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    stale_at TIMESTAMPTZ,
    FOREIGN KEY (repo_id, commit_sha)
        REFERENCES commits (repo_id, commit_sha)
        ON DELETE RESTRICT,
    CHECK (confidence >= 0.0 AND confidence <= 1.0),
    CHECK (evidence_start_line IS NULL OR evidence_start_line >= 1),
    CHECK (evidence_end_line IS NULL OR evidence_end_line >= evidence_start_line)
);

CREATE INDEX graph_edges_source_idx
    ON graph_edges (repo_id, commit_sha, source_node_id, edge_type)
    WHERE stale_at IS NULL;

CREATE INDEX graph_edges_target_idx
    ON graph_edges (repo_id, commit_sha, target_node_id, edge_type)
    WHERE stale_at IS NULL;

CREATE INDEX graph_edges_type_idx
    ON graph_edges (repo_id, commit_sha, edge_type)
    WHERE stale_at IS NULL;

CREATE INDEX graph_edges_generation_idx
    ON graph_edges (generation_id);

CREATE TABLE jobs (
    job_id TEXT PRIMARY KEY,
    queue TEXT NOT NULL,
    kind TEXT NOT NULL,
    state TEXT NOT NULL,
    idempotency_key TEXT,
    generation_id TEXT REFERENCES index_generations (generation_id) ON DELETE RESTRICT,
    payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    priority INTEGER NOT NULL DEFAULT 0,
    run_after TIMESTAMPTZ NOT NULL DEFAULT now(),
    attempt_count INTEGER NOT NULL DEFAULT 0,
    max_attempts INTEGER NOT NULL DEFAULT 3,
    leased_by TEXT,
    leased_until TIMESTAMPTZ,
    last_error TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    completed_at TIMESTAMPTZ,
    CHECK (state IN ('queued', 'leased', 'succeeded', 'failed', 'dead_lettered', 'cancelled')),
    CHECK (attempt_count >= 0),
    CHECK (max_attempts > 0),
    CHECK (priority >= 0),
    CHECK (
        (state = 'leased' AND leased_by IS NOT NULL AND leased_until IS NOT NULL)
        OR (state <> 'leased')
    )
);

CREATE UNIQUE INDEX jobs_idempotency_idx
    ON jobs (queue, kind, idempotency_key)
    WHERE idempotency_key IS NOT NULL;

CREATE INDEX jobs_ready_lease_idx
    ON jobs (queue, priority DESC, run_after, created_at)
    WHERE state = 'queued';

CREATE INDEX jobs_lease_recovery_idx
    ON jobs (leased_until)
    WHERE state = 'leased';

CREATE INDEX jobs_generation_idx
    ON jobs (generation_id)
    WHERE generation_id IS NOT NULL;

CREATE TABLE job_attempts (
    job_attempt_id BIGSERIAL PRIMARY KEY,
    job_id TEXT NOT NULL REFERENCES jobs (job_id) ON DELETE RESTRICT,
    attempt_no INTEGER NOT NULL,
    worker_id TEXT NOT NULL,
    started_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    finished_at TIMESTAMPTZ,
    status TEXT NOT NULL,
    error TEXT,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    CHECK (attempt_no > 0),
    CHECK (status IN ('started', 'succeeded', 'failed', 'cancelled')),
    CHECK (finished_at IS NULL OR finished_at >= started_at),
    UNIQUE (job_id, attempt_no)
);

CREATE INDEX job_attempts_job_idx
    ON job_attempts (job_id);

CREATE TABLE search_sync_outbox (
    outbox_id TEXT PRIMARY KEY,
    repo_id TEXT NOT NULL REFERENCES repos (repo_id) ON DELETE RESTRICT,
    generation_id TEXT REFERENCES index_generations (generation_id) ON DELETE RESTRICT,
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    operation TEXT NOT NULL,
    target_index TEXT NOT NULL,
    payload_hash TEXT NOT NULL,
    payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    state TEXT NOT NULL,
    attempt_count INTEGER NOT NULL DEFAULT 0,
    max_attempts INTEGER NOT NULL DEFAULT 3,
    run_after TIMESTAMPTZ NOT NULL DEFAULT now(),
    leased_by TEXT,
    leased_until TIMESTAMPTZ,
    processed_at TIMESTAMPTZ,
    last_error TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (operation IN ('upsert', 'delete')),
    CHECK (state IN ('queued', 'leased', 'succeeded', 'failed', 'dead_lettered', 'cancelled')),
    CHECK (attempt_count >= 0),
    CHECK (max_attempts > 0),
    CHECK (
        (state = 'leased' AND leased_by IS NOT NULL AND leased_until IS NOT NULL)
        OR (state <> 'leased')
    )
);

CREATE UNIQUE INDEX search_sync_outbox_entity_operation_idx
    ON search_sync_outbox (target_index, entity_type, entity_id, operation, payload_hash);

CREATE INDEX search_sync_outbox_ready_lease_idx
    ON search_sync_outbox (target_index, run_after, created_at)
    WHERE state = 'queued';

CREATE INDEX search_sync_outbox_lease_recovery_idx
    ON search_sync_outbox (leased_until)
    WHERE state = 'leased';

CREATE INDEX search_sync_outbox_generation_idx
    ON search_sync_outbox (generation_id)
    WHERE generation_id IS NOT NULL;

CREATE TABLE embedding_cache (
    cache_key TEXT PRIMARY KEY,
    provider TEXT NOT NULL,
    model TEXT NOT NULL,
    input_sha256 TEXT NOT NULL,
    input_kind TEXT NOT NULL,
    dimensions INTEGER NOT NULL,
    embedding_f32 BYTEA NOT NULL,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_accessed_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (dimensions > 0)
);

CREATE UNIQUE INDEX embedding_cache_model_input_idx
    ON embedding_cache (provider, model, input_sha256, dimensions);

CREATE INDEX embedding_cache_input_sha256_idx
    ON embedding_cache (input_sha256);

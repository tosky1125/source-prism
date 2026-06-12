use axum::{
    Json,
    extract::{Path, State},
};
use serde::Serialize;
use sqlx::{PgPool, Row as _};

use crate::{
    AppError,
    local_index::{LocalIndexSummary, local_index_summary},
    run_jobs::{RunSearchSyncJob, find_search_sync_jobs},
    run_outbox::{
        RunSearchSyncOutboxItem, RunSearchSyncOutboxStateCounts, count_search_sync_outbox_states,
        find_search_sync_outbox,
    },
    state::AppState,
};

#[derive(Debug, Serialize)]
pub(crate) struct RepoRunsResponse {
    status: &'static str,
    kind: &'static str,
    repo_id: String,
    run_count: usize,
    runs: Vec<RepoRunSummary>,
}

#[derive(Debug, Serialize)]
pub(crate) struct RepoRunSummary {
    run_id: String,
    commit_sha: String,
    index_kind: String,
    status: String,
    started_at: String,
    finished_at: Option<String>,
    evidence: RepoRunEvidence,
}

#[derive(Debug, Serialize)]
pub(crate) struct RepoRunEvidence {
    file_manifests: i64,
    symbols: i64,
    graph_edges: i64,
    search_chunks: i64,
    search_sync_outbox_details: Vec<RunSearchSyncOutboxItem>,
    search_sync_outbox_state_counts: RunSearchSyncOutboxStateCounts,
    search_sync_jobs: i64,
    search_sync_job_details: Vec<RunSearchSyncJob>,
    test_cases: i64,
}

pub(crate) async fn list(
    State(state): State<AppState>,
    Path(repo_id): Path<String>,
) -> Result<Json<RepoRunsResponse>, AppError> {
    let runs = if let Some(pool) = state.database.pool.as_ref() {
        find_repo_runs(pool, &repo_id).await?
    } else {
        vec![local_run_summary(local_index_summary(&state, &repo_id)?)]
    };
    Ok(Json(RepoRunsResponse {
        status: "ok",
        kind: "repo_runs",
        repo_id,
        run_count: runs.len(),
        runs,
    }))
}

fn local_run_summary(local: LocalIndexSummary) -> RepoRunSummary {
    RepoRunSummary {
        run_id: local.run_id,
        commit_sha: local.commit_sha,
        index_kind: "local_worktree".to_owned(),
        status: "succeeded".to_owned(),
        started_at: local.started_at,
        finished_at: local.finished_at,
        evidence: RepoRunEvidence {
            file_manifests: local.file_manifests,
            symbols: local.symbols,
            graph_edges: local.graph_edges,
            search_chunks: local.search_chunks,
            search_sync_outbox_details: Vec::new(),
            search_sync_outbox_state_counts: empty_outbox_counts(),
            search_sync_jobs: 0,
            search_sync_job_details: Vec::new(),
            test_cases: local.test_cases,
        },
    }
}

const fn empty_outbox_counts() -> RunSearchSyncOutboxStateCounts {
    RunSearchSyncOutboxStateCounts {
        queued: 0,
        leased: 0,
        succeeded: 0,
        failed: 0,
        dead_lettered: 0,
        cancelled: 0,
        total: 0,
    }
}

async fn find_repo_runs(pool: &PgPool, repo_id: &str) -> Result<Vec<RepoRunSummary>, sqlx::Error> {
    let rows = sqlx::query(
        r"
        SELECT
            g.generation_id,
            g.commit_sha,
            g.index_kind,
            g.status,
            g.started_at::text AS started_at,
            g.finished_at::text AS finished_at,
            (
                SELECT count(*)::bigint FROM file_manifests AS item
                WHERE item.generation_id = g.generation_id
            ) AS file_manifest_count,
            (
                SELECT count(*)::bigint FROM symbols AS item
                WHERE item.generation_id = g.generation_id
            ) AS symbol_count,
            (
                SELECT count(*)::bigint FROM graph_edges AS item
                WHERE item.generation_id = g.generation_id
            ) AS graph_edge_count,
            (
                SELECT count(*)::bigint FROM search_sync_outbox AS item
                WHERE item.generation_id = g.generation_id
            ) AS search_chunk_count,
            (
                SELECT count(*)::bigint FROM jobs AS item
                WHERE item.generation_id = g.generation_id
                  AND item.kind = 'search.sync_once'
            ) AS search_sync_job_count,
            (
                SELECT count(*)::bigint FROM test_cases AS item
                WHERE item.generation_id = g.generation_id
            ) AS test_case_count
        FROM index_generations AS g
        WHERE g.repo_id = $1
        ORDER BY g.started_at DESC
        LIMIT 20
        ",
    )
    .bind(repo_id)
    .fetch_all(pool)
    .await?;

    let mut runs = Vec::with_capacity(rows.len());
    for row in rows {
        let run_id = row.try_get::<String, _>("generation_id")?;
        let search_sync_job_details = find_search_sync_jobs(pool, &run_id).await?;
        let search_sync_outbox_details = find_search_sync_outbox(pool, &run_id).await?;
        let search_sync_outbox_state_counts =
            count_search_sync_outbox_states(pool, &run_id).await?;
        runs.push(RepoRunSummary {
            run_id,
            commit_sha: row.try_get("commit_sha")?,
            index_kind: row.try_get("index_kind")?,
            status: row.try_get("status")?,
            started_at: row.try_get("started_at")?,
            finished_at: row.try_get("finished_at")?,
            evidence: RepoRunEvidence {
                file_manifests: row.try_get("file_manifest_count")?,
                symbols: row.try_get("symbol_count")?,
                graph_edges: row.try_get("graph_edge_count")?,
                search_chunks: row.try_get("search_chunk_count")?,
                search_sync_outbox_details,
                search_sync_outbox_state_counts,
                search_sync_jobs: row.try_get("search_sync_job_count")?,
                search_sync_job_details,
                test_cases: row.try_get("test_case_count")?,
            },
        });
    }
    Ok(runs)
}

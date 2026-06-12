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
pub(crate) struct RunResponse {
    status: &'static str,
    kind: &'static str,
    run: RunSummary,
}

#[derive(Debug, Serialize)]
pub(crate) struct RunSummary {
    run_id: String,
    repo_id: String,
    commit_sha: String,
    index_kind: String,
    status: String,
    extractor_version: Option<String>,
    started_at: String,
    finished_at: Option<String>,
    failed_at: Option<String>,
    error: Option<String>,
    evidence: RunEvidence,
}

#[derive(Debug, Serialize)]
pub(crate) struct RunEvidence {
    file_manifests: i64,
    symbols: i64,
    graph_nodes: i64,
    graph_edges: i64,
    search_chunks: i64,
    search_sync_outbox_details: Vec<RunSearchSyncOutboxItem>,
    search_sync_outbox_state_counts: RunSearchSyncOutboxStateCounts,
    search_sync_jobs: i64,
    search_sync_job_details: Vec<RunSearchSyncJob>,
    test_cases: i64,
    test_runs: i64,
    coverage_segments: i64,
    architecture_entities: i64,
}

pub(crate) async fn get(
    State(state): State<AppState>,
    Path(run_id): Path<String>,
) -> Result<Json<RunResponse>, AppError> {
    let Some(pool) = state.database.pool.as_ref() else {
        let repo_id = local_run_repo_id(&run_id).ok_or_else(|| AppError::RunNotFound {
            run_id: run_id.clone(),
        })?;
        let local = local_index_summary(&state, repo_id)?;
        if local.run_id != run_id {
            return Err(AppError::RunNotFound { run_id });
        }
        return Ok(Json(RunResponse {
            status: "ok",
            kind: "run",
            run: local_run_summary(repo_id, local),
        }));
    };
    let run = find_run(pool, &run_id).await?;
    Ok(Json(RunResponse {
        status: "ok",
        kind: "run",
        run,
    }))
}

fn local_run_repo_id(run_id: &str) -> Option<&str> {
    let suffix = run_id.strip_prefix("local:")?;
    let (repo_id, _commit_sha) = suffix.rsplit_once(':')?;
    if repo_id.is_empty() {
        None
    } else {
        Some(repo_id)
    }
}

fn local_run_summary(repo_id: &str, local: LocalIndexSummary) -> RunSummary {
    RunSummary {
        run_id: local.run_id,
        repo_id: repo_id.to_owned(),
        commit_sha: local.commit_sha,
        index_kind: "local_worktree".to_owned(),
        status: "succeeded".to_owned(),
        extractor_version: Some("ri-api-local-index-v1".to_owned()),
        started_at: local.started_at,
        finished_at: local.finished_at,
        failed_at: None,
        error: None,
        evidence: RunEvidence {
            file_manifests: local.file_manifests,
            symbols: local.symbols,
            graph_nodes: local.graph_nodes,
            graph_edges: local.graph_edges,
            search_chunks: local.search_chunks,
            search_sync_outbox_details: Vec::new(),
            search_sync_outbox_state_counts: empty_outbox_counts(),
            search_sync_jobs: 0,
            search_sync_job_details: Vec::new(),
            test_cases: local.test_cases,
            test_runs: 0,
            coverage_segments: 0,
            architecture_entities: local.architecture_entities,
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

async fn find_run(pool: &PgPool, run_id: &str) -> Result<RunSummary, AppError> {
    let row = sqlx::query(
        r"
        SELECT
            g.generation_id,
            g.repo_id,
            g.commit_sha,
            g.index_kind,
            g.status,
            g.extractor_version,
            g.started_at::text AS started_at,
            g.finished_at::text AS finished_at,
            g.failed_at::text AS failed_at,
            g.error,
            (
                SELECT count(*)::bigint FROM file_manifests AS item
                WHERE item.generation_id = g.generation_id
            ) AS file_manifest_count,
            (
                SELECT count(*)::bigint FROM symbols AS item
                WHERE item.generation_id = g.generation_id
            ) AS symbol_count,
            (
                SELECT count(*)::bigint FROM graph_nodes AS item
                WHERE item.generation_id = g.generation_id
            ) AS graph_node_count,
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
            ) AS test_case_count,
            (
                SELECT count(*)::bigint FROM test_runs AS item
                WHERE item.generation_id = g.generation_id
            ) AS test_run_count,
            (
                SELECT count(*)::bigint FROM coverage_segments AS item
                WHERE item.generation_id = g.generation_id
            ) AS coverage_segment_count,
            (
                SELECT count(*)::bigint FROM architecture_entities AS item
                WHERE item.generation_id = g.generation_id
            ) AS architecture_entity_count
        FROM index_generations AS g
        WHERE g.generation_id = $1
        ",
    )
    .bind(run_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::RunNotFound {
        run_id: run_id.to_owned(),
    })?;
    let search_sync_job_details = find_search_sync_jobs(pool, run_id).await?;
    let search_sync_outbox_details = find_search_sync_outbox(pool, run_id).await?;
    let search_sync_outbox_state_counts = count_search_sync_outbox_states(pool, run_id).await?;
    Ok(RunSummary {
        run_id: row.try_get("generation_id")?,
        repo_id: row.try_get("repo_id")?,
        commit_sha: row.try_get("commit_sha")?,
        index_kind: row.try_get("index_kind")?,
        status: row.try_get("status")?,
        extractor_version: row.try_get("extractor_version")?,
        started_at: row.try_get("started_at")?,
        finished_at: row.try_get("finished_at")?,
        failed_at: row.try_get("failed_at")?,
        error: row.try_get("error")?,
        evidence: RunEvidence {
            file_manifests: row.try_get("file_manifest_count")?,
            symbols: row.try_get("symbol_count")?,
            graph_nodes: row.try_get("graph_node_count")?,
            graph_edges: row.try_get("graph_edge_count")?,
            search_chunks: row.try_get("search_chunk_count")?,
            search_sync_outbox_details,
            search_sync_outbox_state_counts,
            search_sync_jobs: row.try_get("search_sync_job_count")?,
            search_sync_job_details,
            test_cases: row.try_get("test_case_count")?,
            test_runs: row.try_get("test_run_count")?,
            coverage_segments: row.try_get("coverage_segment_count")?,
            architecture_entities: row.try_get("architecture_entity_count")?,
        },
    })
}

#[cfg(test)]
mod tests;

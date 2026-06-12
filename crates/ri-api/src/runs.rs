use axum::{
    Json,
    extract::{Path, State},
};
use serde::Serialize;
use sqlx::{PgPool, Row as _};

use crate::{AppError, state::AppState};

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
    search_sync_jobs: i64,
    search_sync_job_details: Vec<RunSearchSyncJob>,
    test_cases: i64,
    test_runs: i64,
    coverage_segments: i64,
    architecture_entities: i64,
}

#[derive(Debug, Serialize)]
pub(crate) struct RunSearchSyncJob {
    job_id: String,
    state: String,
    attempt_count: i32,
}

pub(crate) async fn get(
    State(state): State<AppState>,
    Path(run_id): Path<String>,
) -> Result<Json<RunResponse>, AppError> {
    let pool = state
        .database
        .pool
        .as_ref()
        .ok_or(AppError::DatabaseNotConfigured)?;
    let run = find_run(pool, &run_id).await?;
    Ok(Json(RunResponse {
        status: "ok",
        kind: "run",
        run,
    }))
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
            search_sync_jobs: row.try_get("search_sync_job_count")?,
            search_sync_job_details,
            test_cases: row.try_get("test_case_count")?,
            test_runs: row.try_get("test_run_count")?,
            coverage_segments: row.try_get("coverage_segment_count")?,
            architecture_entities: row.try_get("architecture_entity_count")?,
        },
    })
}

async fn find_search_sync_jobs(
    pool: &PgPool,
    generation_id: &str,
) -> Result<Vec<RunSearchSyncJob>, sqlx::Error> {
    let rows = sqlx::query(
        r"
        SELECT job_id, state, attempt_count
        FROM jobs
        WHERE generation_id = $1
          AND kind = 'search.sync_once'
        ORDER BY created_at ASC
        ",
    )
    .bind(generation_id)
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|row| {
            Ok(RunSearchSyncJob {
                job_id: row.try_get("job_id")?,
                state: row.try_get("state")?,
                attempt_count: row.try_get("attempt_count")?,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{RunEvidence, RunSearchSyncJob};
    use serde_json::Value;

    #[test]
    fn run_evidence_serializes_search_sync_job_details() -> Result<(), serde_json::Error> {
        let evidence = RunEvidence {
            file_manifests: 1,
            symbols: 2,
            graph_nodes: 3,
            graph_edges: 4,
            search_chunks: 5,
            search_sync_jobs: 1,
            search_sync_job_details: vec![RunSearchSyncJob {
                job_id: "job-1".to_owned(),
                state: "queued".to_owned(),
                attempt_count: 0,
            }],
            test_cases: 6,
            test_runs: 7,
            coverage_segments: 8,
            architecture_entities: 9,
        };

        let body = serde_json::to_value(evidence)?;

        assert_eq!(
            body.pointer("/search_sync_job_details/0/job_id")
                .and_then(Value::as_str),
            Some("job-1")
        );
        assert_eq!(
            body.pointer("/search_sync_job_details/0/state")
                .and_then(Value::as_str),
            Some("queued")
        );
        Ok(())
    }
}

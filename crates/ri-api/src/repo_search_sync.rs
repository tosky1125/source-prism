use axum::{
    Json,
    extract::{Path, State},
};
use serde::Serialize;
use sqlx::{PgPool, Row as _};

use crate::{
    AppError,
    run_outbox::{RunSearchSyncOutboxStateCounts, count_search_sync_outbox_states},
    state::AppState,
};

#[derive(Debug, Serialize)]
pub(crate) struct RepoSearchSyncResponse {
    status: &'static str,
    kind: &'static str,
    repo_id: String,
    latest_generation_id: Option<String>,
    latest_commit_sha: Option<String>,
    latest_run_status: Option<String>,
    outbox_state_counts: RunSearchSyncOutboxStateCounts,
    job_state_counts: JobStateCounts,
}

#[derive(Debug, Serialize)]
pub(crate) struct JobStateCounts {
    queued: i64,
    leased: i64,
    succeeded: i64,
    failed: i64,
    dead_lettered: i64,
    cancelled: i64,
    total: i64,
}

impl JobStateCounts {
    const fn zero() -> Self {
        Self {
            queued: 0,
            leased: 0,
            succeeded: 0,
            failed: 0,
            dead_lettered: 0,
            cancelled: 0,
            total: 0,
        }
    }
}

#[derive(Debug)]
struct LatestGeneration {
    generation_id: String,
    commit_sha: String,
    status: String,
}

pub(crate) async fn get(
    State(state): State<AppState>,
    Path(repo_id): Path<String>,
) -> Result<Json<RepoSearchSyncResponse>, AppError> {
    let pool = state
        .database
        .pool
        .as_ref()
        .ok_or(AppError::DatabaseNotConfigured)?;
    ensure_repo_exists(pool, &repo_id).await?;
    let generation = latest_generation(pool, &repo_id).await?;
    let (outbox_state_counts, job_state_counts) = match generation.as_ref() {
        Some(latest) => (
            count_search_sync_outbox_states(pool, &latest.generation_id).await?,
            count_job_states(pool, &latest.generation_id).await?,
        ),
        None => (zero_outbox_counts(), JobStateCounts::zero()),
    };

    Ok(Json(RepoSearchSyncResponse {
        status: "ok",
        kind: "repo_search_sync",
        repo_id,
        latest_generation_id: generation
            .as_ref()
            .map(|latest| latest.generation_id.clone()),
        latest_commit_sha: generation.as_ref().map(|latest| latest.commit_sha.clone()),
        latest_run_status: generation.map(|latest| latest.status),
        outbox_state_counts,
        job_state_counts,
    }))
}

async fn ensure_repo_exists(pool: &PgPool, repo_id: &str) -> Result<(), AppError> {
    let exists = sqlx::query_scalar::<_, bool>(
        r"
        SELECT EXISTS(
            SELECT 1 FROM repos WHERE repo_id = $1
        )
        ",
    )
    .bind(repo_id)
    .fetch_one(pool)
    .await?;
    if exists {
        Ok(())
    } else {
        Err(AppError::RepoNotFound {
            repo_id: repo_id.to_owned(),
        })
    }
}

async fn latest_generation(
    pool: &PgPool,
    repo_id: &str,
) -> Result<Option<LatestGeneration>, sqlx::Error> {
    sqlx::query(
        r"
        SELECT generation_id, commit_sha, status
        FROM index_generations
        WHERE repo_id = $1
        ORDER BY started_at DESC, generation_id DESC
        LIMIT 1
        ",
    )
    .bind(repo_id)
    .fetch_optional(pool)
    .await?
    .map(|row| {
        Ok(LatestGeneration {
            generation_id: row.try_get("generation_id")?,
            commit_sha: row.try_get("commit_sha")?,
            status: row.try_get("status")?,
        })
    })
    .transpose()
}

async fn count_job_states(
    pool: &PgPool,
    generation_id: &str,
) -> Result<JobStateCounts, sqlx::Error> {
    let row = sqlx::query(
        r"
        SELECT
            count(*) FILTER (WHERE state = 'queued')::bigint AS queued,
            count(*) FILTER (WHERE state = 'leased')::bigint AS leased,
            count(*) FILTER (WHERE state = 'succeeded')::bigint AS succeeded,
            count(*) FILTER (WHERE state = 'failed')::bigint AS failed,
            count(*) FILTER (WHERE state = 'dead_lettered')::bigint AS dead_lettered,
            count(*) FILTER (WHERE state = 'cancelled')::bigint AS cancelled,
            count(*)::bigint AS total
        FROM jobs
        WHERE generation_id = $1
          AND kind = 'search.sync_once'
        ",
    )
    .bind(generation_id)
    .fetch_one(pool)
    .await?;

    Ok(JobStateCounts {
        queued: row.try_get("queued")?,
        leased: row.try_get("leased")?,
        succeeded: row.try_get("succeeded")?,
        failed: row.try_get("failed")?,
        dead_lettered: row.try_get("dead_lettered")?,
        cancelled: row.try_get("cancelled")?,
        total: row.try_get("total")?,
    })
}

const fn zero_outbox_counts() -> RunSearchSyncOutboxStateCounts {
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

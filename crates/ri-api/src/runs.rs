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
        SELECT generation_id, repo_id, commit_sha, index_kind, status
        FROM index_generations
        WHERE generation_id = $1
        ",
    )
    .bind(run_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::RunNotFound {
        run_id: run_id.to_owned(),
    })?;
    Ok(RunSummary {
        run_id: row.try_get("generation_id")?,
        repo_id: row.try_get("repo_id")?,
        commit_sha: row.try_get("commit_sha")?,
        index_kind: row.try_get("index_kind")?,
        status: row.try_get("status")?,
    })
}

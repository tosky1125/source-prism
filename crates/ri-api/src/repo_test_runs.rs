use axum::{
    Json,
    extract::{Path, State},
};
use ri_indexer::{PgTestRunStore, TestRunRecord};
use serde::Serialize;

use crate::{AppError, state::AppState};

#[derive(Debug, Serialize)]
pub(crate) struct RepoTestRunsResponse {
    status: &'static str,
    kind: &'static str,
    repo_id: String,
    run_count: usize,
    runs: Vec<TestRunRecord>,
}

pub(crate) async fn list(
    State(state): State<AppState>,
    Path(repo_id): Path<String>,
) -> Result<Json<RepoTestRunsResponse>, AppError> {
    let pool = state
        .database
        .pool
        .as_ref()
        .ok_or(AppError::DatabaseNotConfigured)?;
    let runs = PgTestRunStore::new(pool.clone())
        .active_test_runs_for_repo(&repo_id)
        .await?;
    Ok(Json(RepoTestRunsResponse {
        status: "ok",
        kind: "test_runs",
        repo_id,
        run_count: runs.len(),
        runs,
    }))
}

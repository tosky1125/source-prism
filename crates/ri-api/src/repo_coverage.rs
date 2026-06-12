use axum::{
    Json,
    extract::{Path, State},
};
use ri_indexer::{CoverageSegmentRecord, PgCoverageStore};
use serde::Serialize;

use crate::{AppError, state::AppState};

#[derive(Debug, Serialize)]
pub(crate) struct RepoCoverageResponse {
    status: &'static str,
    kind: &'static str,
    repo_id: String,
    segment_count: usize,
    segments: Vec<CoverageSegmentRecord>,
}

pub(crate) async fn list(
    State(state): State<AppState>,
    Path(repo_id): Path<String>,
) -> Result<Json<RepoCoverageResponse>, AppError> {
    let pool = state
        .database
        .pool
        .as_ref()
        .ok_or(AppError::DatabaseNotConfigured)?;
    let segments = PgCoverageStore::new(pool.clone())
        .active_coverage_segments_for_repo(&repo_id)
        .await?;
    Ok(Json(RepoCoverageResponse {
        status: "ok",
        kind: "coverage",
        repo_id,
        segment_count: segments.len(),
        segments,
    }))
}

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
    let Some(pool) = state.database.pool.as_ref() else {
        return Ok(Json(response(repo_id, Vec::new())));
    };
    let segments = PgCoverageStore::new(pool.clone())
        .active_coverage_segments_for_repo(&repo_id)
        .await?;
    Ok(Json(response(repo_id, segments)))
}

fn response(repo_id: String, segments: Vec<CoverageSegmentRecord>) -> RepoCoverageResponse {
    RepoCoverageResponse {
        status: "ok",
        kind: "coverage",
        repo_id,
        segment_count: segments.len(),
        segments,
    }
}

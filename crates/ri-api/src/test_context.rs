use axum::{Json, extract::State};
use ri_behavior::{
    CoverageEvidenceSegment, TestContext, build_test_context, build_test_context_with_evidence,
};
use ri_indexer::{CoverageSegmentRecord, PgCoverageStore, PgGraphStore, PgSymbolStore};
use serde::{Deserialize, Serialize};

use crate::{AppError, graph_test_edges::graph_test_coverage_edges, state::AppState};

#[derive(Debug, Deserialize)]
pub(crate) struct TestContextRequest {
    repo_id: Option<String>,
    symbol: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct TestContextResponse {
    status: &'static str,
    kind: &'static str,
    test_context: TestContext,
}

pub(crate) async fn get(
    State(state): State<AppState>,
    Json(request): Json<TestContextRequest>,
) -> Result<Json<TestContextResponse>, AppError> {
    let symbol = request.symbol.trim();
    if symbol.is_empty() {
        return Err(AppError::Validation("symbol must not be empty".to_owned()));
    }
    let test_context = test_context_for_symbol(&state, request.repo_id.as_deref(), symbol).await?;
    Ok(Json(TestContextResponse {
        status: "ok",
        kind: "test_context",
        test_context,
    }))
}

async fn test_context_for_symbol(
    state: &AppState,
    repo_id: Option<&str>,
    symbol: &str,
) -> Result<TestContext, AppError> {
    let Some(repo_id) = repo_id else {
        let symbols = state.context_symbols()?.into_owned();
        return Ok(build_test_context(symbols.as_slice(), symbol)?);
    };
    let repo_id = repo_id.trim();
    if repo_id.is_empty() {
        return Err(AppError::Validation("repo_id must not be empty".to_owned()));
    }
    let pool = state
        .database
        .pool
        .as_ref()
        .ok_or(AppError::DatabaseNotConfigured)?;
    let symbols = PgSymbolStore::new(pool.clone())
        .active_symbols_for_repo(repo_id)
        .await?;
    let graph = PgGraphStore::new(pool.clone())
        .active_graph_for_repo(repo_id)
        .await?;
    let coverage_edges = graph_test_coverage_edges(&graph)?;
    let coverage_segments = PgCoverageStore::new(pool.clone())
        .active_coverage_segments_for_repo(repo_id)
        .await?;
    let coverage_evidence = coverage_segments
        .iter()
        .filter_map(coverage_segment_evidence)
        .collect::<Vec<_>>();
    Ok(build_test_context_with_evidence(
        symbols.as_slice(),
        coverage_edges.as_slice(),
        coverage_evidence.as_slice(),
        symbol,
    )?)
}

fn coverage_segment_evidence(record: &CoverageSegmentRecord) -> Option<CoverageEvidenceSegment> {
    Some(CoverageEvidenceSegment::new(
        record.file_path.clone(),
        u32::try_from(record.start_line).ok()?,
        u32::try_from(record.end_line).ok()?,
        u32::try_from(record.hit_count).ok()?,
        record.format.clone(),
        record.source_path.clone(),
    ))
}

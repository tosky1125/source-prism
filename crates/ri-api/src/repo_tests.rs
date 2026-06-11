use axum::{
    Json,
    extract::{Path, State},
};
use ri_core::SymbolKind;
use ri_indexer::{PgTestCaseStore, TestCaseRecord};
use serde::Serialize;

use crate::{AppError, state::AppState};

#[derive(Debug, Serialize)]
pub(crate) struct RepoTestsResponse {
    status: &'static str,
    kind: &'static str,
    repo_id: String,
    test_count: usize,
    tests: Vec<TestCaseRecord>,
}

pub(crate) async fn list(
    State(state): State<AppState>,
    Path(repo_id): Path<String>,
) -> Result<Json<RepoTestsResponse>, AppError> {
    let tests = if let Some(pool) = state.database.pool.as_ref() {
        PgTestCaseStore::new(pool.clone())
            .active_test_cases_for_repo(&repo_id)
            .await?
    } else {
        state
            .context_symbols()?
            .iter()
            .filter(|symbol| symbol.kind == SymbolKind::TestCase)
            .map(TestCaseRecord::from_symbol)
            .collect()
    };
    Ok(Json(RepoTestsResponse {
        status: "ok",
        kind: "tests",
        repo_id,
        test_count: tests.len(),
        tests,
    }))
}

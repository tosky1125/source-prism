use axum::{Json, extract::State};
use ri_behavior::{TestContext, build_test_context};
use serde::{Deserialize, Serialize};

use crate::{AppError, state::AppState};

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
    let symbols = state
        .symbols_for_optional_repo(request.repo_id.as_deref())
        .await?;
    Ok(Json(TestContextResponse {
        status: "ok",
        kind: "test_context",
        test_context: build_test_context(symbols.as_slice(), symbol)?,
    }))
}

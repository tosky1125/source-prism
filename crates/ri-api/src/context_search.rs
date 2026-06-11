use axum::{Json, extract::State};
use ri_context::{ContextPack, build_context_pack};
use serde::{Deserialize, Serialize};

use crate::{AppError, state::AppState};

const DEFAULT_LIMIT: usize = 8;
const MAX_LIMIT: usize = 50;

#[derive(Debug, Deserialize)]
pub(crate) struct ContextSearchRequest {
    repo_id: Option<String>,
    query: String,
    limit: Option<usize>,
}

#[derive(Debug, Serialize)]
pub(crate) struct ContextSearchResponse {
    status: &'static str,
    kind: &'static str,
    context_pack: ContextPack,
}

pub(crate) async fn search(
    State(state): State<AppState>,
    Json(request): Json<ContextSearchRequest>,
) -> Result<Json<ContextSearchResponse>, AppError> {
    let query = request.query.trim();
    if query.is_empty() {
        return Err(AppError::Validation("query must not be empty".to_owned()));
    }
    let limit = request.limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT);
    let symbols = state
        .symbols_for_optional_repo(request.repo_id.as_deref())
        .await?;
    Ok(Json(ContextSearchResponse {
        status: "ok",
        kind: "context_search",
        context_pack: build_context_pack(symbols.as_slice(), query, limit),
    }))
}

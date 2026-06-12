use axum::{
    Json,
    extract::{Path, State},
};
use ri_indexer::PgSymbolStore;
use ri_symbols::{ChangedSymbol, changed_symbols_for_diff};
use serde::{Deserialize, Serialize};

use crate::{AppError, state::AppState};

#[derive(Debug, Deserialize)]
pub(crate) struct ChangedSymbolsRequest {
    diff: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct ChangedSymbolsResponse {
    status: &'static str,
    kind: &'static str,
    repo_id: String,
    changed_line_count: usize,
    matched_symbol_count: usize,
    changed_symbols: Vec<ChangedSymbol>,
}

pub(crate) async fn map(
    State(state): State<AppState>,
    Path(repo_id): Path<String>,
    Json(request): Json<ChangedSymbolsRequest>,
) -> Result<Json<ChangedSymbolsResponse>, AppError> {
    let symbols = if let Some(pool) = state.database.pool.as_ref() {
        PgSymbolStore::new(pool.clone())
            .active_symbols_for_repo(&repo_id)
            .await?
    } else {
        state.context_symbols()?.into_owned()
    };
    let (changed_lines, changed_symbols) =
        changed_symbols_for_diff(symbols.as_slice(), request.diff.as_str());
    Ok(Json(ChangedSymbolsResponse {
        status: "ok",
        kind: "changed_symbols",
        repo_id,
        changed_line_count: changed_lines.len(),
        matched_symbol_count: changed_symbols.len(),
        changed_symbols,
    }))
}

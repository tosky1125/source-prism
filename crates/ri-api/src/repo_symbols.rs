use axum::{
    Json,
    extract::{Path, State},
};
use ri_indexer::PgSymbolStore;
use ri_symbols::SymbolRecord;
use serde::Serialize;

use crate::{AppError, state::AppState};

#[derive(Debug, Serialize)]
pub(crate) struct RepoSymbolsResponse {
    status: &'static str,
    kind: &'static str,
    repo_id: String,
    symbol_count: usize,
    symbols: Vec<SymbolRecord>,
}

pub(crate) async fn list(
    State(state): State<AppState>,
    Path(repo_id): Path<String>,
) -> Result<Json<RepoSymbolsResponse>, AppError> {
    let symbols = if let Some(pool) = state.database.pool.as_ref() {
        PgSymbolStore::new(pool.clone())
            .active_symbols_for_repo(&repo_id)
            .await?
    } else {
        state.context_symbols()?.into_owned()
    };
    Ok(Json(RepoSymbolsResponse {
        status: "ok",
        kind: "symbols",
        repo_id,
        symbol_count: symbols.len(),
        symbols,
    }))
}

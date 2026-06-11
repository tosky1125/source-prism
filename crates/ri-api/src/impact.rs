use axum::{Json, extract::State};
use ri_impact::{ImpactReport, analyze_symbol_impact};
use ri_indexer::PgSymbolStore;
use ri_symbols::SymbolRecord;
use serde::{Deserialize, Serialize};

use crate::{AppError, state::AppState};

#[derive(Debug, Deserialize)]
pub(crate) struct ImpactRequest {
    repo_id: Option<String>,
    symbol: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct ImpactResponse {
    status: &'static str,
    kind: &'static str,
    impact: ImpactReport,
}

pub(crate) async fn analyze(
    State(state): State<AppState>,
    Json(request): Json<ImpactRequest>,
) -> Result<Json<ImpactResponse>, AppError> {
    let symbol = request.symbol.trim();
    if symbol.is_empty() {
        return Err(AppError::Validation("symbol must not be empty".to_owned()));
    }
    let symbols = impact_symbols(&state, request.repo_id.as_deref()).await?;
    Ok(Json(ImpactResponse {
        status: "ok",
        kind: "impact",
        impact: analyze_symbol_impact(symbols, symbol)?,
    }))
}

async fn impact_symbols(
    state: &AppState,
    repo_id: Option<&str>,
) -> Result<Vec<SymbolRecord>, AppError> {
    let Some(repo_id) = repo_id else {
        return Ok(state.context_symbols()?.into_owned());
    };
    let repo_id = repo_id.trim();
    if repo_id.is_empty() {
        return Err(AppError::Validation("repo_id must not be empty".to_owned()));
    }
    let Some(pool) = state.database.pool.as_ref() else {
        return Err(AppError::DatabaseNotConfigured);
    };
    PgSymbolStore::new(pool.clone())
        .active_symbols_for_repo(repo_id)
        .await
        .map_err(AppError::from)
}

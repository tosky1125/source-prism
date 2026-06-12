use axum::{
    Json,
    extract::{Path, State},
};
use ri_impact::{ImpactCallEdge, ImpactReport, analyze_symbol_impact_with_calls};
use ri_indexer::{PgGraphStore, PgSymbolStore};
use serde::{Deserialize, Serialize};

use crate::{
    AppError,
    graph_call_edges::{context_call_edges, graph_call_edges},
    state::AppState,
};

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
    analyze_with_repo(state, request, None).await
}

pub(crate) async fn analyze_for_repo(
    State(state): State<AppState>,
    Path(repo_id): Path<String>,
    Json(request): Json<ImpactRequest>,
) -> Result<Json<ImpactResponse>, AppError> {
    analyze_with_repo(state, request, Some(repo_id)).await
}

async fn analyze_with_repo(
    state: AppState,
    request: ImpactRequest,
    repo_id: Option<String>,
) -> Result<Json<ImpactResponse>, AppError> {
    let symbol = request.symbol.trim();
    if symbol.is_empty() {
        return Err(AppError::Validation("symbol must not be empty".to_owned()));
    }
    let repo_id = repo_id.as_deref().or(request.repo_id.as_deref());
    let (symbols, calls) = impact_inputs(&state, repo_id).await?;
    Ok(Json(ImpactResponse {
        status: "ok",
        kind: "impact",
        impact: analyze_symbol_impact_with_calls(symbols, calls.as_slice(), symbol)?,
    }))
}

pub(crate) async fn impact_inputs(
    state: &AppState,
    repo_id: Option<&str>,
) -> Result<(Vec<ri_symbols::SymbolRecord>, Vec<ImpactCallEdge>), AppError> {
    let Some(repo_id) = repo_id else {
        let evidence = state.context_index_evidence()?;
        return Ok((evidence.symbols, context_call_edges(&evidence.calls)));
    };
    let repo_id = repo_id.trim();
    if repo_id.is_empty() {
        return Err(AppError::Validation("repo_id must not be empty".to_owned()));
    }
    let Some(pool) = state.database.pool.as_ref() else {
        let evidence = state.context_index_evidence()?;
        return Ok((evidence.symbols, context_call_edges(&evidence.calls)));
    };
    let symbols = PgSymbolStore::new(pool.clone())
        .active_symbols_for_repo(repo_id)
        .await?;
    let graph = PgGraphStore::new(pool.clone())
        .active_graph_for_repo(repo_id)
        .await?;
    Ok((symbols, graph_call_edges(&graph)?))
}

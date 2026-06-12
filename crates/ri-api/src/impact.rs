use axum::{Json, extract::State};
use ri_core::SymbolId;
use ri_impact::{ImpactCallEdge, ImpactReport, analyze_symbol_impact_with_calls};
use ri_indexer::{GraphProjection, PgGraphStore, PgSymbolStore};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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
    let (symbols, calls) = impact_inputs(&state, request.repo_id.as_deref()).await?;
    Ok(Json(ImpactResponse {
        status: "ok",
        kind: "impact",
        impact: analyze_symbol_impact_with_calls(symbols, calls.as_slice(), symbol)?,
    }))
}

async fn impact_inputs(
    state: &AppState,
    repo_id: Option<&str>,
) -> Result<(Vec<ri_symbols::SymbolRecord>, Vec<ImpactCallEdge>), AppError> {
    let Some(repo_id) = repo_id else {
        let evidence = state.context_index_evidence()?;
        return Ok((evidence.symbols, call_edges_from_context(&evidence.calls)));
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
    Ok((symbols, call_edges_from_graph(&graph)?))
}

fn call_edges_from_context(calls: &[ri_context::ResolvedCallReference]) -> Vec<ImpactCallEdge> {
    calls
        .iter()
        .map(|call| {
            ImpactCallEdge::new(call.source_symbol_id.clone(), call.target_symbol_id.clone())
        })
        .collect()
}

fn call_edges_from_graph(graph: &GraphProjection) -> Result<Vec<ImpactCallEdge>, AppError> {
    let subject_by_node = graph
        .nodes
        .iter()
        .filter_map(|node| {
            node.subject_id
                .as_ref()
                .map(|subject_id| (node.graph_node_id.as_str(), subject_id.as_str()))
        })
        .collect::<BTreeMap<_, _>>();
    graph
        .edges
        .iter()
        .filter(|edge| edge.edge_type == "calls")
        .filter_map(|edge| {
            let source = subject_by_node.get(edge.source_node_id.as_str())?;
            let target = subject_by_node.get(edge.target_node_id.as_str())?;
            Some(symbol_call_edge(source, target))
        })
        .collect()
}

fn symbol_call_edge(source: &str, target: &str) -> Result<ImpactCallEdge, AppError> {
    let source_symbol_id =
        SymbolId::new(source).map_err(|error| AppError::Validation(error.to_string()))?;
    let target_symbol_id =
        SymbolId::new(target).map_err(|error| AppError::Validation(error.to_string()))?;
    Ok(ImpactCallEdge::new(source_symbol_id, target_symbol_id))
}

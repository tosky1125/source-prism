use axum::{Json, extract::State};
use ri_context::{ContextPack, build_context_pack_with_calls};
use ri_impact::ImpactCallEdge;
use ri_indexer::{PgGraphStore, PgSearchSyncStore, PgSymbolStore};
use serde::{Deserialize, Serialize};

use crate::{
    AppError,
    graph_call_edges::{context_call_edges, graph_call_edges},
    state::AppState,
};

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
    hit_count: usize,
    impact_count: usize,
    search_chunk_count: i64,
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
    let (symbols, calls, search_chunk_count) =
        context_inputs(&state, request.repo_id.as_deref()).await?;
    let context_pack =
        build_context_pack_with_calls(symbols.as_slice(), calls.as_slice(), query, limit);
    Ok(Json(ContextSearchResponse {
        status: "ok",
        kind: "context_search",
        hit_count: context_pack.hits.len(),
        impact_count: context_pack.impacts.len(),
        search_chunk_count,
        context_pack,
    }))
}

async fn context_inputs(
    state: &AppState,
    repo_id: Option<&str>,
) -> Result<(Vec<ri_symbols::SymbolRecord>, Vec<ImpactCallEdge>, i64), AppError> {
    let Some(repo_id) = repo_id else {
        let evidence = state.context_index_evidence()?;
        return Ok((evidence.symbols, context_call_edges(&evidence.calls), 0));
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
    let search_chunk_count = PgSearchSyncStore::new(pool.clone())
        .active_symbol_chunk_count_for_repo(repo_id)
        .await?;
    Ok((symbols, graph_call_edges(&graph)?, search_chunk_count))
}

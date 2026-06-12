use axum::{
    Json,
    extract::{Path, Query, State},
};
use ri_context::{
    ReferenceDirection, ReferenceEndpoints, ReferenceEvidence, ReferenceReport, SymbolReference,
    find_symbol_references, reference_report, symbol_for_query,
};
use ri_core::Confidence;
use ri_indexer::{GraphProjection, PgGraphStore, PgSymbolStore};
use ri_symbols::{SymbolRange, SymbolRecord};
use serde::Deserialize;
use std::collections::BTreeMap;

use crate::{AppError, state::AppState};

#[derive(Debug, Deserialize)]
pub(crate) struct ReferencesQuery {
    symbol: String,
}

pub(crate) async fn list(
    State(state): State<AppState>,
    Path(repo_id): Path<String>,
    Query(query): Query<ReferencesQuery>,
) -> Result<Json<ReferenceReport>, AppError> {
    let symbol = query.symbol.trim();
    if symbol.is_empty() {
        return Err(AppError::Validation("symbol must not be empty".to_owned()));
    }
    if let Some(pool) = state.database.pool.as_ref() {
        let symbols = PgSymbolStore::new(pool.clone())
            .active_symbols_for_repo(&repo_id)
            .await?;
        let graph = PgGraphStore::new(pool.clone())
            .active_graph_for_repo(&repo_id)
            .await?;
        return Ok(Json(references_from_graph(
            symbols.as_slice(),
            &graph,
            symbol,
        )?));
    }
    let evidence = state.context_index_evidence()?;
    Ok(Json(find_symbol_references(
        evidence.symbols.as_slice(),
        evidence.calls.as_slice(),
        symbol,
    )?))
}

fn references_from_graph(
    symbols: &[SymbolRecord],
    graph: &GraphProjection,
    query: &str,
) -> Result<ReferenceReport, AppError> {
    let symbol = symbol_for_query(symbols, query)?;
    let node_by_id = graph
        .nodes
        .iter()
        .map(|node| (node.graph_node_id.as_str(), node))
        .collect::<BTreeMap<_, _>>();
    let references = graph
        .edges
        .iter()
        .filter(|edge| edge.edge_type == "calls" || edge.edge_type == "test_covers")
        .filter_map(|edge| {
            let source = node_by_id.get(edge.source_node_id.as_str())?;
            let target = node_by_id.get(edge.target_node_id.as_str())?;
            let source_subject = source.subject_id.as_ref()?;
            let target_subject = target.subject_id.as_ref()?;
            let direction = if target_subject == symbol.versioned_symbol_id.as_str() {
                ReferenceDirection::Incoming
            } else if source_subject == symbol.versioned_symbol_id.as_str() {
                ReferenceDirection::Outgoing
            } else {
                return None;
            };
            Some(SymbolReference::new(
                direction,
                edge.edge_type.clone(),
                ReferenceEndpoints::new(source.display_name.clone(), target.display_name.clone()),
                ReferenceEvidence::new(
                    edge.evidence_file_path.clone().unwrap_or_default(),
                    evidence_range(edge)?,
                    confidence_tier(edge.confidence),
                    edge.resolution_method.clone(),
                ),
            ))
        })
        .collect();
    Ok(reference_report(symbol, references))
}

fn evidence_range(edge: &ri_indexer::GraphEdgeRecord) -> Option<SymbolRange> {
    Some(SymbolRange::new(
        u32::try_from(edge.evidence_start_line?).ok()?,
        u32::try_from(edge.evidence_start_col?).ok()?,
        u32::try_from(edge.evidence_end_line?).ok()?,
        u32::try_from(edge.evidence_end_col?).ok()?,
    ))
}

fn confidence_tier(confidence: f64) -> Confidence {
    if confidence >= 0.95 {
        Confidence::Exact
    } else if confidence >= 0.80 {
        Confidence::High
    } else if confidence >= 0.50 {
        Confidence::Medium
    } else {
        Confidence::Low
    }
}

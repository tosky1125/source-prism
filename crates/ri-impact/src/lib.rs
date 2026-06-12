#![allow(
    missing_docs,
    reason = "Impact result contracts are self-describing at this milestone."
)]

use ri_core::SymbolId;
use ri_graph::{GraphRelation, SymbolGraph};
use ri_symbols::SymbolRecord;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum ImpactError {
    #[error("symbol not found: {query}")]
    SymbolNotFound { query: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ImpactReport {
    pub symbol: SymbolRecord,
    pub affected_files: Vec<String>,
    pub related: Vec<GraphRelation>,
    pub direct_callers: Vec<String>,
    pub direct_callees: Vec<String>,
    pub impact_score: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct ImpactCallEdge {
    pub source_symbol_id: SymbolId,
    pub target_symbol_id: SymbolId,
}

impl ImpactCallEdge {
    pub const fn new(source_symbol_id: SymbolId, target_symbol_id: SymbolId) -> Self {
        Self {
            source_symbol_id,
            target_symbol_id,
        }
    }
}

pub fn analyze_symbol_impact(
    symbols: Vec<SymbolRecord>,
    query: &str,
) -> Result<ImpactReport, ImpactError> {
    analyze_symbol_impact_with_calls(symbols, &[], query)
}

pub fn analyze_symbol_impact_with_calls(
    symbols: Vec<SymbolRecord>,
    calls: &[ImpactCallEdge],
    query: &str,
) -> Result<ImpactReport, ImpactError> {
    let symbols_by_id = symbols_by_id(&symbols);
    let graph = SymbolGraph::new(symbols);
    let symbol = graph
        .find_by_query(query)
        .ok_or_else(|| ImpactError::SymbolNotFound {
            query: query.to_owned(),
        })?
        .clone();
    let inbound_symbol_ids = direct_callers(calls, &symbol, &symbols_by_id);
    let outbound_symbol_ids = direct_callees(calls, &symbol, &symbols_by_id);
    let affected_files = affected_files(
        &symbol,
        &inbound_symbol_ids,
        &outbound_symbol_ids,
        &symbols_by_id,
    );
    let related = graph.related_symbols(&symbol);
    let impact_score = u32::try_from(related.len())
        .unwrap_or(u32::MAX)
        .saturating_add(u32::try_from(inbound_symbol_ids.len()).unwrap_or(u32::MAX))
        .saturating_add(u32::try_from(outbound_symbol_ids.len()).unwrap_or(u32::MAX))
        .saturating_add(1);
    Ok(ImpactReport {
        affected_files,
        symbol,
        related,
        direct_callers: symbol_names(&inbound_symbol_ids, &symbols_by_id),
        direct_callees: symbol_names(&outbound_symbol_ids, &symbols_by_id),
        impact_score,
    })
}

fn symbols_by_id(symbols: &[SymbolRecord]) -> BTreeMap<SymbolId, SymbolRecord> {
    symbols
        .iter()
        .map(|symbol| (symbol.versioned_symbol_id.clone(), symbol.clone()))
        .collect()
}

fn direct_callers(
    calls: &[ImpactCallEdge],
    symbol: &SymbolRecord,
    symbols_by_id: &BTreeMap<SymbolId, SymbolRecord>,
) -> Vec<SymbolId> {
    unique_call_symbols(
        calls
            .iter()
            .filter(|call| call.target_symbol_id == symbol.versioned_symbol_id)
            .map(|call| call.source_symbol_id.clone()),
        symbols_by_id,
    )
}

fn direct_callees(
    calls: &[ImpactCallEdge],
    symbol: &SymbolRecord,
    symbols_by_id: &BTreeMap<SymbolId, SymbolRecord>,
) -> Vec<SymbolId> {
    unique_call_symbols(
        calls
            .iter()
            .filter(|call| call.source_symbol_id == symbol.versioned_symbol_id)
            .map(|call| call.target_symbol_id.clone()),
        symbols_by_id,
    )
}

fn unique_call_symbols(
    symbols: impl Iterator<Item = SymbolId>,
    symbols_by_id: &BTreeMap<SymbolId, SymbolRecord>,
) -> Vec<SymbolId> {
    symbols
        .filter(|symbol_id| symbols_by_id.contains_key(symbol_id))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn affected_files(
    symbol: &SymbolRecord,
    inbound_symbol_ids: &[SymbolId],
    outbound_symbol_ids: &[SymbolId],
    symbols_by_id: &BTreeMap<SymbolId, SymbolRecord>,
) -> Vec<String> {
    let mut files = BTreeSet::from([symbol.file_path.to_string()]);
    for symbol_id in inbound_symbol_ids.iter().chain(outbound_symbol_ids.iter()) {
        if let Some(call_symbol) = symbols_by_id.get(symbol_id) {
            files.insert(call_symbol.file_path.to_string());
        }
    }
    files.into_iter().collect()
}

fn symbol_names(
    symbol_ids: &[SymbolId],
    symbols_by_id: &BTreeMap<SymbolId, SymbolRecord>,
) -> Vec<String> {
    symbol_ids
        .iter()
        .filter_map(|symbol_id| symbols_by_id.get(symbol_id))
        .map(|symbol| symbol.fqn.clone())
        .collect()
}

#![allow(
    missing_docs,
    reason = "Impact result contracts are self-describing at this milestone."
)]

use ri_graph::{GraphRelation, SymbolGraph};
use ri_symbols::SymbolRecord;
use serde::{Deserialize, Serialize};
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

pub fn analyze_symbol_impact(
    symbols: Vec<SymbolRecord>,
    query: &str,
) -> Result<ImpactReport, ImpactError> {
    let graph = SymbolGraph::new(symbols);
    let symbol = graph
        .find_by_query(query)
        .ok_or_else(|| ImpactError::SymbolNotFound {
            query: query.to_owned(),
        })?
        .clone();
    let related = graph.related_symbols(&symbol);
    let impact_score = u32::try_from(related.len())
        .unwrap_or(u32::MAX)
        .saturating_add(1);
    Ok(ImpactReport {
        affected_files: vec![symbol.file_path.to_string()],
        symbol,
        related,
        direct_callers: Vec::new(),
        direct_callees: Vec::new(),
        impact_score,
    })
}

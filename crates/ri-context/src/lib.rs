#![allow(
    missing_docs,
    reason = "Context pack contracts are self-describing at this milestone."
)]

use ri_impact::{ImpactReport, analyze_symbol_impact};
use ri_search::{SearchHit, search_symbols};
use ri_symbols::SymbolRecord;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ContextPack {
    pub query: String,
    pub retrieval_modes: Vec<RetrievalMode>,
    pub vector_used: bool,
    pub vector_only: bool,
    pub hits: Vec<SearchHit>,
    pub impacts: Vec<ImpactReport>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum RetrievalMode {
    ExactIdentifier,
    Lexical,
    SymbolGraphProximity,
}

pub fn build_context_pack(symbols: &[SymbolRecord], query: &str, limit: usize) -> ContextPack {
    let search = search_symbols(symbols, query, limit);
    let impacts = search
        .hits
        .iter()
        .filter_map(|hit| analyze_symbol_impact(symbols.to_vec(), &hit.symbol.fqn).ok())
        .collect::<Vec<_>>();
    ContextPack {
        query: query.to_owned(),
        retrieval_modes: vec![
            RetrievalMode::ExactIdentifier,
            RetrievalMode::Lexical,
            RetrievalMode::SymbolGraphProximity,
        ],
        vector_used: search.vector_used,
        vector_only: search.vector_only,
        hits: search.hits,
        impacts,
    }
}

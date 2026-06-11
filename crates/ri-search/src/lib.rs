#![allow(
    missing_docs,
    reason = "Search ranking contracts are self-describing at this milestone."
)]

use ri_symbols::SymbolRecord;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SearchHit {
    pub symbol: SymbolRecord,
    pub score: u32,
    pub exact_identifier_match: bool,
    pub lexical_match: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SearchReport {
    pub query: String,
    pub vector_used: bool,
    pub vector_only: bool,
    pub hits: Vec<SearchHit>,
}

pub fn search_symbols(symbols: &[SymbolRecord], query: &str, limit: usize) -> SearchReport {
    let normalized_query = query.trim().to_ascii_lowercase();
    let query_terms = terms(&normalized_query);
    let mut hits = symbols
        .iter()
        .filter_map(|symbol| rank_symbol(symbol, &normalized_query, &query_terms))
        .collect::<Vec<_>>();
    hits.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then(left.symbol.file_path.cmp(&right.symbol.file_path))
            .then(left.symbol.fqn.cmp(&right.symbol.fqn))
    });
    hits.truncate(limit);
    SearchReport {
        query: query.to_owned(),
        vector_used: false,
        vector_only: false,
        hits,
    }
}

fn rank_symbol(symbol: &SymbolRecord, query: &str, query_terms: &[&str]) -> Option<SearchHit> {
    let fqn = symbol.fqn.to_ascii_lowercase();
    let name = symbol.name.to_ascii_lowercase();
    let path = symbol.file_path.to_string().to_ascii_lowercase();
    let exact_identifier_match = fqn == query || name == query;
    let lexical_score = query_terms
        .iter()
        .filter(|term| fqn.contains(**term) || name.contains(**term) || path.contains(**term))
        .count();
    if !exact_identifier_match && lexical_score == 0 {
        return None;
    }
    let lexical_match = lexical_score > 0;
    let score = u32::from(exact_identifier_match)
        .saturating_mul(100)
        .saturating_add(
            u32::try_from(lexical_score)
                .unwrap_or(u32::MAX)
                .saturating_mul(10),
        );
    Some(SearchHit {
        symbol: symbol.clone(),
        score,
        exact_identifier_match,
        lexical_match,
    })
}

fn terms(query: &str) -> Vec<&str> {
    query
        .split(|character: char| !character.is_ascii_alphanumeric() && character != '_')
        .filter(|term| !term.is_empty())
        .collect()
}

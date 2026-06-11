#![allow(
    missing_docs,
    reason = "Graph projection contracts are self-describing at this milestone."
)]

use ri_core::{Confidence, EvidenceSourceKind, EvidenceSpan, SourcePosition, SymbolKind};
use ri_symbols::SymbolRecord;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct GraphRelation {
    pub source_fqn: String,
    pub target_fqn: String,
    pub relation: GraphRelationKind,
    pub confidence: Confidence,
    pub evidence: EvidenceSpan,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum GraphRelationKind {
    SameFile,
    Contains,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SymbolGraph {
    symbols: Vec<SymbolRecord>,
}

impl SymbolGraph {
    pub fn new(mut symbols: Vec<SymbolRecord>) -> Self {
        symbols.sort_by(|left, right| {
            left.file_path
                .cmp(&right.file_path)
                .then(left.range.start_line.cmp(&right.range.start_line))
                .then(left.fqn.cmp(&right.fqn))
        });
        Self { symbols }
    }

    pub fn find_by_query(&self, query: &str) -> Option<&SymbolRecord> {
        self.symbols
            .iter()
            .find(|symbol| symbol.fqn == query || symbol.name == query)
    }

    pub fn related_symbols(&self, symbol: &SymbolRecord) -> Vec<GraphRelation> {
        self.symbols
            .iter()
            .filter(|candidate| {
                candidate.file_path == symbol.file_path
                    && candidate.versioned_symbol_id != symbol.versioned_symbol_id
            })
            .filter(|candidate| candidate.kind != SymbolKind::Module)
            .take(16)
            .map(|candidate| GraphRelation {
                source_fqn: symbol.fqn.clone(),
                target_fqn: candidate.fqn.clone(),
                relation: GraphRelationKind::SameFile,
                confidence: Confidence::Low,
                evidence: evidence_for(candidate),
            })
            .collect()
    }
}

fn evidence_for(symbol: &SymbolRecord) -> EvidenceSpan {
    EvidenceSpan::from_source(
        symbol.file_path.clone(),
        SourcePosition::new(symbol.range.start_line, symbol.range.start_column),
        SourcePosition::new(symbol.range.end_line, symbol.range.end_column),
        EvidenceSourceKind::RepositoryCode,
    )
}

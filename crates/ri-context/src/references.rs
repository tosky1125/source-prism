use ri_core::{Confidence, SymbolId};
use ri_symbols::{SymbolRange, SymbolRecord};
use serde::Serialize;
use std::collections::BTreeMap;

use crate::{ContextError, ResolvedCallReference};

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct ReferenceReport {
    pub kind: &'static str,
    pub symbol: SymbolRecord,
    pub incoming_count: usize,
    pub outgoing_count: usize,
    pub references: Vec<SymbolReference>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct SymbolReference {
    pub direction: ReferenceDirection,
    pub relation: String,
    pub source_fqn: String,
    pub target_fqn: String,
    pub file_path: String,
    pub range: SymbolRange,
    pub confidence: Confidence,
    pub evidence: String,
}

impl SymbolReference {
    pub fn new(
        direction: ReferenceDirection,
        relation: String,
        endpoints: ReferenceEndpoints,
        evidence: ReferenceEvidence,
    ) -> Self {
        Self {
            direction,
            relation,
            source_fqn: endpoints.source_fqn,
            target_fqn: endpoints.target_fqn,
            file_path: evidence.file_path,
            range: evidence.range,
            confidence: evidence.confidence,
            evidence: evidence.evidence,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct ReferenceEndpoints {
    source_fqn: String,
    target_fqn: String,
}

impl ReferenceEndpoints {
    pub const fn new(source_fqn: String, target_fqn: String) -> Self {
        Self {
            source_fqn,
            target_fqn,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct ReferenceEvidence {
    file_path: String,
    range: SymbolRange,
    confidence: Confidence,
    evidence: String,
}

impl ReferenceEvidence {
    pub const fn new(
        file_path: String,
        range: SymbolRange,
        confidence: Confidence,
        evidence: String,
    ) -> Self {
        Self {
            file_path,
            range,
            confidence,
            evidence,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ReferenceDirection {
    Incoming,
    Outgoing,
}

pub fn find_symbol_references(
    symbols: &[SymbolRecord],
    calls: &[ResolvedCallReference],
    query: &str,
) -> Result<ReferenceReport, ContextError> {
    let symbol = symbol_for_query(symbols, query)?;
    let symbols_by_id = symbols_by_id(symbols);
    let references = calls
        .iter()
        .filter_map(|call| reference_from_call(call, &symbol, &symbols_by_id))
        .collect();
    Ok(reference_report(symbol, references))
}

pub fn reference_report(symbol: SymbolRecord, references: Vec<SymbolReference>) -> ReferenceReport {
    let incoming_count = references
        .iter()
        .filter(|reference| reference.direction == ReferenceDirection::Incoming)
        .count();
    let outgoing_count = references
        .iter()
        .filter(|reference| reference.direction == ReferenceDirection::Outgoing)
        .count();
    ReferenceReport {
        kind: "references",
        symbol,
        incoming_count,
        outgoing_count,
        references,
    }
}

pub fn symbol_for_query(
    symbols: &[SymbolRecord],
    query: &str,
) -> Result<SymbolRecord, ContextError> {
    symbols
        .iter()
        .find(|symbol| symbol.fqn == query || symbol.name == query)
        .cloned()
        .ok_or_else(|| ContextError::SymbolNotFound {
            query: query.to_owned(),
        })
}

fn symbols_by_id(symbols: &[SymbolRecord]) -> BTreeMap<SymbolId, SymbolRecord> {
    symbols
        .iter()
        .map(|symbol| (symbol.versioned_symbol_id.clone(), symbol.clone()))
        .collect()
}

fn reference_from_call(
    call: &ResolvedCallReference,
    symbol: &SymbolRecord,
    symbols_by_id: &BTreeMap<SymbolId, SymbolRecord>,
) -> Option<SymbolReference> {
    let source = symbols_by_id.get(&call.source_symbol_id)?;
    let target = symbols_by_id.get(&call.target_symbol_id)?;
    let direction = if call.target_symbol_id == symbol.versioned_symbol_id {
        ReferenceDirection::Incoming
    } else if call.source_symbol_id == symbol.versioned_symbol_id {
        ReferenceDirection::Outgoing
    } else {
        return None;
    };
    Some(SymbolReference::new(
        direction,
        "calls".to_owned(),
        ReferenceEndpoints::new(source.fqn.clone(), target.fqn.clone()),
        ReferenceEvidence::new(
            call.file_path.to_string(),
            call.range.clone(),
            Confidence::Medium,
            format!("call target: {}", call.target_name),
        ),
    ))
}

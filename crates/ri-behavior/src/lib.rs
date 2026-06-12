#![allow(
    missing_docs,
    reason = "Behavior evidence contracts are self-describing at this milestone."
)]

mod cobertura;
mod junit;
mod lcov;

use ri_core::{Confidence, Language, SymbolId, SymbolKind};
use ri_symbols::{SymbolRange, SymbolRecord};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

pub use cobertura::parse_cobertura_xml;
pub use junit::{JunitReport, TestCaseResult, TestResultStatus, TestSuiteResult, parse_junit_xml};
pub use lcov::{CoverageFile, CoverageReport, CoverageSegment, parse_lcov};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct TestContext {
    pub symbol: String,
    pub code_execution_allowed: bool,
    pub execution_policy: ExecutionPolicy,
    pub coverage_available: bool,
    pub related_tests: Vec<RelatedTest>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ExecutionPolicy {
    StaticOnlySandboxRequired,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RelatedTest {
    pub fqn: String,
    pub name: String,
    pub file_path: String,
    pub language: Language,
    pub range: SymbolRange,
    pub confidence: Confidence,
    pub evidence: String,
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum BehaviorError {
    #[error("symbol not found: {symbol}")]
    SymbolNotFound { symbol: String },
    #[error("failed to parse junit xml: {message}")]
    JunitXml { message: String },
    #[error("failed to parse lcov: {message}")]
    Lcov { message: String },
    #[error("failed to parse cobertura xml: {message}")]
    CoberturaXml { message: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct TestCoverageEdge {
    pub test_symbol_id: SymbolId,
    pub target_symbol_id: SymbolId,
    pub confidence: Confidence,
    pub evidence: String,
}

impl TestCoverageEdge {
    pub const fn new(
        test_symbol_id: SymbolId,
        target_symbol_id: SymbolId,
        confidence: Confidence,
        evidence: String,
    ) -> Self {
        Self {
            test_symbol_id,
            target_symbol_id,
            confidence,
            evidence,
        }
    }
}

pub fn build_test_context(
    symbols: &[SymbolRecord],
    symbol_query: &str,
) -> Result<TestContext, BehaviorError> {
    build_test_context_with_coverage(symbols, &[], symbol_query)
}

pub fn build_test_context_with_coverage(
    symbols: &[SymbolRecord],
    coverage_edges: &[TestCoverageEdge],
    symbol_query: &str,
) -> Result<TestContext, BehaviorError> {
    let target = symbols
        .iter()
        .find(|symbol| symbol.fqn == symbol_query || symbol.name == symbol_query)
        .ok_or_else(|| BehaviorError::SymbolNotFound {
            symbol: symbol_query.to_owned(),
        })?;
    let symbols_by_id = symbols_by_id(symbols);
    let graph_test_ids = graph_test_ids(target, coverage_edges);
    let mut seen = BTreeSet::new();
    let mut related_tests = coverage_edges
        .iter()
        .filter(|edge| edge.target_symbol_id == target.versioned_symbol_id)
        .filter_map(|edge| graph_related_test(edge, &symbols_by_id))
        .inspect(|related| {
            seen.insert(related.fqn.clone());
        })
        .collect::<Vec<_>>();
    related_tests.extend(
        symbols
            .iter()
            .filter(|symbol| symbol.kind == SymbolKind::TestCase)
            .filter(|test| !graph_test_ids.contains(&test.versioned_symbol_id))
            .filter_map(|test| related_test_for(target, test))
            .filter(|test| seen.insert(test.fqn.clone())),
    );
    related_tests.sort_by(|left, right| {
        left.file_path
            .cmp(&right.file_path)
            .then(left.fqn.cmp(&right.fqn))
    });

    Ok(TestContext {
        symbol: target.fqn.clone(),
        code_execution_allowed: false,
        execution_policy: ExecutionPolicy::StaticOnlySandboxRequired,
        coverage_available: false,
        related_tests,
    })
}

fn symbols_by_id(symbols: &[SymbolRecord]) -> BTreeMap<SymbolId, &SymbolRecord> {
    symbols
        .iter()
        .map(|symbol| (symbol.versioned_symbol_id.clone(), symbol))
        .collect()
}

fn graph_test_ids(
    target: &SymbolRecord,
    coverage_edges: &[TestCoverageEdge],
) -> BTreeSet<SymbolId> {
    coverage_edges
        .iter()
        .filter(|edge| edge.target_symbol_id == target.versioned_symbol_id)
        .map(|edge| edge.test_symbol_id.clone())
        .collect()
}

fn graph_related_test(
    edge: &TestCoverageEdge,
    symbols_by_id: &BTreeMap<SymbolId, &SymbolRecord>,
) -> Option<RelatedTest> {
    let test = symbols_by_id.get(&edge.test_symbol_id)?;
    Some(RelatedTest {
        fqn: test.fqn.clone(),
        name: test.name.clone(),
        file_path: test.file_path.as_str().to_owned(),
        language: test.language,
        range: test.range.clone(),
        confidence: edge.confidence,
        evidence: edge.evidence.clone(),
    })
}

fn related_test_for(target: &SymbolRecord, test: &SymbolRecord) -> Option<RelatedTest> {
    if normalized_contains(&test.fqn, &target.name) || test.file_path == target.file_path {
        return Some(RelatedTest {
            fqn: test.fqn.clone(),
            name: test.name.clone(),
            file_path: test.file_path.as_str().to_owned(),
            language: test.language,
            range: test.range.clone(),
            confidence: confidence_for(target, test),
            evidence: evidence_for(target, test),
        });
    }
    None
}

fn confidence_for(target: &SymbolRecord, test: &SymbolRecord) -> Confidence {
    if normalized_contains(&test.fqn, &target.name) {
        Confidence::Medium
    } else {
        Confidence::Low
    }
}

fn evidence_for(target: &SymbolRecord, test: &SymbolRecord) -> String {
    if normalized_contains(&test.fqn, &target.name) {
        return "test name references target symbol".to_owned();
    }
    if test.file_path == target.file_path {
        return "test shares source file with target symbol".to_owned();
    }
    "static test symbol candidate".to_owned()
}

fn normalized_contains(haystack: &str, needle: &str) -> bool {
    let normalized_haystack = normalize_identifier(haystack);
    let normalized_needle = normalize_identifier(needle);
    !normalized_needle.is_empty() && normalized_haystack.contains(normalized_needle.as_str())
}

fn normalize_identifier(value: &str) -> String {
    value
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .flat_map(char::to_lowercase)
        .collect()
}

#![allow(
    missing_docs,
    reason = "Behavior evidence contracts are self-describing at this milestone."
)]

use ri_core::{Confidence, Language, SymbolKind};
use ri_symbols::{SymbolRange, SymbolRecord};
use serde::{Deserialize, Serialize};
use thiserror::Error;

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
}

pub fn build_test_context(
    symbols: &[SymbolRecord],
    symbol_query: &str,
) -> Result<TestContext, BehaviorError> {
    let target = symbols
        .iter()
        .find(|symbol| symbol.fqn == symbol_query || symbol.name == symbol_query)
        .ok_or_else(|| BehaviorError::SymbolNotFound {
            symbol: symbol_query.to_owned(),
        })?;
    let mut related_tests = symbols
        .iter()
        .filter(|symbol| symbol.kind == SymbolKind::TestCase)
        .filter_map(|test| related_test_for(target, test))
        .collect::<Vec<_>>();
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

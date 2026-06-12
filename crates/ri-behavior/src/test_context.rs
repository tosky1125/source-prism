use crate::BehaviorError;
use crate::{
    CoverageEvidenceSegment, ExecutionPolicy, RelatedCoverageSegment, RelatedTest, TestContext,
    TestCoverageEdge,
};
use ri_core::{Confidence, SymbolId, SymbolKind};
use ri_symbols::SymbolRecord;
use std::collections::{BTreeMap, BTreeSet};

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
    build_test_context_with_evidence(symbols, coverage_edges, &[], symbol_query)
}

pub fn build_test_context_with_evidence(
    symbols: &[SymbolRecord],
    coverage_edges: &[TestCoverageEdge],
    coverage_segments: &[CoverageEvidenceSegment],
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
    let graph_related_tests = coverage_edges
        .iter()
        .filter(|edge| edge.target_symbol_id == target.versioned_symbol_id)
        .filter_map(|edge| graph_related_test(edge, &symbols_by_id))
        .collect::<Vec<_>>();
    let related_coverage_segments = related_coverage_segments(target, coverage_segments);
    let coverage_available =
        !graph_related_tests.is_empty() || !related_coverage_segments.is_empty();
    let mut related_tests = graph_related_tests;
    for related in &related_tests {
        seen.insert(related.fqn.clone());
    }
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
        coverage_available,
        coverage_segments: related_coverage_segments,
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

fn related_coverage_segments(
    target: &SymbolRecord,
    segments: &[CoverageEvidenceSegment],
) -> Vec<RelatedCoverageSegment> {
    segments
        .iter()
        .filter(|segment| segment.file_path == target.file_path.as_str())
        .filter(|segment| {
            segment.end_line >= target.range.start_line
                && segment.start_line <= target.range.end_line
        })
        .map(|segment| RelatedCoverageSegment {
            file_path: segment.file_path.clone(),
            start_line: segment.start_line,
            end_line: segment.end_line,
            hit_count: segment.hit_count,
            format: segment.format.clone(),
            source_path: segment.source_path.clone(),
            evidence: "coverage range overlaps target symbol".to_owned(),
        })
        .collect()
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

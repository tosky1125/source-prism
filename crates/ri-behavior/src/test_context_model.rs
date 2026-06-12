use ri_core::{Confidence, Language, SymbolId};
use ri_symbols::SymbolRange;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct TestContext {
    pub symbol: String,
    pub code_execution_allowed: bool,
    pub execution_policy: ExecutionPolicy,
    pub coverage_available: bool,
    pub coverage_segments: Vec<RelatedCoverageSegment>,
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RelatedCoverageSegment {
    pub file_path: String,
    pub start_line: u32,
    pub end_line: u32,
    pub hit_count: u32,
    pub format: String,
    pub source_path: String,
    pub evidence: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct TestCoverageEdge {
    pub test_symbol_id: SymbolId,
    pub target_symbol_id: SymbolId,
    pub confidence: Confidence,
    pub evidence: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct CoverageEvidenceSegment {
    pub file_path: String,
    pub start_line: u32,
    pub end_line: u32,
    pub hit_count: u32,
    pub format: String,
    pub source_path: String,
}

impl CoverageEvidenceSegment {
    pub fn new(
        file_path: impl Into<String>,
        start_line: u32,
        end_line: u32,
        hit_count: u32,
        format: impl Into<String>,
        source_path: impl Into<String>,
    ) -> Self {
        Self {
            file_path: file_path.into(),
            start_line,
            end_line,
            hit_count,
            format: format.into(),
            source_path: source_path.into(),
        }
    }
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

#![allow(
    missing_docs,
    reason = "Behavior evidence contracts are self-describing at this milestone."
)]

mod cobertura;
mod go_test_json;
mod jacoco;
mod junit;
mod lcov;
mod playwright_json;
mod pytest_json;
mod test_context;
mod test_context_model;

use thiserror::Error;

pub use cobertura::parse_cobertura_xml;
pub use go_test_json::parse_go_test_json;
pub use jacoco::parse_jacoco_xml;
pub use junit::{JunitReport, TestCaseResult, TestResultStatus, TestSuiteResult, parse_junit_xml};
pub use lcov::{CoverageFile, CoverageReport, CoverageSegment, parse_lcov};
pub use playwright_json::parse_playwright_json;
pub use pytest_json::parse_pytest_json;
pub use test_context::{
    build_test_context, build_test_context_with_coverage, build_test_context_with_evidence,
};
pub use test_context_model::{
    CoverageEvidenceSegment, ExecutionPolicy, RelatedCoverageSegment, RelatedTest, TestContext,
    TestCoverageEdge,
};

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
    #[error("failed to parse jacoco xml: {message}")]
    JacocoXml { message: String },
    #[error("failed to parse pytest json: {message}")]
    PytestJson { message: String },
    #[error("failed to parse playwright json: {message}")]
    PlaywrightJson { message: String },
    #[error("failed to parse go test json: {message}")]
    GoTestJson { message: String },
}

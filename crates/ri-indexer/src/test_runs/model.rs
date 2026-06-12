use ri_behavior::{JunitReport, TestResultStatus};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum TestRunStoreError {
    #[error("index generation not found: {generation_id}")]
    GenerationNotFound { generation_id: String },
    #[error("invalid test run count value: {field}={value}")]
    InvalidCount { field: &'static str, value: u32 },
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct TestRunIngestOutcome {
    pub test_run_id: String,
    pub result_count: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct TestRunRecord {
    pub test_run_id: String,
    pub source_path: String,
    pub framework: String,
    pub status: String,
    pub total_count: i32,
    pub passed_count: i32,
    pub failed_count: i32,
    pub error_count: i32,
    pub skipped_count: i32,
    pub duration_ms: Option<i64>,
    pub results: Vec<TestResultRecord>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct TestResultRecord {
    pub test_result_id: String,
    pub suite_name: String,
    pub class_name: Option<String>,
    pub name: String,
    pub fqn: String,
    pub file_path: Option<String>,
    pub status: String,
    pub duration_ms: Option<i64>,
    pub message: Option<String>,
}

pub(super) fn run_status(report: &JunitReport) -> TestResultStatus {
    if report.error_count() > 0 {
        TestResultStatus::Error
    } else if report.failed_count() > 0 {
        TestResultStatus::Failed
    } else if report.skipped_count() == report.total_count() && report.total_count() > 0 {
        TestResultStatus::Skipped
    } else {
        TestResultStatus::Passed
    }
}

pub(super) const fn result_status(status: TestResultStatus) -> &'static str {
    match status {
        TestResultStatus::Passed => "passed",
        TestResultStatus::Failed => "failed",
        TestResultStatus::Skipped => "skipped",
        _ => "error",
    }
}

pub(super) fn count_value(value: u32, field: &'static str) -> Result<i32, TestRunStoreError> {
    i32::try_from(value).map_err(|_| TestRunStoreError::InvalidCount { field, value })
}

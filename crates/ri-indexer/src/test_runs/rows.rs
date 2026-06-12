use sqlx::Row as _;

use super::{TestResultRecord, TestRunRecord, TestRunStoreError};

#[derive(Debug)]
pub(super) struct TestResultWithRun {
    pub test_run_id: String,
    pub record: TestResultRecord,
}

pub(super) fn test_run_from_row(
    row: &sqlx::postgres::PgRow,
) -> Result<TestRunRecord, TestRunStoreError> {
    Ok(TestRunRecord {
        test_run_id: row.try_get("test_run_id")?,
        source_path: row.try_get("source_path")?,
        framework: row.try_get("framework")?,
        status: row.try_get("status")?,
        total_count: row.try_get("total_count")?,
        passed_count: row.try_get("passed_count")?,
        failed_count: row.try_get("failed_count")?,
        error_count: row.try_get("error_count")?,
        skipped_count: row.try_get("skipped_count")?,
        duration_ms: row.try_get("duration_ms")?,
        results: Vec::new(),
    })
}

pub(super) fn test_result_from_row(
    row: &sqlx::postgres::PgRow,
) -> Result<TestResultWithRun, TestRunStoreError> {
    Ok(TestResultWithRun {
        test_run_id: row.try_get("test_run_id")?,
        record: TestResultRecord {
            test_result_id: row.try_get("test_result_id")?,
            suite_name: row.try_get("suite_name")?,
            class_name: row.try_get("class_name")?,
            name: row.try_get("name")?,
            fqn: row.try_get("fqn")?,
            file_path: row.try_get("file_path")?,
            status: row.try_get("status")?,
            duration_ms: row.try_get("duration_ms")?,
            message: row.try_get("message")?,
        },
    })
}

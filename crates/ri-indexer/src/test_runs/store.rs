use std::collections::BTreeMap;

use ri_behavior::JunitReport;
use ri_core::GenerationId;
use sqlx::{PgPool, Row as _};

use super::{
    TestResultRecord, TestRunIngestOutcome, TestRunRecord, TestRunStoreError,
    ids::test_run_id,
    rows::{TestResultWithRun, test_result_from_row, test_run_from_row},
    write::{stale_previous_test_run, upsert_test_result, upsert_test_run},
};

#[derive(Debug, Clone)]
pub struct PgTestRunStore {
    pool: PgPool,
}

impl PgTestRunStore {
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn replace_junit_run_for_generation(
        &self,
        generation_id: &GenerationId,
        source_path: &str,
        report: &JunitReport,
    ) -> Result<TestRunIngestOutcome, TestRunStoreError> {
        self.replace_run_for_generation(generation_id, source_path, "junit", report)
            .await
    }

    pub async fn replace_pytest_run_for_generation(
        &self,
        generation_id: &GenerationId,
        source_path: &str,
        report: &JunitReport,
    ) -> Result<TestRunIngestOutcome, TestRunStoreError> {
        self.replace_run_for_generation(generation_id, source_path, "pytest", report)
            .await
    }

    pub async fn replace_playwright_run_for_generation(
        &self,
        generation_id: &GenerationId,
        source_path: &str,
        report: &JunitReport,
    ) -> Result<TestRunIngestOutcome, TestRunStoreError> {
        self.replace_run_for_generation(generation_id, source_path, "playwright", report)
            .await
    }

    pub async fn replace_go_test_run_for_generation(
        &self,
        generation_id: &GenerationId,
        source_path: &str,
        report: &JunitReport,
    ) -> Result<TestRunIngestOutcome, TestRunStoreError> {
        self.replace_run_for_generation(generation_id, source_path, "go_test", report)
            .await
    }

    async fn replace_run_for_generation(
        &self,
        generation_id: &GenerationId,
        source_path: &str,
        framework: &str,
        report: &JunitReport,
    ) -> Result<TestRunIngestOutcome, TestRunStoreError> {
        let generation = self.generation(generation_id).await?;
        let mut transaction = self.pool.begin().await?;
        stale_previous_test_run(
            &mut transaction,
            &generation,
            generation_id,
            source_path,
            framework,
        )
        .await?;
        let test_run_id = test_run_id(&generation, generation_id, source_path, framework);
        upsert_test_run(
            &mut transaction,
            &generation,
            generation_id,
            source_path,
            framework,
            report,
        )
        .await?;
        let mut result_count = 0_u64;
        for result in report.results() {
            let inserted = upsert_test_result(
                &mut transaction,
                &generation,
                generation_id,
                &test_run_id,
                result,
            )
            .await?;
            result_count = result_count.saturating_add(inserted);
        }
        transaction.commit().await?;
        Ok(TestRunIngestOutcome {
            test_run_id,
            result_count,
        })
    }

    pub async fn active_test_runs_for_repo(
        &self,
        repo_id: &str,
    ) -> Result<Vec<TestRunRecord>, TestRunStoreError> {
        let run_rows = sqlx::query(
            r"
            SELECT test_run_id, source_path, framework, status, total_count, passed_count,
                   failed_count, error_count, skipped_count, duration_ms
            FROM test_runs
            WHERE repo_id = $1 AND stale_at IS NULL
            ORDER BY created_at DESC, source_path
            ",
        )
        .bind(repo_id)
        .fetch_all(&self.pool)
        .await?;
        let mut runs = run_rows
            .iter()
            .map(test_run_from_row)
            .collect::<Result<Vec<_>, _>>()?;
        let results = self.active_results_for_repo(repo_id).await?;
        let mut by_run = results.into_iter().fold(
            BTreeMap::<String, Vec<TestResultRecord>>::new(),
            |mut grouped, result| {
                grouped
                    .entry(result.test_run_id)
                    .or_default()
                    .push(result.record);
                grouped
            },
        );
        for run in &mut runs {
            run.results = by_run.remove(&run.test_run_id).unwrap_or_default();
        }
        Ok(runs)
    }

    async fn active_results_for_repo(
        &self,
        repo_id: &str,
    ) -> Result<Vec<TestResultWithRun>, TestRunStoreError> {
        let rows = sqlx::query(
            r"
            SELECT test_result_id, test_run_id, suite_name, class_name, name, fqn,
                   file_path, status, duration_ms, message
            FROM test_results
            WHERE repo_id = $1 AND stale_at IS NULL
            ORDER BY suite_name, fqn
            ",
        )
        .bind(repo_id)
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(test_result_from_row).collect()
    }

    async fn generation(
        &self,
        generation_id: &GenerationId,
    ) -> Result<StoredGeneration, TestRunStoreError> {
        let row = sqlx::query(
            r"
            SELECT repo_id, commit_sha
            FROM index_generations
            WHERE generation_id = $1
            ",
        )
        .bind(generation_id.to_string())
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| TestRunStoreError::GenerationNotFound {
            generation_id: generation_id.to_string(),
        })?;
        Ok(StoredGeneration {
            repo_id: row.try_get("repo_id")?,
            commit_sha: row.try_get("commit_sha")?,
        })
    }
}

#[derive(Debug)]
pub(super) struct StoredGeneration {
    pub repo_id: String,
    pub commit_sha: String,
}

use ri_behavior::{JunitReport, TestCaseResult};
use ri_core::GenerationId;

use super::{
    TestRunStoreError,
    ids::{test_result_id, test_run_id},
    model::{count_value, result_status, run_status},
    store::StoredGeneration,
};

pub(super) async fn stale_previous_test_run(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    generation: &StoredGeneration,
    generation_id: &GenerationId,
    source_path: &str,
) -> Result<(), TestRunStoreError> {
    sqlx::query(
        r"
        UPDATE test_results
        SET stale_at = now()
        WHERE repo_id = $1 AND commit_sha = $2 AND generation_id <> $3
          AND test_run_id IN (
              SELECT test_run_id FROM test_runs
              WHERE repo_id = $1 AND commit_sha = $2 AND source_path = $4 AND stale_at IS NULL
          )
        ",
    )
    .bind(&generation.repo_id)
    .bind(&generation.commit_sha)
    .bind(generation_id.to_string())
    .bind(source_path)
    .execute(&mut **transaction)
    .await?;
    sqlx::query(
        r"
        UPDATE test_runs
        SET stale_at = now()
        WHERE repo_id = $1 AND commit_sha = $2 AND source_path = $3
          AND generation_id <> $4 AND stale_at IS NULL
        ",
    )
    .bind(&generation.repo_id)
    .bind(&generation.commit_sha)
    .bind(source_path)
    .bind(generation_id.to_string())
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

pub(super) async fn upsert_test_run(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    generation: &StoredGeneration,
    generation_id: &GenerationId,
    source_path: &str,
    report: &JunitReport,
) -> Result<(), TestRunStoreError> {
    sqlx::query(
        r"
        INSERT INTO test_runs (
            test_run_id, repo_id, commit_sha, generation_id, source_path, framework, status,
            total_count, passed_count, failed_count, error_count, skipped_count, duration_ms,
            stale_at
        )
        VALUES ($1, $2, $3, $4, $5, 'junit', $6, $7, $8, $9, $10, $11, NULL, NULL)
        ON CONFLICT (test_run_id) DO UPDATE
        SET generation_id = EXCLUDED.generation_id,
            status = EXCLUDED.status,
            total_count = EXCLUDED.total_count,
            passed_count = EXCLUDED.passed_count,
            failed_count = EXCLUDED.failed_count,
            error_count = EXCLUDED.error_count,
            skipped_count = EXCLUDED.skipped_count,
            stale_at = NULL
        ",
    )
    .bind(test_run_id(generation, generation_id, source_path))
    .bind(&generation.repo_id)
    .bind(&generation.commit_sha)
    .bind(generation_id.to_string())
    .bind(source_path)
    .bind(result_status(run_status(report)))
    .bind(count_value(report.total_count(), "total_count")?)
    .bind(count_value(report.passed_count(), "passed_count")?)
    .bind(count_value(report.failed_count(), "failed_count")?)
    .bind(count_value(report.error_count(), "error_count")?)
    .bind(count_value(report.skipped_count(), "skipped_count")?)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

pub(super) async fn upsert_test_result(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    generation: &StoredGeneration,
    generation_id: &GenerationId,
    test_run_id: &str,
    result: &TestCaseResult,
) -> Result<u64, TestRunStoreError> {
    let query_result = sqlx::query(
        r"
        INSERT INTO test_results (
            test_result_id, test_run_id, repo_id, commit_sha, generation_id, suite_name,
            class_name, name, fqn, file_path, status, duration_ms, message, stale_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, NULL)
        ON CONFLICT (test_result_id) DO UPDATE
        SET generation_id = EXCLUDED.generation_id,
            suite_name = EXCLUDED.suite_name,
            class_name = EXCLUDED.class_name,
            name = EXCLUDED.name,
            fqn = EXCLUDED.fqn,
            file_path = EXCLUDED.file_path,
            status = EXCLUDED.status,
            duration_ms = EXCLUDED.duration_ms,
            message = EXCLUDED.message,
            stale_at = NULL
        ",
    )
    .bind(test_result_id(test_run_id, result))
    .bind(test_run_id)
    .bind(&generation.repo_id)
    .bind(&generation.commit_sha)
    .bind(generation_id.to_string())
    .bind(&result.suite_name)
    .bind(&result.class_name)
    .bind(&result.name)
    .bind(&result.fqn)
    .bind(&result.file_path)
    .bind(result_status(result.status))
    .bind(result.duration_ms)
    .bind(&result.message)
    .execute(&mut **transaction)
    .await?;
    Ok(query_result.rows_affected())
}

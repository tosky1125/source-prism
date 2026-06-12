#![allow(missing_docs, reason = "Integration test names document behavior.")]

use sqlx::{PgPool, Row as _};
use std::time::{SystemTime, UNIX_EPOCH};

#[tokio::test]
async fn deleting_generation_cascades_search_sync_jobs() -> Result<(), Box<dyn std::error::Error>> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };
    let repo_id = format!("api-job-cascade-{}", unique_suffix()?);
    let commit_sha = format!("commit-{}", unique_suffix()?);
    let generation_id = format!("generation-{}", unique_suffix()?);
    let job_id = format!("job-{}", unique_suffix()?);
    let mut tx = pool.begin().await?;

    sqlx::query("INSERT INTO repos (repo_id, name, default_branch) VALUES ($1, $1, 'main')")
        .bind(&repo_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO commits (repo_id, commit_sha) VALUES ($1, $2)")
        .bind(&repo_id)
        .bind(&commit_sha)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        r"
        INSERT INTO index_generations (
            generation_id, repo_id, commit_sha, index_kind, status
        )
        VALUES ($1, $2, $3, 'file_manifest', 'succeeded')
        ",
    )
    .bind(&generation_id)
    .bind(&repo_id)
    .bind(&commit_sha)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        r"
        INSERT INTO jobs (
            job_id, queue, kind, state, generation_id, payload
        )
        VALUES ($1, 'test', 'search.sync_once', 'queued', $2, '{}'::jsonb)
        ",
    )
    .bind(&job_id)
    .bind(&generation_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        r"
        INSERT INTO job_attempts (job_id, attempt_no, worker_id, status)
        VALUES ($1, 1, 'test-worker', 'started')
        ",
    )
    .bind(&job_id)
    .execute(&mut *tx)
    .await?;

    sqlx::query("DELETE FROM index_generations WHERE generation_id = $1")
        .bind(&generation_id)
        .execute(&mut *tx)
        .await?;

    assert_eq!(job_count(&mut tx, &job_id).await?, 0);
    assert_eq!(job_attempt_count(&mut tx, &job_id).await?, 0);
    tx.rollback().await?;
    Ok(())
}

async fn test_pool() -> Result<Option<PgPool>, sqlx::Error> {
    let Ok(database_url) = std::env::var("DATABASE_URL") else {
        return Ok(None);
    };
    PgPool::connect(database_url.as_str()).await.map(Some)
}

fn unique_suffix() -> Result<String, std::time::SystemTimeError> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_nanos()
        .to_string())
}

async fn job_count(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    job_id: &str,
) -> Result<i64, sqlx::Error> {
    let row = sqlx::query("SELECT count(*)::bigint AS count FROM jobs WHERE job_id = $1")
        .bind(job_id)
        .fetch_one(&mut **tx)
        .await?;
    row.try_get("count")
}

async fn job_attempt_count(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    job_id: &str,
) -> Result<i64, sqlx::Error> {
    let row = sqlx::query("SELECT count(*)::bigint AS count FROM job_attempts WHERE job_id = $1")
        .bind(job_id)
        .fetch_one(&mut **tx)
        .await?;
    row.try_get("count")
}

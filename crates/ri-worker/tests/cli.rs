#![allow(
    missing_docs,
    reason = "Worker CLI integration test names document the surface."
)]

use std::{
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use ri_indexer::{OpenSearchClient, PgSearchSyncStore, SearchSyncInput};
use ri_worker::{
    EnqueueJob, JobKind, JobQueue, JobRuntime, JobState, LeaseConfig, PgJobStore, WorkerId,
};
use serde_json::json;
use sqlx::{PgPool, Row as _};

#[tokio::test]
async fn daemon_mode_processes_bounded_polls() -> Result<(), Box<dyn std::error::Error>> {
    let Some(database_url) = database_url() else {
        return Ok(());
    };
    let pool = PgPool::connect(database_url.as_str()).await?;
    let queue = format!("daemon-{}", unique_suffix()?);
    let store = PgJobStore::new(pool.clone(), JobQueue::parse(&queue)?);
    let runtime = JobRuntime::new(
        store,
        WorkerId::parse("worker-cli-test")?,
        LeaseConfig::for_tests(std::time::Duration::from_secs(30)),
    );
    let request = EnqueueJob::new(JobQueue::parse(&queue)?, JobKind::parse("noop")?, json!({}))
        .with_idempotency_key("daemon-noop");
    let job = runtime.enqueue(request).await?;

    let output = Command::new(env!("CARGO_BIN_EXE_ri-worker"))
        .env("DATABASE_URL", database_url.as_str())
        .env("RI_WORKER_ID", "worker-cli-test")
        .args([
            "--queue",
            queue.as_str(),
            "--max-polls",
            "2",
            "--poll-interval-ms",
            "1",
        ])
        .output()?;

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("ri-worker daemon polls=2 processed=1"));
    assert_eq!(
        job_state(&pool, &job.job_id.to_string()).await?,
        JobState::Succeeded
    );
    cleanup(&pool, &queue).await?;
    Ok(())
}

#[tokio::test]
async fn once_mode_can_enqueue_and_process_noop_job() -> Result<(), Box<dyn std::error::Error>> {
    let Some(database_url) = database_url() else {
        return Ok(());
    };
    let pool = PgPool::connect(database_url.as_str()).await?;
    let queue = format!("enqueue-once-{}", unique_suffix()?);

    let output = Command::new(env!("CARGO_BIN_EXE_ri-worker"))
        .env("DATABASE_URL", database_url.as_str())
        .env("RI_WORKER_ID", "worker-cli-test")
        .args(["--queue", queue.as_str(), "--enqueue-noop", "--once"])
        .output()?;

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("ri-worker once processed=1 job_id="));
    assert_eq!(succeeded_jobs(&pool, &queue).await?, 1);
    assert_eq!(finished_attempts(&pool, &queue).await?, 1);
    cleanup(&pool, &queue).await?;
    Ok(())
}

#[tokio::test]
async fn once_mode_can_process_search_sync_job() -> Result<(), Box<dyn std::error::Error>> {
    let Some(database_url) = database_url() else {
        return Ok(());
    };
    let Some(opensearch_url) = opensearch_url() else {
        return Ok(());
    };
    let pool = PgPool::connect(database_url.as_str()).await?;
    let queue = format!("search-sync-{}", unique_suffix()?);
    let repo_id = format!("repo-{queue}");
    let index = format!("source-prism-worker-{queue}");
    seed_repo(&pool, &repo_id).await?;
    OpenSearchClient::new(&opensearch_url)
        .delete_index_if_exists(&index)
        .await?;
    let outbox = SearchSyncInput::upsert(
        &repo_id,
        "symbol_chunk",
        "chunk-1",
        &index,
        json!({ "chunk_id": "chunk-1", "text": "worker search sync" }),
    );
    let outbox_record = PgSearchSyncStore::new(pool.clone())
        .enqueue(&outbox)
        .await?;

    let output = Command::new(env!("CARGO_BIN_EXE_ri-worker"))
        .env("DATABASE_URL", database_url.as_str())
        .env("OPENSEARCH_URL", opensearch_url.as_str())
        .env("RI_WORKER_ID", "worker-cli-test")
        .args([
            "--queue",
            queue.as_str(),
            "--enqueue-search-sync",
            "--search-outbox-id",
            outbox_record.outbox_id.as_str(),
            "--once",
        ])
        .output()?;

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("ri-worker once processed=1 job_id="));
    assert_eq!(succeeded_jobs(&pool, &queue).await?, 1);
    assert_eq!(succeeded_search_outbox(&pool, &repo_id).await?, 1);
    assert_eq!(
        OpenSearchClient::new(&opensearch_url)
            .count_documents(&index)
            .await?,
        1
    );
    cleanup(&pool, &queue).await?;
    cleanup_search(&pool, &repo_id, &index, &opensearch_url).await?;
    Ok(())
}

fn database_url() -> Option<String> {
    std::env::var("DATABASE_URL").ok()
}

fn opensearch_url() -> Option<String> {
    std::env::var("OPENSEARCH_URL").ok()
}

fn unique_suffix() -> Result<String, std::time::SystemTimeError> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_nanos()
        .to_string())
}

async fn job_state(pool: &PgPool, job_id: &str) -> Result<JobState, Box<dyn std::error::Error>> {
    let row = sqlx::query("SELECT state FROM jobs WHERE job_id = $1")
        .bind(job_id)
        .fetch_one(pool)
        .await?;
    Ok(JobState::parse(
        row.try_get::<String, _>("state")?.as_str(),
    )?)
}

async fn succeeded_jobs(pool: &PgPool, queue: &str) -> Result<i64, sqlx::Error> {
    let row = sqlx::query(
        r"
        SELECT count(*)::bigint AS count
        FROM jobs
        WHERE queue = $1 AND state = 'succeeded'
        ",
    )
    .bind(queue)
    .fetch_one(pool)
    .await?;
    row.try_get("count")
}

async fn finished_attempts(pool: &PgPool, queue: &str) -> Result<i64, sqlx::Error> {
    let row = sqlx::query(
        r"
        SELECT count(*)::bigint AS count
        FROM job_attempts
        WHERE job_id IN (SELECT job_id FROM jobs WHERE queue = $1)
          AND status = 'succeeded'
          AND finished_at IS NOT NULL
        ",
    )
    .bind(queue)
    .fetch_one(pool)
    .await?;
    row.try_get("count")
}

async fn succeeded_search_outbox(pool: &PgPool, repo_id: &str) -> Result<i64, sqlx::Error> {
    let row = sqlx::query(
        r"
        SELECT count(*)::bigint AS count
        FROM search_sync_outbox
        WHERE repo_id = $1
          AND operation = $2
          AND state = 'succeeded'
          AND processed_at IS NOT NULL
        ",
    )
    .bind(repo_id)
    .bind("upsert")
    .fetch_one(pool)
    .await?;
    row.try_get("count")
}

async fn seed_repo(pool: &PgPool, repo_id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO repos (repo_id, name) VALUES ($1, $1)")
        .bind(repo_id)
        .execute(pool)
        .await?;
    Ok(())
}

async fn cleanup(pool: &PgPool, queue: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        r"
        DELETE FROM job_attempts
        WHERE job_id IN (SELECT job_id FROM jobs WHERE queue = $1)
        ",
    )
    .bind(queue)
    .execute(pool)
    .await?;
    sqlx::query("DELETE FROM jobs WHERE queue = $1")
        .bind(queue)
        .execute(pool)
        .await?;
    Ok(())
}

async fn cleanup_search(
    pool: &PgPool,
    repo_id: &str,
    index: &str,
    opensearch_url: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    sqlx::query("DELETE FROM search_sync_outbox WHERE repo_id = $1")
        .bind(repo_id)
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM repos WHERE repo_id = $1")
        .bind(repo_id)
        .execute(pool)
        .await?;
    OpenSearchClient::new(opensearch_url)
        .delete_index_if_exists(index)
        .await?;
    Ok(())
}

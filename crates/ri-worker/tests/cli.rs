#![allow(
    missing_docs,
    reason = "Worker CLI integration test names document the surface."
)]

use std::{
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

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

fn database_url() -> Option<String> {
    std::env::var("DATABASE_URL").ok()
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

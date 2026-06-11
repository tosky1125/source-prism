#![allow(
    missing_docs,
    reason = "Integration tests use BDD names and Given/When/Then comments instead of API docs."
)]
#![allow(
    clippy::multiple_crate_versions,
    reason = "SQLx TLS dependencies currently pull duplicate platform crates outside this crate's control."
)]

use std::time::Duration;

use ri_worker::{
    Backoff, EnqueueJob, JobKind, JobQueue, JobRuntime, JobState, LeaseConfig, MemoryJobStore,
    WorkerId,
};
use serde_json::json;

#[tokio::test]
async fn lease_is_unique_when_two_workers_race() -> Result<(), Box<dyn std::error::Error>> {
    // Given: one ready job and two worker runtimes.
    let store = MemoryJobStore::default();
    let runtime_a = JobRuntime::new(
        store.clone(),
        WorkerId::parse("worker-a")?,
        LeaseConfig::for_tests(Duration::from_secs(30)),
    );
    let runtime_b = JobRuntime::new(
        store.clone(),
        WorkerId::parse("worker-b")?,
        LeaseConfig::for_tests(Duration::from_secs(30)),
    );
    let request = EnqueueJob::new(JobQueue::default(), JobKind::parse("noop")?, json!({}))
        .with_idempotency_key("unique-lease");
    runtime_a.enqueue(request).await?;

    // When: both workers attempt to lease work.
    let leased_a = runtime_a.lease_next().await?;
    let leased_b = runtime_b.lease_next().await?;

    // Then: only one worker receives the job.
    let leased_count = usize::from(leased_a.is_some()) + usize::from(leased_b.is_some());
    assert_eq!(leased_count, 1);
    Ok(())
}

#[tokio::test]
async fn retry_dead_letters_after_max_attempts() -> Result<(), Box<dyn std::error::Error>> {
    // Given: a job with two allowed attempts.
    let store = MemoryJobStore::default();
    let runtime = JobRuntime::new(
        store.clone(),
        WorkerId::parse("worker-a")?,
        LeaseConfig::for_tests(Duration::from_secs(30)),
    );
    let request = EnqueueJob::new(JobQueue::default(), JobKind::parse("noop")?, json!({}))
        .with_idempotency_key("retry-dead-letter")
        .with_max_attempts(2)
        .with_backoff(Backoff::fixed(Duration::from_secs(5)));
    let job = runtime.enqueue(request).await?;

    // When: the job fails twice.
    let first = runtime.require_lease().await?;
    runtime.fail(first.lease, "first failure").await?;
    let failed = store.get(job.job_id)?;
    assert_eq!(failed.state, JobState::Failed);
    store.advance_by(Duration::from_secs(5))?;
    let second = runtime.require_lease().await?;
    runtime.fail(second.lease, "second failure").await?;

    // Then: the job is dead-lettered.
    let stored = store.get(job.job_id)?;
    assert_eq!(stored.state, JobState::DeadLettered);
    assert_eq!(stored.attempt_count, 2);
    Ok(())
}

#[tokio::test]
async fn cancelled_job_is_not_leased() -> Result<(), Box<dyn std::error::Error>> {
    // Given: a queued job.
    let store = MemoryJobStore::default();
    let runtime = JobRuntime::new(
        store.clone(),
        WorkerId::parse("worker-a")?,
        LeaseConfig::for_tests(Duration::from_secs(30)),
    );
    let request = EnqueueJob::new(JobQueue::default(), JobKind::parse("noop")?, json!({}))
        .with_idempotency_key("cancelled");
    let job = runtime.enqueue(request).await?;

    // When: the job is cancelled before leasing.
    runtime.cancel(job.job_id).await?;
    let leased = runtime.lease_next().await?;

    // Then: no worker receives it and the state is cancelled.
    assert!(leased.is_none());
    let stored = store.get(job.job_id)?;
    assert_eq!(stored.state, JobState::Cancelled);
    Ok(())
}

#[tokio::test]
async fn enqueue_is_idempotent_for_same_key() -> Result<(), Box<dyn std::error::Error>> {
    // Given: two enqueue requests with the same queue, kind, and idempotency key.
    let store = MemoryJobStore::default();
    let runtime = JobRuntime::new(
        store.clone(),
        WorkerId::parse("worker-a")?,
        LeaseConfig::for_tests(Duration::from_secs(30)),
    );
    let first = EnqueueJob::new(
        JobQueue::default(),
        JobKind::parse("noop")?,
        json!({ "n": 1 }),
    )
    .with_idempotency_key("same-key");
    let second = EnqueueJob::new(
        JobQueue::default(),
        JobKind::parse("noop")?,
        json!({ "n": 2 }),
    )
    .with_idempotency_key("same-key");

    // When: both requests are enqueued.
    let first_job = runtime.enqueue(first).await?;
    let second_job = runtime.enqueue(second).await?;

    // Then: both calls return the same durable job row.
    assert_eq!(first_job.job_id, second_job.job_id);
    assert_eq!(store.len(), 1);
    Ok(())
}

use async_trait::async_trait;
use sqlx::{PgPool, Row as _};
use std::time::Duration;

use crate::model::{
    EnqueueJob, JobId, JobKind, JobLease, JobQueue, JobRecord, JobState, LeasedJob, WorkerId,
};
use crate::runtime::{JobError, JobStore};

#[derive(Debug, Clone)]
pub struct PgJobStore {
    pool: PgPool,
    queue: JobQueue,
}

impl PgJobStore {
    pub const fn new(pool: PgPool, queue: JobQueue) -> Self {
        Self { pool, queue }
    }

    pub(crate) const fn pool(&self) -> &PgPool {
        &self.pool
    }
}

#[async_trait]
impl JobStore for PgJobStore {
    async fn enqueue(&self, request: EnqueueJob) -> Result<JobRecord, JobError> {
        let job_id = JobId::new();
        let backoff_seconds = seconds_i64(request.backoff.delay())?;
        let row = sqlx::query(
            r"
            INSERT INTO jobs (
                job_id, queue, kind, state, idempotency_key, payload, priority,
                run_after, attempt_count, max_attempts, metadata
            )
            VALUES ($1, $2, $3, 'queued', $4, $5, $6, now(), 0, $7, jsonb_build_object('backoff_seconds', $8::bigint))
            ON CONFLICT (queue, kind, idempotency_key)
                WHERE idempotency_key IS NOT NULL
            DO UPDATE SET updated_at = jobs.updated_at
            RETURNING job_id, state, attempt_count
            ",
        )
        .bind(job_id.to_string())
        .bind(request.queue.to_string())
        .bind(request.kind.to_string())
        .bind(request.idempotency_key)
        .bind(request.payload)
        .bind(request.priority)
        .bind(request.max_attempts)
        .bind(backoff_seconds)
        .fetch_one(&self.pool)
        .await?;
        record_from_row(&row)
    }

    async fn lease_next(
        &self,
        worker_id: &WorkerId,
        lease_timeout: Duration,
    ) -> Result<Option<LeasedJob>, JobError> {
        let lease_seconds = seconds_i64(lease_timeout)?;
        let mut transaction = self.pool.begin().await?;
        let row = sqlx::query(
            r"
            WITH candidate AS (
                SELECT job_id
                FROM jobs
                WHERE queue = $1
                  AND (
                    (state IN ('queued', 'failed') AND run_after <= now())
                    OR (state = 'leased' AND leased_until <= now())
                  )
                ORDER BY priority DESC, run_after ASC, created_at ASC
                FOR UPDATE SKIP LOCKED
                LIMIT 1
            )
            UPDATE jobs AS j
            SET state = 'leased',
                leased_by = $2,
                leased_until = now() + ($3 * INTERVAL '1 second'),
                attempt_count = attempt_count + 1,
                updated_at = now()
            FROM candidate
            WHERE j.job_id = candidate.job_id
            RETURNING j.job_id, j.kind, j.payload, j.attempt_count
            ",
        )
        .bind(self.queue.to_string())
        .bind(worker_id.to_string())
        .bind(lease_seconds)
        .fetch_optional(&mut *transaction)
        .await?;
        let Some(row) = row else {
            transaction.commit().await?;
            return Ok(None);
        };
        let job_id = JobId::parse(row.try_get::<String, _>("job_id")?.as_str())?;
        let attempt_no = row.try_get::<i32, _>("attempt_count")?;
        sqlx::query(
            r"
            INSERT INTO job_attempts (job_id, attempt_no, worker_id, status)
            VALUES ($1, $2, $3, 'started')
            ON CONFLICT (job_id, attempt_no) DO NOTHING
            ",
        )
        .bind(job_id.to_string())
        .bind(attempt_no)
        .bind(worker_id.to_string())
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(Some(LeasedJob {
            lease: JobLease { job_id, attempt_no },
            kind: JobKind::parse(row.try_get::<String, _>("kind")?.as_str())?,
            payload: row.try_get("payload")?,
        }))
    }

    async fn succeed(&self, lease: JobLease) -> Result<(), JobError> {
        sqlx::query(
            r"
            UPDATE jobs
            SET state = 'succeeded',
                leased_by = NULL,
                leased_until = NULL,
                completed_at = now(),
                updated_at = now()
            WHERE job_id = $1 AND attempt_count = $2
            ",
        )
        .bind(lease.job_id.to_string())
        .bind(lease.attempt_no)
        .execute(&self.pool)
        .await?;
        finish_attempt(&self.pool, lease, "succeeded", None).await
    }

    async fn fail(&self, lease: JobLease, error: &str) -> Result<(), JobError> {
        sqlx::query(
            r"
            UPDATE jobs
            SET state = CASE
                    WHEN attempt_count >= max_attempts THEN 'dead_lettered'
                    ELSE 'failed'
                END,
                run_after = CASE
                    WHEN attempt_count >= max_attempts THEN run_after
                    ELSE now() + (((metadata->>'backoff_seconds')::bigint) * INTERVAL '1 second')
                END,
                leased_by = NULL,
                leased_until = NULL,
                last_error = $3,
                completed_at = CASE WHEN attempt_count >= max_attempts THEN now() ELSE completed_at END,
                updated_at = now()
            WHERE job_id = $1 AND attempt_count = $2
            ",
        )
        .bind(lease.job_id.to_string())
        .bind(lease.attempt_no)
        .bind(error)
        .execute(&self.pool)
        .await?;
        finish_attempt(&self.pool, lease, "failed", Some(error)).await
    }

    async fn cancel(&self, job_id: JobId) -> Result<(), JobError> {
        sqlx::query(
            r"
            UPDATE jobs
            SET state = 'cancelled',
                leased_by = NULL,
                leased_until = NULL,
                completed_at = now(),
                updated_at = now()
            WHERE job_id = $1 AND state IN ('queued', 'leased', 'failed')
            ",
        )
        .bind(job_id.to_string())
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

async fn finish_attempt(
    pool: &PgPool,
    lease: JobLease,
    status: &str,
    error: Option<&str>,
) -> Result<(), JobError> {
    sqlx::query(
        r"
        UPDATE job_attempts
        SET status = $3, error = $4, finished_at = now()
        WHERE job_id = $1 AND attempt_no = $2
        ",
    )
    .bind(lease.job_id.to_string())
    .bind(lease.attempt_no)
    .bind(status)
    .bind(error)
    .execute(pool)
    .await?;
    Ok(())
}

fn record_from_row(row: &sqlx::postgres::PgRow) -> Result<JobRecord, JobError> {
    Ok(JobRecord {
        job_id: JobId::parse(row.try_get::<String, _>("job_id")?.as_str())?,
        state: JobState::parse(row.try_get::<String, _>("state")?.as_str())?,
        attempt_count: row.try_get("attempt_count")?,
    })
}

fn seconds_i64(duration: Duration) -> Result<i64, JobError> {
    i64::try_from(duration.as_secs()).map_err(|_| JobError::DurationTooLarge)
}

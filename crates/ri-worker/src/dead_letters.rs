use sqlx::{PgPool, Row as _};

use crate::{
    model::{DeadLetterAttempt, DeadLetterJob},
    pg::PgJobStore,
    runtime::JobError,
};

impl PgJobStore {
    pub async fn dead_letters_for_repo(
        &self,
        repo_id: &str,
        limit: i64,
    ) -> Result<Vec<DeadLetterJob>, JobError> {
        dead_letters_for_repo(self.pool(), repo_id, limit).await
    }
}

async fn dead_letters_for_repo(
    pool: &PgPool,
    repo_id: &str,
    limit: i64,
) -> Result<Vec<DeadLetterJob>, JobError> {
    let rows = sqlx::query(
        r"
        SELECT
            j.job_id,
            j.queue,
            j.kind,
            j.generation_id,
            j.attempt_count,
            j.max_attempts,
            j.last_error,
            j.created_at::text AS created_at,
            j.updated_at::text AS updated_at,
            j.completed_at::text AS completed_at
        FROM jobs AS j
        JOIN index_generations AS g ON g.generation_id = j.generation_id
        WHERE g.repo_id = $1
          AND j.state = 'dead_lettered'
        ORDER BY j.completed_at DESC NULLS LAST, j.updated_at DESC, j.job_id ASC
        LIMIT $2
        ",
    )
    .bind(repo_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    let mut jobs = Vec::with_capacity(rows.len());
    for row in rows {
        let job_id = row.try_get::<String, _>("job_id")?;
        jobs.push(DeadLetterJob {
            job_id: job_id.clone(),
            queue: row.try_get("queue")?,
            kind: row.try_get("kind")?,
            generation_id: row.try_get("generation_id")?,
            attempt_count: row.try_get("attempt_count")?,
            max_attempts: row.try_get("max_attempts")?,
            last_error: row.try_get("last_error")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
            completed_at: row.try_get("completed_at")?,
            attempts: dead_letter_attempts(pool, job_id.as_str()).await?,
        });
    }
    Ok(jobs)
}

async fn dead_letter_attempts(
    pool: &PgPool,
    job_id: &str,
) -> Result<Vec<DeadLetterAttempt>, JobError> {
    let rows = sqlx::query(
        r"
        SELECT
            attempt_no,
            worker_id,
            status,
            error,
            started_at::text AS started_at,
            finished_at::text AS finished_at
        FROM job_attempts
        WHERE job_id = $1
        ORDER BY attempt_no ASC
        ",
    )
    .bind(job_id)
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|row| {
            Ok(DeadLetterAttempt {
                attempt_no: row.try_get("attempt_no")?,
                worker_id: row.try_get("worker_id")?,
                status: row.try_get("status")?,
                error: row.try_get("error")?,
                started_at: row.try_get("started_at")?,
                finished_at: row.try_get("finished_at")?,
            })
        })
        .collect()
}

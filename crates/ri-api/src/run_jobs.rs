use serde::Serialize;
use sqlx::{PgPool, Row as _};

#[derive(Debug, Serialize)]
pub(crate) struct RunSearchSyncJob {
    pub(crate) job_id: String,
    pub(crate) state: String,
    pub(crate) attempt_count: i32,
    pub(crate) attempts: Vec<RunSearchSyncJobAttempt>,
}

#[derive(Debug, Serialize)]
pub(crate) struct RunSearchSyncJobAttempt {
    pub(crate) attempt_no: i32,
    pub(crate) worker_id: String,
    pub(crate) status: String,
    pub(crate) error: Option<String>,
    pub(crate) started_at: String,
    pub(crate) finished_at: Option<String>,
}

pub(crate) async fn find_search_sync_jobs(
    pool: &PgPool,
    generation_id: &str,
) -> Result<Vec<RunSearchSyncJob>, sqlx::Error> {
    let rows = sqlx::query(
        r"
        SELECT job_id, state, attempt_count
        FROM jobs
        WHERE generation_id = $1
          AND kind = 'search.sync_once'
        ORDER BY created_at ASC
        ",
    )
    .bind(generation_id)
    .fetch_all(pool)
    .await?;

    let mut jobs = Vec::with_capacity(rows.len());
    for row in rows {
        let job_id = row.try_get::<String, _>("job_id")?;
        jobs.push(RunSearchSyncJob {
            attempts: find_job_attempts(pool, &job_id).await?,
            job_id,
            state: row.try_get("state")?,
            attempt_count: row.try_get("attempt_count")?,
        });
    }
    Ok(jobs)
}

async fn find_job_attempts(
    pool: &PgPool,
    job_id: &str,
) -> Result<Vec<RunSearchSyncJobAttempt>, sqlx::Error> {
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
            Ok(RunSearchSyncJobAttempt {
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

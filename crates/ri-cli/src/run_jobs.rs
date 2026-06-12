#![allow(
    clippy::redundant_pub_crate,
    reason = "Binary crate helper modules share crate-visible command handlers."
)]

use serde_json::json;
use sqlx::{PgPool, Row as _};

use crate::CliError;

pub(crate) async fn search_sync_jobs(
    pool: &PgPool,
    generation_id: &str,
) -> Result<Vec<serde_json::Value>, CliError> {
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
        jobs.push(json!({
            "job_id": job_id,
            "state": row.try_get::<String, _>("state")?,
            "attempt_count": row.try_get::<i32, _>("attempt_count")?,
            "attempts": job_attempts(pool, job_id.as_str()).await?,
        }));
    }
    Ok(jobs)
}

async fn job_attempts(pool: &PgPool, job_id: &str) -> Result<Vec<serde_json::Value>, CliError> {
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
            Ok(json!({
                "attempt_no": row.try_get::<i32, _>("attempt_no")?,
                "worker_id": row.try_get::<String, _>("worker_id")?,
                "status": row.try_get::<String, _>("status")?,
                "error": row.try_get::<Option<String>, _>("error")?,
                "started_at": row.try_get::<String, _>("started_at")?,
                "finished_at": row.try_get::<Option<String>, _>("finished_at")?,
            }))
        })
        .collect()
}

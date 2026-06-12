use sqlx::{PgPool, Row as _};
use uuid::Uuid;

use crate::AppError;

pub(crate) const DEFAULT_SEARCH_SYNC_QUEUE: &str = "default";

pub(crate) fn search_sync_queue(raw: Option<&str>) -> Result<String, AppError> {
    let Some(raw) = raw else {
        return Ok(DEFAULT_SEARCH_SYNC_QUEUE.to_owned());
    };
    let queue = raw.trim();
    if queue.is_empty() {
        return Err(AppError::Validation(
            "search_sync_queue must not be empty".to_owned(),
        ));
    }
    Ok(queue.to_owned())
}

pub(crate) async fn enqueue_search_sync_job(
    pool: &PgPool,
    repo_id: &str,
    generation_id: &str,
    queue: &str,
) -> Result<u64, sqlx::Error> {
    let job_id = Uuid::now_v7().to_string();
    let payload = serde_json::json!({
        "source": "ri-api-index",
        "repo_id": repo_id,
        "generation_id": generation_id,
    });
    let row = sqlx::query(
        r"
        INSERT INTO jobs (
            job_id, queue, kind, state, idempotency_key, generation_id, payload,
            priority, run_after, attempt_count, max_attempts, metadata
        )
        VALUES (
            $1, $2, 'search.sync_once', 'queued', $3, $4, $5,
            0, now(), 0, 3, jsonb_build_object('backoff_seconds', 30::bigint)
        )
        ON CONFLICT (queue, kind, idempotency_key)
            WHERE idempotency_key IS NOT NULL
        DO UPDATE SET updated_at = jobs.updated_at
        RETURNING job_id
        ",
    )
    .bind(job_id)
    .bind(queue)
    .bind(format!("search-sync:{generation_id}"))
    .bind(generation_id)
    .bind(payload)
    .fetch_one(pool)
    .await?;
    let _: String = row.try_get("job_id")?;
    Ok(1)
}

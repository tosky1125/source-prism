use serde::Serialize;
use sqlx::{PgPool, Row as _};

#[derive(Debug, Serialize)]
pub(crate) struct RunSearchSyncOutboxItem {
    pub(crate) outbox_id: String,
    pub(crate) entity_type: String,
    pub(crate) entity_id: String,
    pub(crate) operation: String,
    pub(crate) target_index: String,
    pub(crate) state: String,
    pub(crate) attempt_count: i32,
    pub(crate) processed_at: Option<String>,
    pub(crate) last_error: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct RunSearchSyncOutboxStateCounts {
    pub(crate) queued: i64,
    pub(crate) leased: i64,
    pub(crate) succeeded: i64,
    pub(crate) failed: i64,
    pub(crate) dead_lettered: i64,
    pub(crate) cancelled: i64,
    pub(crate) total: i64,
}

pub(crate) async fn find_search_sync_outbox(
    pool: &PgPool,
    generation_id: &str,
) -> Result<Vec<RunSearchSyncOutboxItem>, sqlx::Error> {
    let rows = sqlx::query(
        r"
        SELECT
            outbox_id,
            entity_type,
            entity_id,
            operation,
            target_index,
            state,
            attempt_count,
            processed_at::text AS processed_at,
            last_error
        FROM search_sync_outbox
        WHERE generation_id = $1
        ORDER BY created_at ASC
        ",
    )
    .bind(generation_id)
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|row| {
            Ok(RunSearchSyncOutboxItem {
                outbox_id: row.try_get("outbox_id")?,
                entity_type: row.try_get("entity_type")?,
                entity_id: row.try_get("entity_id")?,
                operation: row.try_get("operation")?,
                target_index: row.try_get("target_index")?,
                state: row.try_get("state")?,
                attempt_count: row.try_get("attempt_count")?,
                processed_at: row.try_get("processed_at")?,
                last_error: row.try_get("last_error")?,
            })
        })
        .collect()
}

pub(crate) async fn count_search_sync_outbox_states(
    pool: &PgPool,
    generation_id: &str,
) -> Result<RunSearchSyncOutboxStateCounts, sqlx::Error> {
    let row = sqlx::query(
        r"
        SELECT
            count(*) FILTER (WHERE state = 'queued')::bigint AS queued,
            count(*) FILTER (WHERE state = 'leased')::bigint AS leased,
            count(*) FILTER (WHERE state = 'succeeded')::bigint AS succeeded,
            count(*) FILTER (WHERE state = 'failed')::bigint AS failed,
            count(*) FILTER (WHERE state = 'dead_lettered')::bigint AS dead_lettered,
            count(*) FILTER (WHERE state = 'cancelled')::bigint AS cancelled,
            count(*)::bigint AS total
        FROM search_sync_outbox
        WHERE generation_id = $1
        ",
    )
    .bind(generation_id)
    .fetch_one(pool)
    .await?;

    Ok(RunSearchSyncOutboxStateCounts {
        queued: row.try_get("queued")?,
        leased: row.try_get("leased")?,
        succeeded: row.try_get("succeeded")?,
        failed: row.try_get("failed")?,
        dead_lettered: row.try_get("dead_lettered")?,
        cancelled: row.try_get("cancelled")?,
        total: row.try_get("total")?,
    })
}

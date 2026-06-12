#![allow(
    clippy::redundant_pub_crate,
    reason = "Binary crate helper modules share crate-visible command handlers."
)]

use serde_json::json;
use sqlx::{PgPool, Row as _};

use crate::CliError;

pub(crate) async fn search_sync_outbox(
    pool: &PgPool,
    generation_id: &str,
) -> Result<Vec<serde_json::Value>, CliError> {
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
            Ok(json!({
                "outbox_id": row.try_get::<String, _>("outbox_id")?,
                "entity_type": row.try_get::<String, _>("entity_type")?,
                "entity_id": row.try_get::<String, _>("entity_id")?,
                "operation": row.try_get::<String, _>("operation")?,
                "target_index": row.try_get::<String, _>("target_index")?,
                "state": row.try_get::<String, _>("state")?,
                "attempt_count": row.try_get::<i32, _>("attempt_count")?,
                "processed_at": row.try_get::<Option<String>, _>("processed_at")?,
                "last_error": row.try_get::<Option<String>, _>("last_error")?,
            }))
        })
        .collect()
}

pub(crate) async fn search_sync_outbox_state_counts(
    pool: &PgPool,
    generation_id: &str,
) -> Result<serde_json::Value, CliError> {
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

    Ok(json!({
        "queued": row.try_get::<i64, _>("queued")?,
        "leased": row.try_get::<i64, _>("leased")?,
        "succeeded": row.try_get::<i64, _>("succeeded")?,
        "failed": row.try_get::<i64, _>("failed")?,
        "dead_lettered": row.try_get::<i64, _>("dead_lettered")?,
        "cancelled": row.try_get::<i64, _>("cancelled")?,
        "total": row.try_get::<i64, _>("total")?,
    }))
}

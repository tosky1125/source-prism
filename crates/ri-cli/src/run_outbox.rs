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

use sqlx::{PgPool, Row as _};

use crate::search_sync_types::{outbox_id, payload_hash, record_from_row};
use crate::{
    OpenSearchClient, RebuildOutcome, SearchSyncError, SearchSyncInput, SearchSyncOperation,
    SearchSyncRecord, SyncOnceOutcome,
};

#[derive(Debug, Clone)]
pub struct PgSearchSyncStore {
    pub(crate) pool: PgPool,
}

impl PgSearchSyncStore {
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn enqueue(
        &self,
        input: &SearchSyncInput,
    ) -> Result<SearchSyncRecord, SearchSyncError> {
        let payload_hash = payload_hash(&input.payload);
        let outbox_id = outbox_id(input, &payload_hash);
        let row = sqlx::query(
            r"
            INSERT INTO search_sync_outbox (
                outbox_id, repo_id, generation_id, entity_type, entity_id, operation,
                target_index, payload_hash, payload, state
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'queued')
            ON CONFLICT (target_index, entity_type, entity_id, operation, payload_hash)
            DO UPDATE SET
                repo_id = EXCLUDED.repo_id,
                generation_id = EXCLUDED.generation_id,
                payload = EXCLUDED.payload,
                state = 'queued',
                attempt_count = 0,
                run_after = now(),
                leased_by = NULL,
                leased_until = NULL,
                processed_at = NULL,
                last_error = NULL,
                updated_at = now()
            RETURNING outbox_id, entity_id, operation, target_index, payload_hash, payload
            ",
        )
        .bind(outbox_id)
        .bind(&input.repo_id)
        .bind(&input.generation_id)
        .bind(&input.entity_type)
        .bind(&input.entity_id)
        .bind(input.operation.as_str())
        .bind(&input.target_index)
        .bind(payload_hash)
        .bind(&input.payload)
        .fetch_one(&self.pool)
        .await?;
        record_from_row(&row)
    }

    pub async fn sync_once(
        &self,
        client: &OpenSearchClient,
    ) -> Result<SyncOnceOutcome, SearchSyncError> {
        client.health().await?;
        let Some(record) = self.lease_next().await? else {
            return Ok(SyncOnceOutcome {
                processed: false,
                outbox_id: None,
            });
        };
        let result = match record.operation {
            SearchSyncOperation::Upsert => {
                client
                    .upsert_document(&record.target_index, &record.entity_id, &record.payload)
                    .await
            }
            SearchSyncOperation::Delete => {
                client
                    .delete_document(&record.target_index, &record.entity_id)
                    .await
            }
        };
        match result {
            Ok(()) => self.mark_succeeded(&record.outbox_id).await?,
            Err(error) => {
                self.mark_failed(&record.outbox_id, &error.to_string())
                    .await?;
                return Err(error.into());
            }
        }
        Ok(SyncOnceOutcome {
            processed: true,
            outbox_id: Some(record.outbox_id),
        })
    }

    pub async fn rebuild_index(
        &self,
        client: &OpenSearchClient,
        index: &str,
    ) -> Result<RebuildOutcome, SearchSyncError> {
        self.rebuild_index_with_generation(client, index, None)
            .await
    }

    pub async fn rebuild_index_for_generation(
        &self,
        client: &OpenSearchClient,
        index: &str,
        generation_id: &str,
    ) -> Result<RebuildOutcome, SearchSyncError> {
        self.rebuild_index_with_generation(client, index, Some(generation_id))
            .await
    }

    async fn rebuild_index_with_generation(
        &self,
        client: &OpenSearchClient,
        index: &str,
        generation_id: Option<&str>,
    ) -> Result<RebuildOutcome, SearchSyncError> {
        client.health().await?;
        client.delete_index_if_exists(index).await?;
        client.create_index(index).await?;
        let rows = sqlx::query(
            r"
            SELECT entity_id, payload
            FROM search_sync_outbox
            WHERE target_index = $1
              AND ($2::text IS NULL OR generation_id = $2)
              AND operation = 'upsert'
              AND state <> 'cancelled'
            ORDER BY created_at ASC
            ",
        )
        .bind(index)
        .bind(generation_id)
        .fetch_all(&self.pool)
        .await?;
        let mut indexed = 0_u64;
        for row in rows {
            let entity_id = row.try_get::<String, _>("entity_id")?;
            let payload = row.try_get("payload")?;
            client.upsert_document(index, &entity_id, &payload).await?;
            indexed = indexed.saturating_add(1);
        }
        Ok(RebuildOutcome { indexed })
    }

    async fn lease_next(&self) -> Result<Option<SearchSyncRecord>, SearchSyncError> {
        let mut transaction = self.pool.begin().await?;
        let row = sqlx::query(
            r"
            WITH candidate AS (
                SELECT outbox_id
                FROM search_sync_outbox
                WHERE state = 'queued' AND run_after <= now()
                ORDER BY created_at ASC
                FOR UPDATE SKIP LOCKED
                LIMIT 1
            )
            UPDATE search_sync_outbox AS s
            SET state = 'leased',
                leased_by = 'ri-cli-search-sync',
                leased_until = now() + INTERVAL '60 seconds',
                attempt_count = attempt_count + 1,
                updated_at = now()
            FROM candidate
            WHERE s.outbox_id = candidate.outbox_id
            RETURNING s.outbox_id, s.entity_id, s.operation, s.target_index, s.payload_hash, s.payload
            ",
        )
        .fetch_optional(&mut *transaction)
        .await?;
        transaction.commit().await?;
        row.as_ref().map(record_from_row).transpose()
    }

    pub(crate) async fn mark_succeeded(&self, outbox_id: &str) -> Result<(), SearchSyncError> {
        sqlx::query(
            r"
            UPDATE search_sync_outbox
            SET state = 'succeeded', processed_at = now(), updated_at = now()
            WHERE outbox_id = $1
            ",
        )
        .bind(outbox_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub(crate) async fn mark_failed(
        &self,
        outbox_id: &str,
        error: &str,
    ) -> Result<(), SearchSyncError> {
        sqlx::query(
            r"
            UPDATE search_sync_outbox
            SET state = CASE
                    WHEN attempt_count >= max_attempts THEN 'dead_lettered'
                    ELSE 'failed'
                END,
                last_error = $2,
                updated_at = now()
            WHERE outbox_id = $1
            ",
        )
        .bind(outbox_id)
        .bind(error)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

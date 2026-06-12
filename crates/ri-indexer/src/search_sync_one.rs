use crate::search_sync_types::record_from_row;
use crate::{
    OpenSearchClient, PgSearchSyncStore, SearchSyncError, SearchSyncOperation, SyncOnceOutcome,
};

impl PgSearchSyncStore {
    pub async fn sync_one_by_id(
        &self,
        client: &OpenSearchClient,
        outbox_id: &str,
    ) -> Result<SyncOnceOutcome, SearchSyncError> {
        client.health().await?;
        let Some(record) = self.lease_by_id(outbox_id).await? else {
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

    async fn lease_by_id(
        &self,
        outbox_id: &str,
    ) -> Result<Option<crate::SearchSyncRecord>, SearchSyncError> {
        let mut transaction = self.pool.begin().await?;
        let row = sqlx::query(
            r"
            WITH candidate AS (
                SELECT outbox_id
                FROM search_sync_outbox
                WHERE outbox_id = $1
                  AND (
                    (state IN ('queued', 'failed') AND run_after <= now())
                    OR (state = 'leased' AND leased_until <= now())
                  )
                FOR UPDATE SKIP LOCKED
                LIMIT 1
            )
            UPDATE search_sync_outbox AS s
            SET state = 'leased',
                leased_by = 'ri-worker-search-sync',
                leased_until = now() + INTERVAL '60 seconds',
                attempt_count = attempt_count + 1,
                updated_at = now()
            FROM candidate
            WHERE s.outbox_id = candidate.outbox_id
            RETURNING s.outbox_id, s.entity_id, s.operation, s.target_index, s.payload_hash, s.payload
            ",
        )
        .bind(outbox_id)
        .fetch_optional(&mut *transaction)
        .await?;
        transaction.commit().await?;
        row.as_ref().map(record_from_row).transpose()
    }
}

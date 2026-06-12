use sqlx::Row as _;

use crate::{DriftReport, OpenSearchClient, PgSearchSyncStore, SearchSyncError};

impl PgSearchSyncStore {
    pub async fn drift_report(
        &self,
        client: &OpenSearchClient,
        index: &str,
    ) -> Result<DriftReport, SearchSyncError> {
        self.drift_report_with_generation(client, index, None).await
    }

    pub async fn drift_report_for_generation(
        &self,
        client: &OpenSearchClient,
        index: &str,
        generation_id: &str,
    ) -> Result<DriftReport, SearchSyncError> {
        self.drift_report_with_generation(client, index, Some(generation_id))
            .await
    }

    pub async fn drift_report_for_repo_generation(
        &self,
        client: &OpenSearchClient,
        index: &str,
        repo_id: &str,
        generation_id: &str,
    ) -> Result<DriftReport, SearchSyncError> {
        client.health().await?;
        let expected = sqlx::query(
            r"
            SELECT count(*)::bigint AS count
            FROM search_sync_outbox
            WHERE target_index = $1
              AND repo_id = $2
              AND generation_id = $3
              AND operation = 'upsert'
              AND state <> 'cancelled'
            ",
        )
        .bind(index)
        .bind(repo_id)
        .bind(generation_id)
        .fetch_one(&self.pool)
        .await?
        .try_get("count")?;
        let actual = client
            .count_documents_for_repo_generation(index, repo_id, generation_id)
            .await?;
        Ok(DriftReport {
            expected_documents: expected,
            actual_documents: actual,
        })
    }

    async fn drift_report_with_generation(
        &self,
        client: &OpenSearchClient,
        index: &str,
        generation_id: Option<&str>,
    ) -> Result<DriftReport, SearchSyncError> {
        client.health().await?;
        client.refresh_index(index).await?;
        let expected = sqlx::query(
            r"
            SELECT count(*)::bigint AS count
            FROM search_sync_outbox
            WHERE target_index = $1
              AND ($2::text IS NULL OR generation_id = $2)
              AND operation = 'upsert'
              AND state <> 'cancelled'
            ",
        )
        .bind(index)
        .bind(generation_id)
        .fetch_one(&self.pool)
        .await?
        .try_get("count")?;
        let actual = if let Some(generation_id) = generation_id {
            match self.repo_id_for_generation(generation_id).await? {
                Some(repo_id) => {
                    client
                        .count_documents_for_repo_generation(index, &repo_id, generation_id)
                        .await?
                }
                None => 0,
            }
        } else {
            client.count_documents(index).await?
        };
        Ok(DriftReport {
            expected_documents: expected,
            actual_documents: actual,
        })
    }

    async fn repo_id_for_generation(
        &self,
        generation_id: &str,
    ) -> Result<Option<String>, SearchSyncError> {
        let row = sqlx::query(
            r"
            SELECT repo_id
            FROM index_generations
            WHERE generation_id = $1
            ",
        )
        .bind(generation_id)
        .fetch_optional(&self.pool)
        .await?;
        row.map(|row| row.try_get("repo_id"))
            .transpose()
            .map_err(Into::into)
    }
}

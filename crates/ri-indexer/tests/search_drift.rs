#![allow(missing_docs, reason = "Integration test names document behavior.")]
#![allow(
    clippy::multiple_crate_versions,
    reason = "SQLx TLS dependencies currently pull duplicate platform crates outside this crate's control."
)]

use ri_indexer::{OpenSearchClient, PgSearchSyncStore, SearchSyncInput};
use serde_json::json;
use sqlx::PgPool;
use std::time::{SystemTime, UNIX_EPOCH};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[tokio::test]
async fn generation_drift_counts_only_documents_for_that_generation() -> TestResult {
    let Some(database_url) = std::env::var("DATABASE_URL").ok() else {
        return Ok(());
    };
    let Some(opensearch_url) = std::env::var("OPENSEARCH_URL").ok() else {
        return Ok(());
    };
    let pool = PgPool::connect(database_url.as_str()).await?;
    let client = OpenSearchClient::new(opensearch_url.as_str());
    let fixture = Fixture::create(&pool, &client).await?;
    fixture.seed_search_chunks(&pool).await?;
    let store = PgSearchSyncStore::new(pool.clone());

    store.rebuild_index(&client, &fixture.search_index).await?;
    let report = store
        .drift_report_for_generation(
            &client,
            &fixture.search_index,
            fixture.target_generation_id.as_str(),
        )
        .await?;

    assert_eq!(report.expected_documents, 1);
    assert_eq!(report.actual_documents, 1);
    assert!(!report.has_drift());
    fixture.cleanup(&pool, &client).await?;
    Ok(())
}

#[tokio::test]
async fn generation_drift_counts_distinct_documents_when_outbox_has_duplicate_upserts() -> TestResult
{
    let Some(database_url) = std::env::var("DATABASE_URL").ok() else {
        return Ok(());
    };
    let Some(opensearch_url) = std::env::var("OPENSEARCH_URL").ok() else {
        return Ok(());
    };
    let pool = PgPool::connect(database_url.as_str()).await?;
    let client = OpenSearchClient::new(opensearch_url.as_str());
    let fixture = Fixture::create(&pool, &client).await?;
    fixture.seed_duplicate_search_chunks(&pool).await?;
    let store = PgSearchSyncStore::new(pool.clone());

    store.rebuild_index(&client, &fixture.search_index).await?;
    let report = store
        .drift_report_for_generation(
            &client,
            &fixture.search_index,
            fixture.target_generation_id.as_str(),
        )
        .await?;

    assert_eq!(report.expected_documents, 1);
    assert_eq!(report.actual_documents, 1);
    assert!(!report.has_drift());
    fixture.cleanup(&pool, &client).await?;
    Ok(())
}

struct Fixture {
    repo_id: String,
    other_repo_id: String,
    commit_sha: String,
    other_commit_sha: String,
    target_generation_id: String,
    other_generation_id: String,
    search_index: String,
}

impl Fixture {
    async fn create(pool: &PgPool, client: &OpenSearchClient) -> TestResult<Self> {
        let suffix = unique_suffix()?;
        let fixture = Self {
            repo_id: format!("drift-target-repo-{suffix}"),
            other_repo_id: format!("drift-other-repo-{suffix}"),
            commit_sha: format!("target-commit-{suffix}"),
            other_commit_sha: format!("other-commit-{suffix}"),
            target_generation_id: format!("target-generation-{suffix}"),
            other_generation_id: format!("other-generation-{suffix}"),
            search_index: format!("source-prism-drift-{suffix}"),
        };
        client.delete_index_if_exists(&fixture.search_index).await?;
        seed_repo_commit(pool, &fixture.repo_id, &fixture.commit_sha).await?;
        seed_repo_commit(pool, &fixture.other_repo_id, &fixture.other_commit_sha).await?;
        seed_generation(
            pool,
            &fixture.target_generation_id,
            &fixture.repo_id,
            &fixture.commit_sha,
        )
        .await?;
        seed_generation(
            pool,
            &fixture.other_generation_id,
            &fixture.other_repo_id,
            &fixture.other_commit_sha,
        )
        .await?;
        Ok(fixture)
    }

    async fn seed_search_chunks(&self, pool: &PgPool) -> TestResult {
        let store = PgSearchSyncStore::new(pool.clone());
        store
            .enqueue(&SearchSyncInput::upsert_for_generation(
                &self.repo_id,
                &self.target_generation_id,
                "symbol_chunk",
                "target-chunk",
                &self.search_index,
                json!({
                    "chunk_id": "target-chunk",
                    "repo_id": self.repo_id,
                    "generation_id": self.target_generation_id,
                    "text": "target generation chunk",
                }),
            ))
            .await?;
        store
            .enqueue(&SearchSyncInput::upsert_for_generation(
                &self.other_repo_id,
                &self.other_generation_id,
                "symbol_chunk",
                "other-chunk",
                &self.search_index,
                json!({
                    "chunk_id": "other-chunk",
                    "repo_id": self.other_repo_id,
                    "generation_id": self.other_generation_id,
                    "text": "other generation chunk",
                }),
            ))
            .await?;
        Ok(())
    }

    async fn seed_duplicate_search_chunks(&self, pool: &PgPool) -> TestResult {
        let store = PgSearchSyncStore::new(pool.clone());
        for text in ["first duplicate chunk", "second duplicate chunk"] {
            store
                .enqueue(&SearchSyncInput::upsert_for_generation(
                    &self.repo_id,
                    &self.target_generation_id,
                    "symbol_chunk",
                    "duplicate-chunk",
                    &self.search_index,
                    json!({
                        "chunk_id": "duplicate-chunk",
                        "repo_id": self.repo_id,
                        "generation_id": self.target_generation_id,
                        "text": text,
                    }),
                ))
                .await?;
        }
        Ok(())
    }

    async fn cleanup(&self, pool: &PgPool, client: &OpenSearchClient) -> TestResult {
        client.delete_index_if_exists(&self.search_index).await?;
        sqlx::query("DELETE FROM search_sync_outbox WHERE repo_id IN ($1, $2)")
            .bind(&self.repo_id)
            .bind(&self.other_repo_id)
            .execute(pool)
            .await?;
        sqlx::query("DELETE FROM index_generations WHERE repo_id IN ($1, $2)")
            .bind(&self.repo_id)
            .bind(&self.other_repo_id)
            .execute(pool)
            .await?;
        sqlx::query("DELETE FROM commits WHERE repo_id IN ($1, $2)")
            .bind(&self.repo_id)
            .bind(&self.other_repo_id)
            .execute(pool)
            .await?;
        sqlx::query("DELETE FROM repos WHERE repo_id IN ($1, $2)")
            .bind(&self.repo_id)
            .bind(&self.other_repo_id)
            .execute(pool)
            .await?;
        Ok(())
    }
}

async fn seed_repo_commit(pool: &PgPool, repo_id: &str, commit_sha: &str) -> TestResult {
    sqlx::query("INSERT INTO repos (repo_id, name) VALUES ($1, $1)")
        .bind(repo_id)
        .execute(pool)
        .await?;
    sqlx::query("INSERT INTO commits (repo_id, commit_sha) VALUES ($1, $2)")
        .bind(repo_id)
        .bind(commit_sha)
        .execute(pool)
        .await?;
    Ok(())
}

async fn seed_generation(
    pool: &PgPool,
    generation_id: &str,
    repo_id: &str,
    commit_sha: &str,
) -> TestResult {
    sqlx::query(
        r"
        INSERT INTO index_generations (
            generation_id, repo_id, commit_sha, index_kind, status, finished_at
        )
        VALUES ($1, $2, $3, 'file_manifest', 'succeeded', now())
        ",
    )
    .bind(generation_id)
    .bind(repo_id)
    .bind(commit_sha)
    .execute(pool)
    .await?;
    Ok(())
}

fn unique_suffix() -> Result<String, std::time::SystemTimeError> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_nanos()
        .to_string())
}

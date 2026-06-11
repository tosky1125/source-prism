#![allow(
    missing_docs,
    reason = "Integration tests use scenario names instead of API docs."
)]
#![allow(
    clippy::multiple_crate_versions,
    reason = "SQLx TLS dependencies currently pull duplicate platform crates outside this crate's control."
)]

use ri_indexer::{PgSearchSyncStore, SearchSyncInput};
use serde_json::json;
use sqlx::{PgPool, Row as _};
use uuid::Uuid;

#[tokio::test]
async fn search_sync_outbox_is_idempotent_by_deterministic_key()
-> Result<(), Box<dyn std::error::Error>> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };
    let repo_id = format!("repo-{}", Uuid::now_v7());
    seed_repo(&pool, &repo_id).await?;
    let store = PgSearchSyncStore::new(pool.clone());
    let input = SearchSyncInput::upsert(
        &repo_id,
        "chunk",
        "chunk-1",
        "source-prism-test",
        json!({ "chunk_id": "chunk-1", "text": "hello" }),
    );

    let first = store.enqueue(&input).await?;
    let second = store.enqueue(&input).await?;

    assert_eq!(first.outbox_id, second.outbox_id);
    assert_eq!(outbox_count(&pool, &repo_id).await?, 1);
    cleanup(&pool, &repo_id).await?;
    Ok(())
}

async fn test_pool() -> Result<Option<PgPool>, sqlx::Error> {
    let Ok(database_url) = std::env::var("DATABASE_URL") else {
        return Ok(None);
    };
    PgPool::connect(database_url.as_str()).await.map(Some)
}

async fn seed_repo(pool: &PgPool, repo_id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO repos (repo_id, name) VALUES ($1, $1)")
        .bind(repo_id)
        .execute(pool)
        .await?;
    Ok(())
}

async fn outbox_count(pool: &PgPool, repo_id: &str) -> Result<i64, sqlx::Error> {
    let row = sqlx::query(
        r"
        SELECT count(*)::bigint AS count
        FROM search_sync_outbox
        WHERE repo_id = $1
        ",
    )
    .bind(repo_id)
    .fetch_one(pool)
    .await?;
    row.try_get("count")
}

async fn cleanup(pool: &PgPool, repo_id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM search_sync_outbox WHERE repo_id = $1")
        .bind(repo_id)
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM repos WHERE repo_id = $1")
        .bind(repo_id)
        .execute(pool)
        .await?;
    Ok(())
}

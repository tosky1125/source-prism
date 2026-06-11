#![allow(
    missing_docs,
    reason = "Integration tests use scenario names instead of API docs."
)]
#![allow(
    clippy::multiple_crate_versions,
    reason = "SQLx TLS dependencies currently pull duplicate platform crates outside this crate's control."
)]

use ri_core::{CommitSha, FilePath, Language, RepoId, SymbolKind};
use ri_indexer::{PgGenerationStore, PgSearchSyncStore, SearchSyncInput};
use ri_symbols::{SymbolRange, SymbolRecord, SymbolSpec};
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

#[tokio::test]
async fn symbol_chunks_are_enqueued_for_generation() -> Result<(), Box<dyn std::error::Error>> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };
    let repo_id = format!("repo-{}", Uuid::now_v7());
    let commit_sha = "commit";
    seed_repo_commit(&pool, &repo_id, commit_sha).await?;
    let generation = PgGenerationStore::new(pool.clone())
        .begin_generation(&repo_id, commit_sha, "symbol_index", Some("test"))
        .await?;
    let store = PgSearchSyncStore::new(pool.clone());
    let symbol = symbol(&repo_id, commit_sha)?;

    let enqueued = store
        .enqueue_symbol_chunks(
            &repo_id,
            &generation.generation_id,
            &[symbol],
            "source-prism-test",
        )
        .await?;

    assert_eq!(enqueued, 1);
    let row = outbox_payload(&pool, &repo_id).await?;
    assert_eq!(row.try_get::<String, _>("entity_type")?, "symbol_chunk");
    assert_eq!(
        row.try_get::<Option<String>, _>("generation_id")?,
        Some(generation.generation_id.to_string())
    );
    let payload = row.try_get::<serde_json::Value, _>("payload")?;
    assert_eq!(
        payload
            .pointer("/symbol/fqn")
            .and_then(serde_json::Value::as_str),
        Some("InvoiceService::apply_tax")
    );
    assert_eq!(
        payload.pointer("/text").and_then(serde_json::Value::as_str),
        Some("InvoiceService::apply_tax function rust src/invoice.rs")
    );
    cleanup(&pool, &repo_id).await?;
    Ok(())
}

async fn test_pool() -> Result<Option<PgPool>, sqlx::Error> {
    let Ok(database_url) = std::env::var("DATABASE_URL") else {
        return Ok(None);
    };
    PgPool::connect(database_url.as_str()).await.map(Some)
}

async fn seed_repo_commit(
    pool: &PgPool,
    repo_id: &str,
    commit_sha: &str,
) -> Result<(), sqlx::Error> {
    seed_repo(pool, repo_id).await?;
    sqlx::query("INSERT INTO commits (repo_id, commit_sha) VALUES ($1, $2)")
        .bind(repo_id)
        .bind(commit_sha)
        .execute(pool)
        .await?;
    Ok(())
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

async fn outbox_payload(
    pool: &PgPool,
    repo_id: &str,
) -> Result<sqlx::postgres::PgRow, sqlx::Error> {
    sqlx::query(
        r"
        SELECT entity_type, generation_id, payload
        FROM search_sync_outbox
        WHERE repo_id = $1
        ORDER BY created_at
        LIMIT 1
        ",
    )
    .bind(repo_id)
    .fetch_one(pool)
    .await
}

async fn cleanup(pool: &PgPool, repo_id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM search_sync_outbox WHERE repo_id = $1")
        .bind(repo_id)
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM index_generations WHERE repo_id = $1")
        .bind(repo_id)
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM commits WHERE repo_id = $1")
        .bind(repo_id)
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM repos WHERE repo_id = $1")
        .bind(repo_id)
        .execute(pool)
        .await?;
    Ok(())
}

fn symbol(repo_id: &str, commit_sha: &str) -> Result<SymbolRecord, ri_core::CoreError> {
    Ok(SymbolRecord::new(
        &RepoId::new(repo_id)?,
        &CommitSha::new(commit_sha)?,
        FilePath::new("src/invoice.rs")?,
        "hash",
        SymbolSpec::new(
            Language::Rust,
            SymbolKind::Function,
            "apply_tax",
            "InvoiceService::apply_tax",
            SymbolRange::new(1, 0, 3, 1),
        ),
    ))
}

#![allow(missing_docs, reason = "Integration test names document behavior.")]

use ri_embedding::{EmbeddingCacheInput, EmbeddingVector, PgEmbeddingCache};
use sqlx::{PgPool, Row as _};
use uuid::Uuid;

#[test]
fn cache_key_is_stable_for_same_provider_model_input_and_dimensions()
-> Result<(), Box<dyn std::error::Error>> {
    let input = EmbeddingCacheInput::parse(
        "deterministic",
        "local-v1",
        "symbol",
        "InvoiceService::applyTax",
        3,
    )?;
    let same = EmbeddingCacheInput::parse(
        "deterministic",
        "local-v1",
        "symbol",
        "InvoiceService::applyTax",
        3,
    )?;
    let changed_model = EmbeddingCacheInput::parse(
        "deterministic",
        "local-v2",
        "symbol",
        "InvoiceService::applyTax",
        3,
    )?;

    assert_eq!(input.cache_key(), same.cache_key());
    assert_ne!(input.cache_key(), changed_model.cache_key());
    assert_eq!(input.input_sha256(), same.input_sha256());
    Ok(())
}

#[tokio::test]
async fn postgres_cache_returns_hit_for_second_identical_embedding()
-> Result<(), Box<dyn std::error::Error>> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };
    let provider = format!("test-{}", Uuid::now_v7());
    let input = EmbeddingCacheInput::parse(
        provider.as_str(),
        "local-v1",
        "symbol",
        "InvoiceService::applyTax",
        3,
    )?;
    let vector = EmbeddingVector::from_f32(vec![0.1, 0.2, 0.3])?;
    let cache = PgEmbeddingCache::new(pool.clone());

    let first = cache.store_or_touch(&input, &vector).await?;
    let second = cache.store_or_touch(&input, &vector).await?;

    assert!(!first.cache_hit);
    assert!(second.cache_hit);
    assert_eq!(second.entry.vector, vector);
    assert_eq!(cache_rows(&pool, provider.as_str()).await?, 1);
    cleanup(&pool, provider.as_str()).await?;
    Ok(())
}

async fn test_pool() -> Result<Option<PgPool>, sqlx::Error> {
    let Ok(database_url) = std::env::var("DATABASE_URL") else {
        return Ok(None);
    };
    PgPool::connect(database_url.as_str()).await.map(Some)
}

async fn cache_rows(pool: &PgPool, provider: &str) -> Result<i64, sqlx::Error> {
    let row = sqlx::query(
        r"
        SELECT count(*)::bigint AS count
        FROM embedding_cache
        WHERE provider = $1
        ",
    )
    .bind(provider)
    .fetch_one(pool)
    .await?;
    row.try_get("count")
}

async fn cleanup(pool: &PgPool, provider: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM embedding_cache WHERE provider = $1")
        .bind(provider)
        .execute(pool)
        .await?;
    Ok(())
}

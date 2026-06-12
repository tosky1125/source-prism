#![allow(missing_docs, reason = "CLI integration test names document behavior.")]

use std::{
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use serde_json::Value;
use sqlx::{PgPool, Row as _};

#[tokio::test]
async fn embeddings_cache_put_persists_and_reports_cache_hit()
-> Result<(), Box<dyn std::error::Error>> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };
    let provider = format!("cli-{}", unique_suffix()?);

    let first = run_cache_put(provider.as_str())?;
    let second = run_cache_put(provider.as_str())?;

    assert_eq!(
        first.pointer("/kind").and_then(Value::as_str),
        Some("embedding_cache")
    );
    assert_eq!(
        first.pointer("/cache_hit").and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        second.pointer("/cache_hit").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        second.pointer("/dimensions").and_then(Value::as_u64),
        Some(3)
    );
    assert_eq!(cache_rows(&pool, provider.as_str()).await?, 1);
    cleanup(&pool, provider.as_str()).await?;
    Ok(())
}

fn unique_suffix() -> Result<String, std::time::SystemTimeError> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_nanos()
        .to_string())
}

fn run_cache_put(provider: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let output = Command::new(env!("CARGO_BIN_EXE_ri-cli"))
        .current_dir(repo_root)
        .env(
            "DATABASE_URL",
            std::env::var("DATABASE_URL").unwrap_or_default(),
        )
        .args([
            "embeddings",
            "cache-put",
            "--provider",
            provider,
            "--model",
            "local-v1",
            "--kind",
            "symbol",
            "--dimensions",
            "3",
            "--input",
            "InvoiceService::applyTax",
            "--vector",
            "0.1,0.2,0.3",
        ])
        .output()?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(serde_json::from_slice(&output.stdout)?)
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

#![allow(missing_docs, reason = "Integration test names document behavior.")]

use axum::{
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode, header},
};
use ri_api::{AppState, app};
use ri_core::{CommitSha, FilePath, Language, RepoId, SymbolKind};
use ri_indexer::{PgGenerationStore, PgGraphStore, PgSearchSyncStore, PgSymbolStore};
use ri_symbols::{SymbolRange, SymbolRecord, SymbolSpec};
use serde_json::Value;
use sqlx::PgPool;
use std::time::{SystemTime, UNIX_EPOCH};
use tower::ServiceExt;

#[tokio::test]
async fn context_search_returns_context_pack_for_matching_symbol()
-> Result<(), Box<dyn std::error::Error>> {
    let app = app(AppState::for_test_symbols(vec![
        symbol("src/invoice.rs", "InvoiceService::apply_tax")?,
        symbol("src/invoice.rs", "InvoiceService::helper")?,
    ])?);
    let request = Request::builder()
        .method(Method::POST)
        .uri("/v1/context/search")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(r#"{"query":"apply_tax"}"#))?;

    let response = app.oneshot(request).await?;

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), 1_000_000).await?;
    let body = serde_json::from_slice::<Value>(&bytes)?;
    assert_eq!(body.pointer("/status").and_then(Value::as_str), Some("ok"));
    assert_eq!(
        body.pointer("/kind").and_then(Value::as_str),
        Some("context_search")
    );
    assert_eq!(
        body.pointer("/context_pack/vector_only")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        body.pointer("/context_pack/hits/0/symbol/fqn")
            .and_then(Value::as_str),
        Some("InvoiceService::apply_tax")
    );
    assert_eq!(body.pointer("/hit_count").and_then(Value::as_u64), Some(1));
    assert_eq!(
        body.pointer("/impact_count").and_then(Value::as_u64),
        Some(1)
    );
    assert_eq!(
        body.pointer("/context_pack/impacts")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(1)
    );
    Ok(())
}

#[tokio::test]
async fn context_search_with_repo_id_requires_database() -> Result<(), Box<dyn std::error::Error>> {
    let app = app(AppState::for_test_symbols(Vec::new())?);
    let request = Request::builder()
        .method(Method::POST)
        .uri("/v1/context/search")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(r#"{"repo_id":"repo","query":"apply_tax"}"#))?;

    let response = app.oneshot(request).await?;

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let bytes = to_bytes(response.into_body(), 1_000_000).await?;
    let body = serde_json::from_slice::<Value>(&bytes)?;
    assert_eq!(
        body.pointer("/error/code").and_then(Value::as_str),
        Some("database_not_configured")
    );
    Ok(())
}

#[tokio::test]
async fn context_search_with_repo_id_reports_search_chunk_evidence()
-> Result<(), Box<dyn std::error::Error>> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };
    let fixture = Fixture::create(&pool).await?;
    let generation = PgGenerationStore::new(pool.clone())
        .begin_generation(
            &fixture.repo_id,
            &fixture.commit_sha,
            "context_search",
            Some("test"),
        )
        .await?;
    let symbol = fixture.symbol("InvoiceService::apply_tax")?;
    PgSymbolStore::new(pool.clone())
        .replace_symbol_generation(&generation.generation_id, std::slice::from_ref(&symbol))
        .await?;
    PgGraphStore::new(pool.clone())
        .replace_contains_graph(&generation.generation_id, std::slice::from_ref(&symbol))
        .await?;
    PgSearchSyncStore::new(pool.clone())
        .enqueue_symbol_chunks(
            &fixture.repo_id,
            &generation.generation_id,
            &[symbol],
            "source-prism-test",
        )
        .await?;
    PgGenerationStore::new(pool.clone())
        .finish_generation(&generation.generation_id)
        .await?;
    let app = app(AppState::for_test_database(pool.clone())?);
    let request = Request::builder()
        .method(Method::POST)
        .uri("/v1/context/search")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(format!(
            r#"{{"repo_id":"{}","query":"apply_tax"}}"#,
            fixture.repo_id
        )))?;

    let response = app.oneshot(request).await?;

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), 1_000_000).await?;
    let body = serde_json::from_slice::<Value>(&bytes)?;
    assert_eq!(
        body.pointer("/search_chunk_count").and_then(Value::as_u64),
        Some(1)
    );
    assert_eq!(
        body.pointer("/context_pack/hits/0/symbol/fqn")
            .and_then(Value::as_str),
        Some("InvoiceService::apply_tax")
    );
    fixture.cleanup(&pool).await?;
    Ok(())
}

#[derive(Debug)]
struct Fixture {
    repo_id: String,
    commit_sha: String,
}

impl Fixture {
    async fn create(pool: &PgPool) -> Result<Self, sqlx::Error> {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| duration.as_nanos());
        let fixture = Self {
            repo_id: format!("repo-{suffix}"),
            commit_sha: format!("commit-{suffix}"),
        };
        sqlx::query("INSERT INTO repos (repo_id, name) VALUES ($1, $1)")
            .bind(&fixture.repo_id)
            .execute(pool)
            .await?;
        sqlx::query("INSERT INTO commits (repo_id, commit_sha) VALUES ($1, $2)")
            .bind(&fixture.repo_id)
            .bind(&fixture.commit_sha)
            .execute(pool)
            .await?;
        Ok(fixture)
    }

    fn symbol(&self, fqn: &str) -> Result<SymbolRecord, ri_core::CoreError> {
        let repo = RepoId::new(&self.repo_id)?;
        let commit = CommitSha::new(&self.commit_sha)?;
        Ok(SymbolRecord::new(
            &repo,
            &commit,
            FilePath::new("src/invoice.rs")?,
            "hash",
            SymbolSpec::new(
                Language::Rust,
                SymbolKind::Function,
                fqn,
                fqn,
                SymbolRange::new(1, 0, 2, 0),
            ),
        ))
    }

    async fn cleanup(&self, pool: &PgPool) -> Result<(), sqlx::Error> {
        for table in [
            "search_sync_outbox",
            "graph_edges",
            "graph_nodes",
            "symbols",
            "index_generations",
            "commits",
            "repos",
        ] {
            sqlx::query(&format!("DELETE FROM {table} WHERE repo_id = $1"))
                .bind(&self.repo_id)
                .execute(pool)
                .await?;
        }
        Ok(())
    }
}

async fn test_pool() -> Result<Option<PgPool>, sqlx::Error> {
    let Ok(database_url) = std::env::var("DATABASE_URL") else {
        return Ok(None);
    };
    PgPool::connect(database_url.as_str()).await.map(Some)
}

fn symbol(path: &str, fqn: &str) -> Result<SymbolRecord, ri_core::CoreError> {
    let repo = RepoId::new("repo")?;
    let commit = CommitSha::new("commit")?;
    Ok(SymbolRecord::new(
        &repo,
        &commit,
        FilePath::new(path)?,
        "hash",
        SymbolSpec::new(
            Language::Rust,
            SymbolKind::Function,
            fqn,
            fqn,
            SymbolRange::new(1, 0, 2, 0),
        ),
    ))
}

#![allow(missing_docs, reason = "Integration test names document behavior.")]

use axum::{
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode, header},
};
use ri_api::{AppState, app};
use ri_behavior::parse_lcov;
use ri_core::{CommitSha, FilePath, Language, RepoId, SymbolKind};
use ri_indexer::{PgCoverageStore, PgGenerationStore, PgSymbolStore};
use ri_symbols::{SymbolRange, SymbolRecord, SymbolSpec};
use serde_json::Value;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

#[tokio::test]
async fn test_context_returns_static_test_evidence() -> Result<(), Box<dyn std::error::Error>> {
    let app = app(AppState::for_test_symbols(vec![
        symbol(SymbolKind::Function, "apply_tax", "src/invoice.rs")?,
        symbol(
            SymbolKind::TestCase,
            "apply_tax_adds_rate",
            "tests/invoice.rs",
        )?,
    ])?);
    let request = Request::builder()
        .method(Method::POST)
        .uri("/v1/test-context")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(r#"{"symbol":"apply_tax"}"#))?;

    let response = app.oneshot(request).await?;

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), 1_000_000).await?;
    let body = serde_json::from_slice::<Value>(&bytes)?;
    assert_eq!(body.pointer("/status").and_then(Value::as_str), Some("ok"));
    assert_eq!(
        body.pointer("/kind").and_then(Value::as_str),
        Some("test_context")
    );
    assert_eq!(
        body.pointer("/test_context/code_execution_allowed")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        body.pointer("/test_context/related_tests/0/fqn")
            .and_then(Value::as_str),
        Some("apply_tax_adds_rate")
    );
    Ok(())
}

#[tokio::test]
async fn test_context_with_repo_id_includes_overlapping_coverage()
-> Result<(), Box<dyn std::error::Error>> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };
    let fixture = Fixture::create(&pool).await?;
    let generation = PgGenerationStore::new(pool.clone())
        .begin_generation(
            &fixture.repo_id,
            &fixture.commit_sha,
            "test_context",
            Some("test"),
        )
        .await?;
    let target = symbol_in(
        &RepoId::new(fixture.repo_id.as_str())?,
        &CommitSha::new(fixture.commit_sha.as_str())?,
        SymbolKind::Function,
        "apply_tax",
        "src/invoice.rs",
    )?;
    PgSymbolStore::new(pool.clone())
        .replace_symbol_generation(&generation.generation_id, &[target])
        .await?;
    let report = parse_lcov("SF:src/invoice.rs\nDA:1,3\nend_of_record\n")?;
    PgCoverageStore::new(pool.clone())
        .replace_lcov_for_generation(&generation.generation_id, "lcov.info", &report)
        .await?;
    PgGenerationStore::new(pool.clone())
        .finish_generation(&generation.generation_id)
        .await?;
    let app = app(AppState::for_test_database(pool.clone())?);
    let request = Request::builder()
        .method(Method::POST)
        .uri("/v1/test-context")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(format!(
            r#"{{"repo_id":"{}","symbol":"apply_tax"}}"#,
            fixture.repo_id
        )))?;

    let response = app.oneshot(request).await?;

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), 1_000_000).await?;
    let body = serde_json::from_slice::<Value>(&bytes)?;
    assert_eq!(
        body.pointer("/test_context/coverage_available")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        body.pointer("/test_context/coverage_segments/0/source_path")
            .and_then(Value::as_str),
        Some("lcov.info")
    );
    fixture.cleanup(&pool).await?;
    Ok(())
}

fn symbol(kind: SymbolKind, fqn: &str, path: &str) -> Result<SymbolRecord, ri_core::CoreError> {
    let repo = RepoId::new("repo")?;
    let commit = CommitSha::new("commit")?;
    symbol_in(&repo, &commit, kind, fqn, path)
}

fn symbol_in(
    repo: &RepoId,
    commit: &CommitSha,
    kind: SymbolKind,
    fqn: &str,
    path: &str,
) -> Result<SymbolRecord, ri_core::CoreError> {
    Ok(SymbolRecord::new(
        repo,
        commit,
        FilePath::new(path)?,
        "hash",
        SymbolSpec::new(Language::Rust, kind, fqn, fqn, SymbolRange::new(1, 0, 2, 0)),
    ))
}

#[derive(Debug)]
struct Fixture {
    repo_id: String,
    commit_sha: String,
}

impl Fixture {
    async fn create(pool: &PgPool) -> Result<Self, sqlx::Error> {
        let suffix = Uuid::now_v7();
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

    async fn cleanup(&self, pool: &PgPool) -> Result<(), sqlx::Error> {
        for table in [
            "coverage_segments",
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

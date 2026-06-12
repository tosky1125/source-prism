#![allow(missing_docs, reason = "Integration test names document behavior.")]

use axum::{
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode},
};
use ri_api::{AppState, app};
use ri_behavior::parse_lcov;
use ri_indexer::{PgCoverageStore, PgGenerationStore};
use serde_json::Value;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

#[tokio::test]
async fn repo_coverage_returns_empty_local_state_without_database()
-> Result<(), Box<dyn std::error::Error>> {
    let app = app(AppState::for_test_repo_path(".".into())?);
    let request = Request::builder()
        .method(Method::GET)
        .uri("/v1/repos/local/coverage")
        .body(Body::empty())?;

    let response = app.oneshot(request).await?;

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), 1_000_000).await?;
    let body = serde_json::from_slice::<Value>(&bytes)?;
    assert_eq!(
        body.pointer("/kind").and_then(Value::as_str),
        Some("coverage")
    );
    assert_eq!(
        body.pointer("/repo_id").and_then(Value::as_str),
        Some("local")
    );
    assert_eq!(
        body.pointer("/segment_count").and_then(Value::as_u64),
        Some(0)
    );
    assert_eq!(
        body.pointer("/segments").and_then(Value::as_array),
        Some(&Vec::new())
    );
    Ok(())
}

#[tokio::test]
async fn repo_coverage_returns_indexed_lcov_segments_for_repo()
-> Result<(), Box<dyn std::error::Error>> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };
    let fixture = Fixture::create(&pool).await?;
    let generation = PgGenerationStore::new(pool.clone())
        .begin_generation(
            &fixture.repo_id,
            &fixture.commit_sha,
            "coverage",
            Some("test"),
        )
        .await?;
    let report = parse_lcov("SF:src/invoice.rs\nDA:3,1\nend_of_record\n")?;
    PgCoverageStore::new(pool.clone())
        .replace_lcov_for_generation(&generation.generation_id, "lcov.info", &report)
        .await?;
    PgGenerationStore::new(pool.clone())
        .finish_generation(&generation.generation_id)
        .await?;
    let app = app(AppState::for_test_database(pool.clone())?);
    let request = Request::builder()
        .method(Method::GET)
        .uri(format!("/v1/repos/{}/coverage", fixture.repo_id))
        .body(Body::empty())?;

    let response = app.oneshot(request).await?;

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), 1_000_000).await?;
    let body = serde_json::from_slice::<Value>(&bytes)?;
    assert_eq!(
        body.pointer("/kind").and_then(Value::as_str),
        Some("coverage")
    );
    assert_eq!(
        body.pointer("/segment_count").and_then(Value::as_u64),
        Some(1)
    );
    assert_eq!(
        body.pointer("/segments/0/file_path")
            .and_then(Value::as_str),
        Some("src/invoice.rs")
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
        for table in ["coverage_segments", "index_generations", "commits", "repos"] {
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

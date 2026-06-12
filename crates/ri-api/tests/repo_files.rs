#![allow(missing_docs, reason = "Integration test names document behavior.")]

use axum::{
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode},
};
use ri_api::{AppState, RepoFile, RepoFileFlags, app};
use ri_core::Language;
use ri_indexer::{FileManifestInput, PgGenerationStore};
use serde_json::Value;
use sqlx::PgPool;
use std::time::{SystemTime, UNIX_EPOCH};
use tower::ServiceExt;

#[tokio::test]
async fn repo_files_returns_file_inventory_for_repo() -> Result<(), Box<dyn std::error::Error>> {
    let app = app(AppState::for_test_files(vec![
        RepoFile::new(
            "src/invoice.rs",
            Language::Rust,
            42,
            "abc123",
            RepoFileFlags::new(false, false, false),
        ),
        RepoFile::new(
            "tests/invoice.rs",
            Language::Rust,
            24,
            "def456",
            RepoFileFlags::new(false, false, true),
        ),
    ])?);
    let request = Request::builder()
        .method(Method::GET)
        .uri("/v1/repos/local/files")
        .body(Body::empty())?;

    let response = app.oneshot(request).await?;

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), 1_000_000).await?;
    let body = serde_json::from_slice::<Value>(&bytes)?;
    assert_eq!(body.pointer("/status").and_then(Value::as_str), Some("ok"));
    assert_eq!(body.pointer("/kind").and_then(Value::as_str), Some("files"));
    assert_eq!(
        body.pointer("/repo_id").and_then(Value::as_str),
        Some("local")
    );
    assert_eq!(body.pointer("/file_count").and_then(Value::as_u64), Some(2));
    assert_eq!(
        body.pointer("/files/0/path").and_then(Value::as_str),
        Some("src/invoice.rs")
    );
    assert_eq!(
        body.pointer("/files/1/is_test").and_then(Value::as_bool),
        Some(true)
    );
    Ok(())
}

#[tokio::test]
async fn repo_files_returns_indexed_db_manifests_for_repo_id()
-> Result<(), Box<dyn std::error::Error>> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };
    let fixture = Fixture::create(&pool).await?;
    let generation = PgGenerationStore::new(pool.clone())
        .begin_generation(
            &fixture.repo_id,
            &fixture.commit_sha,
            "file_manifest",
            Some("test"),
        )
        .await?;
    let mut manifest = FileManifestInput::new("src/db_only.rs", "abc123", 42);
    "rust".clone_into(&mut manifest.language);
    manifest.is_test = true;
    PgGenerationStore::new(pool.clone())
        .replace_file_manifest_generation(&generation.generation_id, &[manifest])
        .await?;
    let app = app(AppState::for_test_database(pool.clone())?);
    let request = Request::builder()
        .method(Method::GET)
        .uri(format!("/v1/repos/{}/files", fixture.repo_id))
        .body(Body::empty())?;

    let response = app.oneshot(request).await?;

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), 1_000_000).await?;
    let body = serde_json::from_slice::<Value>(&bytes)?;
    assert_eq!(body.pointer("/file_count").and_then(Value::as_u64), Some(1));
    assert_eq!(
        body.pointer("/files/0/path").and_then(Value::as_str),
        Some("src/db_only.rs")
    );
    assert_eq!(
        body.pointer("/files/0/language").and_then(Value::as_str),
        Some("rust")
    );
    assert_eq!(
        body.pointer("/files/0/is_test").and_then(Value::as_bool),
        Some(true)
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

    async fn cleanup(&self, pool: &PgPool) -> Result<(), sqlx::Error> {
        for table in ["file_manifests", "index_generations", "commits", "repos"] {
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

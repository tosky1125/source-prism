#![allow(missing_docs, reason = "Integration test names document behavior.")]

use axum::{
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode},
};
use ri_api::{AppState, app};
use serde_json::Value;
use sqlx::PgPool;
use std::time::{SystemTime, UNIX_EPOCH};
use tower::ServiceExt;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[tokio::test]
async fn repo_search_sync_reports_latest_generation_queue_state() -> TestResult {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };
    let fixture = Fixture::create(&pool).await?;
    fixture.seed_sync_state(&pool).await?;
    let app = app(AppState::for_test_database(pool.clone())?);
    let request = Request::builder()
        .method(Method::GET)
        .uri(format!("/v1/repos/{}/search-sync", fixture.repo_id))
        .body(Body::empty())?;

    let response = app.oneshot(request).await?;

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), 1_000_000).await?;
    let body = serde_json::from_slice::<Value>(&bytes)?;
    assert_eq!(
        body.pointer("/kind").and_then(Value::as_str),
        Some("repo_search_sync")
    );
    assert_eq!(
        body.pointer("/repo_id").and_then(Value::as_str),
        Some(fixture.repo_id.as_str())
    );
    assert_eq!(
        body.pointer("/latest_generation_id")
            .and_then(Value::as_str),
        Some(fixture.generation_id.as_str())
    );
    assert_eq!(
        body.pointer("/latest_commit_sha").and_then(Value::as_str),
        Some(fixture.commit_sha.as_str())
    );
    assert_eq!(
        body.pointer("/latest_run_status").and_then(Value::as_str),
        Some("succeeded")
    );
    assert_eq!(
        body.pointer("/outbox_state_counts/queued")
            .and_then(Value::as_i64),
        Some(1)
    );
    assert_eq!(
        body.pointer("/outbox_state_counts/total")
            .and_then(Value::as_i64),
        Some(1)
    );
    assert_eq!(
        body.pointer("/job_state_counts/queued")
            .and_then(Value::as_i64),
        Some(1)
    );
    assert_eq!(
        body.pointer("/job_state_counts/total")
            .and_then(Value::as_i64),
        Some(1)
    );
    fixture.cleanup(&pool).await?;
    Ok(())
}

struct Fixture {
    repo_id: String,
    commit_sha: String,
    generation_id: String,
}

impl Fixture {
    async fn create(pool: &PgPool) -> TestResult<Self> {
        let suffix = unique_suffix()?;
        let fixture = Self {
            repo_id: format!("api-repo-search-sync-{suffix}"),
            commit_sha: format!("commit-{suffix}"),
            generation_id: format!("generation-{suffix}"),
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
        sqlx::query(
            r"
            INSERT INTO index_generations (
                generation_id, repo_id, commit_sha, index_kind, status, finished_at
            )
            VALUES ($1, $2, $3, 'file_manifest', 'succeeded', now())
            ",
        )
        .bind(&fixture.generation_id)
        .bind(&fixture.repo_id)
        .bind(&fixture.commit_sha)
        .execute(pool)
        .await?;
        Ok(fixture)
    }

    async fn seed_sync_state(&self, pool: &PgPool) -> Result<(), sqlx::Error> {
        sqlx::query(
            r"
            INSERT INTO search_sync_outbox (
                outbox_id, repo_id, generation_id, entity_type, entity_id, operation,
                target_index, payload_hash, state
            )
            VALUES ($1, $2, $3, 'symbol_chunk', $4, 'upsert', 'source-prism-test', $5, 'queued')
            ",
        )
        .bind(format!("outbox-{}", self.generation_id))
        .bind(&self.repo_id)
        .bind(&self.generation_id)
        .bind(format!("chunk-{}", self.generation_id))
        .bind(format!("hash-{}", self.generation_id))
        .execute(pool)
        .await?;
        sqlx::query(
            r"
            INSERT INTO jobs (job_id, queue, kind, state, generation_id, payload)
            VALUES ($1, 'default', 'search.sync_once', 'queued', $2, $3::jsonb)
            ",
        )
        .bind(format!("job-{}", self.generation_id))
        .bind(&self.generation_id)
        .bind(serde_json::json!({ "generation_id": self.generation_id }).to_string())
        .execute(pool)
        .await?;
        Ok(())
    }

    async fn cleanup(&self, pool: &PgPool) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM search_sync_outbox WHERE repo_id = $1")
            .bind(&self.repo_id)
            .execute(pool)
            .await?;
        sqlx::query("DELETE FROM index_generations WHERE repo_id = $1")
            .bind(&self.repo_id)
            .execute(pool)
            .await?;
        sqlx::query("DELETE FROM commits WHERE repo_id = $1")
            .bind(&self.repo_id)
            .execute(pool)
            .await?;
        sqlx::query("DELETE FROM repos WHERE repo_id = $1")
            .bind(&self.repo_id)
            .execute(pool)
            .await?;
        Ok(())
    }
}

fn unique_suffix() -> Result<String, std::time::SystemTimeError> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_nanos()
        .to_string())
}

async fn test_pool() -> Result<Option<PgPool>, sqlx::Error> {
    let Ok(database_url) = std::env::var("DATABASE_URL") else {
        return Ok(None);
    };
    PgPool::connect(database_url.as_str()).await.map(Some)
}

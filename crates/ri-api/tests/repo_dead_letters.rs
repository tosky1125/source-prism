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
async fn repo_dead_letters_returns_failed_job_attempt_evidence() -> TestResult {
    // Given: a repo generation with one dead-lettered search sync job.
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };
    let fixture = Fixture::create(&pool).await?;
    fixture.seed_dead_letter(&pool).await?;
    let app = app(AppState::for_test_database(pool.clone())?);

    // When: the repo dead-letter endpoint is queried.
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!("/v1/repos/{}/dead-letters", fixture.repo_id))
                .body(Body::empty())?,
        )
        .await?;

    // Then: the response exposes the failed job and attempt evidence.
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), 1_000_000).await?;
    let body = serde_json::from_slice::<Value>(&bytes)?;
    assert_eq!(
        body.pointer("/kind").and_then(Value::as_str),
        Some("repo_dead_letters")
    );
    assert_eq!(
        body.pointer("/dead_letter_count").and_then(Value::as_u64),
        Some(1)
    );
    assert_eq!(
        body.pointer("/dead_letters/0/job_id")
            .and_then(Value::as_str),
        Some(fixture.job_id.as_str())
    );
    assert_eq!(
        body.pointer("/dead_letters/0/last_error")
            .and_then(Value::as_str),
        Some("OpenSearch bulk failed")
    );
    assert_eq!(
        body.pointer("/dead_letters/0/attempts/0/error")
            .and_then(Value::as_str),
        Some("OpenSearch bulk failed")
    );
    fixture.cleanup(&pool).await?;
    Ok(())
}

struct Fixture {
    repo_id: String,
    commit_sha: String,
    generation_id: String,
    job_id: String,
}

impl Fixture {
    async fn create(pool: &PgPool) -> TestResult<Self> {
        let suffix = unique_suffix()?;
        let fixture = Self {
            repo_id: format!("api-dead-letters-{suffix}"),
            commit_sha: format!("commit-{suffix}"),
            generation_id: format!("generation-{suffix}"),
            job_id: format!("job-{suffix}"),
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

    async fn seed_dead_letter(&self, pool: &PgPool) -> Result<(), sqlx::Error> {
        sqlx::query(
            r"
            INSERT INTO jobs (
                job_id, queue, kind, state, generation_id, payload,
                attempt_count, max_attempts, last_error, completed_at
            )
            VALUES (
                $1, 'default', 'search.sync_once', 'dead_lettered', $2, $3::jsonb,
                3, 3, 'OpenSearch bulk failed', now()
            )
            ",
        )
        .bind(&self.job_id)
        .bind(&self.generation_id)
        .bind(serde_json::json!({ "generation_id": self.generation_id }).to_string())
        .execute(pool)
        .await?;
        sqlx::query(
            r"
            INSERT INTO job_attempts (
                job_id, attempt_no, worker_id, status, finished_at, error
            )
            VALUES ($1, 3, 'worker-test', 'failed', now(), 'OpenSearch bulk failed')
            ",
        )
        .bind(&self.job_id)
        .execute(pool)
        .await?;
        Ok(())
    }

    async fn cleanup(&self, pool: &PgPool) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM job_attempts WHERE job_id = $1")
            .bind(&self.job_id)
            .execute(pool)
            .await?;
        sqlx::query("DELETE FROM jobs WHERE job_id = $1")
            .bind(&self.job_id)
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

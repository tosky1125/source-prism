#![allow(missing_docs, reason = "Integration test names document behavior.")]

use axum::{
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode, header},
};
use ri_api::{AppState, app};
use serde_json::Value;
use sqlx::PgPool;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};
use tower::ServiceExt;

#[tokio::test]
async fn get_run_requires_database() -> Result<(), Box<dyn std::error::Error>> {
    let app = app(AppState::for_test_symbols(Vec::new())?);
    let request = Request::builder()
        .method(Method::GET)
        .uri("/v1/runs/run-1")
        .body(Body::empty())?;

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
async fn get_run_returns_index_evidence_counts() -> Result<(), Box<dyn std::error::Error>> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };
    let repo = TempRepo::create()?;
    repo.write_file(
        "src/lib.rs",
        r"
pub fn run_evidence_fixture() -> i32 {
    7
}

#[test]
fn run_evidence_fixture_is_indexed() {
    assert_eq!(run_evidence_fixture(), 7);
}
",
    )?;
    repo.commit()?;
    let repo_id = format!("api-run-{}", unique_suffix()?);
    let app = app(AppState::for_test_database(pool.clone())?);
    let index_body = serde_json::json!({
        "sha": "HEAD",
        "repo_path": repo.path(),
    });
    let index_request = Request::builder()
        .method(Method::POST)
        .uri(format!("/v1/repos/{repo_id}/index"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(index_body.to_string()))?;

    let index_response = app.clone().oneshot(index_request).await?;
    assert_eq!(index_response.status(), StatusCode::OK);
    let index_bytes = to_bytes(index_response.into_body(), 1_000_000).await?;
    let index = serde_json::from_slice::<Value>(&index_bytes)?;
    let run_id = index
        .pointer("/run_id")
        .and_then(Value::as_str)
        .ok_or("missing run id")?;
    insert_started_search_sync_attempt(&pool, run_id).await?;

    let run_request = Request::builder()
        .method(Method::GET)
        .uri(format!("/v1/runs/{run_id}"))
        .body(Body::empty())?;
    let run_response = app.oneshot(run_request).await?;

    assert_eq!(run_response.status(), StatusCode::OK);
    let run_bytes = to_bytes(run_response.into_body(), 1_000_000).await?;
    let run = serde_json::from_slice::<Value>(&run_bytes)?;
    assert_eq!(
        run.pointer("/run/status").and_then(Value::as_str),
        Some("succeeded")
    );
    assert!(
        run.pointer("/run/extractor_version")
            .and_then(Value::as_str)
            .is_some()
    );
    assert_count_at_least(&run, "/run/evidence/file_manifests", 1)?;
    assert_count_at_least(&run, "/run/evidence/symbols", 2)?;
    assert_count_at_least(&run, "/run/evidence/graph_nodes", 1)?;
    assert_count_at_least(&run, "/run/evidence/graph_edges", 1)?;
    assert_count_at_least(&run, "/run/evidence/search_chunks", 1)?;
    assert_count_at_least(&run, "/run/evidence/search_sync_jobs", 1)?;
    assert_eq!(
        run.pointer("/run/evidence/search_sync_job_details/0/state")
            .and_then(Value::as_str),
        Some("queued")
    );
    assert!(
        run.pointer("/run/evidence/search_sync_job_details/0/job_id")
            .and_then(Value::as_str)
            .is_some()
    );
    assert_eq!(
        run.pointer("/run/evidence/search_sync_job_details/0/attempt_count")
            .and_then(Value::as_i64),
        Some(0)
    );
    assert_eq!(
        run.pointer("/run/evidence/search_sync_job_details/0/attempts/0/status")
            .and_then(Value::as_str),
        Some("started")
    );
    assert_eq!(
        run.pointer("/run/evidence/search_sync_job_details/0/attempts/0/worker_id")
            .and_then(Value::as_str),
        Some("api-test-worker")
    );
    assert_count_at_least(&run, "/run/evidence/test_cases", 1)?;
    cleanup(&pool, &repo_id).await?;
    repo.cleanup()?;
    Ok(())
}

struct TempRepo {
    path: PathBuf,
}

impl TempRepo {
    fn create() -> Result<Self, Box<dyn std::error::Error>> {
        let path = std::env::temp_dir().join(format!("source-prism-api-run-{}", unique_suffix()?));
        fs::create_dir_all(path.join("src"))?;
        run_git(&path, ["init"])?;
        run_git(
            &path,
            ["config", "user.email", "source-prism@example.invalid"],
        )?;
        run_git(&path, ["config", "user.name", "Source Prism Test"])?;
        Ok(Self { path })
    }

    fn path(&self) -> &Path {
        self.path.as_path()
    }

    fn write_file(&self, path: &str, body: &str) -> Result<(), std::io::Error> {
        fs::write(self.path.join(path), body)
    }

    fn commit(&self) -> Result<(), Box<dyn std::error::Error>> {
        run_git(&self.path, ["add", "."])?;
        run_git(&self.path, ["commit", "-m", "fixture"])?;
        Ok(())
    }

    fn cleanup(&self) -> Result<(), std::io::Error> {
        fs::remove_dir_all(&self.path)
    }
}

fn unique_suffix() -> Result<String, std::time::SystemTimeError> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_nanos()
        .to_string())
}

fn run_git<const N: usize>(path: &Path, args: [&str; N]) -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new("git").current_dir(path).args(args).output()?;
    if output.status.success() {
        return Ok(());
    }
    Err(std::io::Error::other(String::from_utf8_lossy(&output.stderr).to_string()).into())
}

async fn test_pool() -> Result<Option<PgPool>, sqlx::Error> {
    let Ok(database_url) = std::env::var("DATABASE_URL") else {
        return Ok(None);
    };
    PgPool::connect(database_url.as_str()).await.map(Some)
}

async fn insert_started_search_sync_attempt(
    pool: &PgPool,
    generation_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r"
        INSERT INTO job_attempts (job_id, attempt_no, worker_id, status)
        SELECT job_id, 1, 'api-test-worker', 'started'
        FROM jobs
        WHERE generation_id = $1 AND kind = 'search.sync_once'
        ORDER BY created_at ASC
        LIMIT 1
        ",
    )
    .bind(generation_id)
    .execute(pool)
    .await?;
    Ok(())
}

fn assert_count_at_least(
    body: &Value,
    pointer: &str,
    minimum: i64,
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(value) = body.pointer(pointer).and_then(Value::as_i64) else {
        return Err(format!("missing count at {pointer}").into());
    };
    if value < minimum {
        return Err(format!("count at {pointer} was {value}, expected at least {minimum}").into());
    }
    Ok(())
}

async fn cleanup(pool: &PgPool, repo_id: &str) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    let _locked_generations = sqlx::query(
        r"
        SELECT generation_id
        FROM index_generations
        WHERE repo_id = $1
        FOR UPDATE
        ",
    )
    .bind(repo_id)
    .fetch_all(&mut *tx)
    .await?;
    sqlx::query(
        r"
        DELETE FROM jobs
        WHERE generation_id IN (
            SELECT generation_id FROM index_generations WHERE repo_id = $1
        )
        ",
    )
    .bind(repo_id)
    .execute(&mut *tx)
    .await?;
    for table in [
        "search_sync_outbox",
        "architecture_entities",
        "test_cases",
        "graph_edges",
        "graph_nodes",
        "symbols",
        "file_manifests",
        "index_generations",
        "commits",
        "repos",
    ] {
        sqlx::query(&format!("DELETE FROM {table} WHERE repo_id = $1"))
            .bind(repo_id)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(())
}

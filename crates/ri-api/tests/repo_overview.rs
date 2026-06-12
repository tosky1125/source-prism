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
async fn get_repo_returns_latest_index_evidence_counts() -> Result<(), Box<dyn std::error::Error>> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };
    let repo = TempRepo::create()?;
    repo.write_file(
        "src/lib.rs",
        r"
pub fn overview_fixture() -> i32 {
    7
}

#[test]
fn overview_fixture_is_indexed() {
    assert_eq!(overview_fixture(), 7);
}
",
    )?;
    repo.commit()?;
    let repo_id = format!("api-overview-{}", unique_suffix()?);
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
    let request = Request::builder()
        .method(Method::GET)
        .uri(format!("/v1/repos/{repo_id}"))
        .body(Body::empty())?;

    let response = app.oneshot(request).await?;

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), 1_000_000).await?;
    let body = serde_json::from_slice::<Value>(&bytes)?;
    assert_eq!(body.pointer("/kind").and_then(Value::as_str), Some("repo"));
    assert_eq!(
        body.pointer("/repo/repo_id").and_then(Value::as_str),
        Some(repo_id.as_str())
    );
    assert_eq!(
        body.pointer("/latest_run/status").and_then(Value::as_str),
        Some("succeeded")
    );
    assert_count_at_least(&body, "/latest_run/evidence/file_manifests", 1)?;
    assert_count_at_least(&body, "/latest_run/evidence/symbols", 2)?;
    assert_count_at_least(&body, "/latest_run/evidence/test_cases", 1)?;
    cleanup(&pool, &repo_id).await?;
    repo.cleanup()?;
    Ok(())
}

struct TempRepo {
    path: PathBuf,
}

impl TempRepo {
    fn create() -> Result<Self, Box<dyn std::error::Error>> {
        let path =
            std::env::temp_dir().join(format!("source-prism-api-overview-{}", unique_suffix()?));
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
    sqlx::query(
        "DELETE FROM job_attempts WHERE job_id IN (SELECT job_id FROM jobs WHERE generation_id IN (SELECT generation_id FROM index_generations WHERE repo_id = $1))",
    )
    .bind(repo_id)
    .execute(pool)
    .await?;
    sqlx::query(
        "DELETE FROM jobs WHERE generation_id IN (SELECT generation_id FROM index_generations WHERE repo_id = $1)",
    )
    .bind(repo_id)
    .execute(pool)
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
            .execute(pool)
            .await?;
    }
    Ok(())
}

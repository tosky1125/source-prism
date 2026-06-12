#![allow(missing_docs, reason = "Integration test names document behavior.")]

use axum::{
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode, header},
};
use ri_api::{AppState, app};
use serde_json::Value;
use sqlx::{PgPool, Row as _};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};
use tower::ServiceExt;

#[tokio::test]
async fn index_repo_uses_request_repo_path() -> Result<(), Box<dyn std::error::Error>> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };
    let repo = TempRepo::create()?;
    repo.write_file(
        "src/lib.rs",
        r"
pub fn api_index_fixture() -> i32 {
    7
}
",
    )?;
    repo.commit()?;
    let repo_id = format!("api-index-{}", unique_suffix()?);
    let search_sync_queue = format!("api-index-search-{}", unique_suffix()?);
    let app = app(AppState::for_test_database(pool.clone())?);
    let index_request_body = serde_json::json!({
        "sha": "HEAD",
        "repo_path": repo.path(),
        "search_sync_queue": search_sync_queue.clone(),
    });
    let request = Request::builder()
        .method(Method::POST)
        .uri(format!("/v1/repos/{repo_id}/index"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(index_request_body.to_string()))?;

    let response = app.clone().oneshot(request).await?;

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), 1_000_000).await?;
    let body = serde_json::from_slice::<Value>(&bytes)?;
    assert_eq!(
        body.pointer("/repo_id").and_then(Value::as_str),
        Some(repo_id.as_str())
    );
    let first_generation = body
        .pointer("/generation_id")
        .and_then(Value::as_str)
        .ok_or_else(|| std::io::Error::other("missing generation_id"))?;
    let first_search_chunks = body
        .pointer("/indexed_search_chunks")
        .and_then(Value::as_i64)
        .ok_or_else(|| std::io::Error::other("missing indexed_search_chunks"))?;
    assert!(first_search_chunks > 0);
    assert_eq!(
        search_chunk_count_for_generation(&pool, first_generation).await?,
        first_search_chunks
    );
    assert_eq!(
        indexed_file_path(&pool, &repo_id).await?,
        Some("src/lib.rs".to_owned())
    );

    let second_request = Request::builder()
        .method(Method::POST)
        .uri(format!("/v1/repos/{repo_id}/index"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(index_request_body.to_string()))?;

    let second_response = app.oneshot(second_request).await?;

    assert_eq!(second_response.status(), StatusCode::OK);
    let second_bytes = to_bytes(second_response.into_body(), 1_000_000).await?;
    let second_body = serde_json::from_slice::<Value>(&second_bytes)?;
    let second_generation = second_body
        .pointer("/generation_id")
        .and_then(Value::as_str)
        .ok_or_else(|| std::io::Error::other("missing second generation_id"))?;
    let second_search_chunks = second_body
        .pointer("/indexed_search_chunks")
        .and_then(Value::as_i64)
        .ok_or_else(|| std::io::Error::other("missing second indexed_search_chunks"))?;
    let enqueued_jobs = second_body
        .pointer("/enqueued_search_sync_jobs")
        .and_then(Value::as_i64)
        .ok_or_else(|| std::io::Error::other("missing enqueued_search_sync_jobs"))?;
    assert_eq!(
        second_body
            .pointer("/search_sync_queue")
            .and_then(Value::as_str),
        Some(search_sync_queue.as_str())
    );
    assert!(second_search_chunks > 0);
    assert_eq!(enqueued_jobs, 1);
    assert_ne!(second_generation, first_generation);
    assert_eq!(
        search_chunk_count_for_generation(&pool, second_generation).await?,
        second_search_chunks
    );
    assert_eq!(
        search_sync_job_count_for_generation(&pool, second_generation, &search_sync_queue).await?,
        1
    );
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
            std::env::temp_dir().join(format!("source-prism-api-index-{}", unique_suffix()?));
        fs::create_dir_all(path.join("src"))?;
        run_git(&path, ["init"])?;
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
        run_git(
            &self.path,
            [
                "-c",
                "user.email=source-prism@example.invalid",
                "-c",
                "user.name=Source Prism Test",
                "commit",
                "-m",
                "fixture",
            ],
        )?;
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

async fn indexed_file_path(pool: &PgPool, repo_id: &str) -> Result<Option<String>, sqlx::Error> {
    let row = sqlx::query(
        r"
        SELECT file_path
        FROM file_manifests
        WHERE repo_id = $1 AND stale_at IS NULL
        ORDER BY file_path
        LIMIT 1
        ",
    )
    .bind(repo_id)
    .fetch_optional(pool)
    .await?;
    row.map(|row| row.try_get("file_path")).transpose()
}

async fn search_chunk_count_for_generation(
    pool: &PgPool,
    generation_id: &str,
) -> Result<i64, sqlx::Error> {
    let row = sqlx::query(
        r"
        SELECT count(*)::bigint AS count
        FROM search_sync_outbox
        WHERE generation_id = $1 AND state <> 'cancelled'
        ",
    )
    .bind(generation_id)
    .fetch_one(pool)
    .await?;
    row.try_get("count")
}

async fn search_sync_job_count_for_generation(
    pool: &PgPool,
    generation_id: &str,
    queue: &str,
) -> Result<i64, sqlx::Error> {
    let row = sqlx::query(
        r"
        SELECT count(*)::bigint AS count
        FROM jobs
        WHERE generation_id = $1
          AND queue = $2
          AND kind = 'search.sync_once'
          AND state = 'queued'
          AND payload->>'generation_id' = $1
        ",
    )
    .bind(generation_id)
    .bind(queue)
    .fetch_one(pool)
    .await?;
    row.try_get("count")
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

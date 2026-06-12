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
    time::{SystemTime, UNIX_EPOCH},
};
use support::{Fixture, symbol, test_pool};
use tower::ServiceExt;

pub mod support;

#[tokio::test]
async fn repo_changed_symbols_maps_diff_lines_to_innermost_symbols()
-> Result<(), Box<dyn std::error::Error>> {
    let app = app(AppState::for_test_symbols(vec![
        symbol("src/invoice.rs", "InvoiceService::apply_tax")?,
        symbol("src/other.rs", "OtherService::noop")?,
    ])?);
    let diff = "\
diff --git a/src/invoice.rs b/src/invoice.rs
--- a/src/invoice.rs
+++ b/src/invoice.rs
@@ -1,2 +1,2 @@
+fn apply_tax() {}
 context
";
    let request = Request::builder()
        .method(Method::POST)
        .uri("/v1/repos/local/changed-symbols")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::json!({ "diff": diff }).to_string()))?;

    let response = app.oneshot(request).await?;

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), 1_000_000).await?;
    let body = serde_json::from_slice::<Value>(&bytes)?;
    assert_eq!(body.pointer("/status").and_then(Value::as_str), Some("ok"));
    assert_eq!(
        body.pointer("/kind").and_then(Value::as_str),
        Some("changed_symbols")
    );
    assert_eq!(
        body.pointer("/repo_id").and_then(Value::as_str),
        Some("local")
    );
    assert_eq!(
        body.pointer("/changed_line_count").and_then(Value::as_u64),
        Some(1)
    );
    assert_eq!(
        body.pointer("/matched_symbol_count")
            .and_then(Value::as_u64),
        Some(1)
    );
    assert_eq!(
        body.pointer("/changed_symbols/0/symbol/fqn")
            .and_then(Value::as_str),
        Some("InvoiceService::apply_tax")
    );
    assert_eq!(
        body.pointer("/changed_file_count").and_then(Value::as_u64),
        Some(1)
    );
    assert_eq!(
        body.pointer("/changed_files/0/path")
            .and_then(Value::as_str),
        Some("src/invoice.rs")
    );
    assert_eq!(
        body.pointer("/changed_files/0/status")
            .and_then(Value::as_str),
        Some("modified")
    );
    Ok(())
}

#[tokio::test]
async fn repo_changed_symbols_persists_head_overlay_without_reindexing()
-> Result<(), Box<dyn std::error::Error>> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };
    let fixture = Fixture::create(&pool).await?;
    fixture.seed_search_symbol(&pool, "file_manifest").await?;
    let repo = TempRepo::create()?;
    repo.write_file(
        "src/invoice.rs",
        r"
fn apply_tax() {
    let tax = 1;
}
",
    )?;
    let app = app(AppState::for_test_database_and_repo_path(
        pool.clone(),
        repo.path().to_path_buf(),
    )?);
    let base_generation_count = generation_count(&pool, fixture.repo_id.as_str()).await?;
    let diff = "\
diff --git a/src/invoice.rs b/src/invoice.rs
--- a/src/invoice.rs
+++ b/src/invoice.rs
@@ -1,2 +1,3 @@
 fn apply_tax() {
+    let fee = 1;
 }
";
    let request = Request::builder()
        .method(Method::POST)
        .uri(format!("/v1/repos/{}/changed-symbols", fixture.repo_id))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            serde_json::json!({
                "diff": diff,
                "persist_overlay": true,
                "head_sha": "worktree",
            })
            .to_string(),
        ))?;

    let response = app.oneshot(request).await?;

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), 1_000_000).await?;
    let body = serde_json::from_slice::<Value>(&bytes)?;
    assert_eq!(
        body.pointer("/overlay_index/indexed_file_count")
            .and_then(Value::as_u64),
        Some(1)
    );
    assert_eq!(
        body.pointer("/overlay_index/head_sha")
            .and_then(Value::as_str),
        Some("worktree")
    );
    assert_eq!(
        generation_count(&pool, fixture.repo_id.as_str()).await?,
        base_generation_count
    );
    assert_eq!(overlay_count(&pool, fixture.repo_id.as_str()).await?, 1);
    fixture.cleanup(&pool).await?;
    repo.cleanup()?;
    Ok(())
}

struct TempRepo {
    path: PathBuf,
}

impl TempRepo {
    fn create() -> Result<Self, Box<dyn std::error::Error>> {
        let suffix = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let path = std::env::temp_dir().join(format!("source-prism-api-overlay-{suffix}"));
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

    fn cleanup(&self) -> Result<(), std::io::Error> {
        fs::remove_dir_all(&self.path)
    }
}

fn run_git<const N: usize>(path: &Path, args: [&str; N]) -> Result<(), Box<dyn std::error::Error>> {
    let output = std::process::Command::new("git")
        .current_dir(path)
        .args(args)
        .output()?;
    if output.status.success() {
        return Ok(());
    }
    Err(std::io::Error::other(String::from_utf8_lossy(&output.stderr).to_string()).into())
}

async fn generation_count(pool: &PgPool, repo_id: &str) -> Result<i64, sqlx::Error> {
    let row =
        sqlx::query("SELECT count(*)::bigint AS count FROM index_generations WHERE repo_id = $1")
            .bind(repo_id)
            .fetch_one(pool)
            .await?;
    row.try_get("count")
}

async fn overlay_count(pool: &PgPool, repo_id: &str) -> Result<i64, sqlx::Error> {
    let row = sqlx::query("SELECT count(*)::bigint AS count FROM file_overlays WHERE repo_id = $1")
        .bind(repo_id)
        .fetch_one(pool)
        .await?;
    row.try_get("count")
}

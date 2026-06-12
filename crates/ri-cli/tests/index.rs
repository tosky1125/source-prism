#![allow(missing_docs, reason = "CLI integration test names document behavior.")]

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use serde_json::Value;
use sqlx::{PgPool, Row as _};

#[tokio::test]
async fn index_command_persists_symbols_graph_and_test_evidence()
-> Result<(), Box<dyn std::error::Error>> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };
    let repo = TempRepo::create()?;
    repo.write_file(
        "src/lib.rs",
        r"
mod invoice;

#[test]
fn apply_tax_adds_rate() {
    assert_eq!(invoice::apply_tax(1), 2);
}
",
    )?;
    repo.write_file(
        "src/invoice.rs",
        r"
pub fn apply_tax(value: i32) -> i32 {
    value + 1
}
",
    )?;
    repo.commit()?;

    let output = Command::new(env!("CARGO_BIN_EXE_ri-cli"))
        .current_dir(repo.path())
        .env(
            "DATABASE_URL",
            std::env::var("DATABASE_URL").unwrap_or_default(),
        )
        .args(["index", "--repo", ".", "--sha", "HEAD"])
        .output()?;

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let body = serde_json::from_slice::<Value>(&output.stdout)?;
    let repo_id = body
        .pointer("/repo_id")
        .and_then(Value::as_str)
        .ok_or_else(|| std::io::Error::other("missing repo_id"))?;
    assert_eq!(body.pointer("/kind").and_then(Value::as_str), Some("index"));
    assert_positive(&body, "/inserted_file_manifests")?;
    assert_positive(&body, "/indexed_symbols")?;
    assert_positive(&body, "/indexed_graph_edges")?;
    assert_positive(&body, "/indexed_import_edges")?;
    assert_positive(&body, "/indexed_call_edges")?;
    assert_positive(&body, "/indexed_test_cases")?;
    assert_positive(&body, "/indexed_test_cover_edges")?;
    assert_positive(&body, "/indexed_search_chunks")?;
    assert_eq!(active_count(&pool, repo_id, "symbols").await?, 3);
    assert_eq!(active_count(&pool, repo_id, "test_cases").await?, 1);
    assert_eq!(edge_count(&pool, repo_id, "imports").await?, 1);
    assert_eq!(edge_count(&pool, repo_id, "calls").await?, 1);
    assert_eq!(edge_count(&pool, repo_id, "test_covers").await?, 1);
    let impact_output = Command::new(env!("CARGO_BIN_EXE_ri-cli"))
        .current_dir(repo.path())
        .args(["impact", "--symbol", "apply_tax"])
        .output()?;
    assert!(
        impact_output.status.success(),
        "{}",
        String::from_utf8_lossy(&impact_output.stderr)
    );
    let impact_body = serde_json::from_slice::<Value>(&impact_output.stdout)?;
    assert_json_array_contains(&impact_body, "/direct_callers", "apply_tax_adds_rate")?;
    let context_output = Command::new(env!("CARGO_BIN_EXE_ri-cli"))
        .current_dir(repo.path())
        .args(["search-context", "apply_tax"])
        .output()?;
    assert!(
        context_output.status.success(),
        "{}",
        String::from_utf8_lossy(&context_output.stderr)
    );
    let context_body = serde_json::from_slice::<Value>(&context_output.stdout)?;
    assert_json_array_contains(
        &context_body,
        "/context_pack/impacts/0/direct_callers",
        "apply_tax_adds_rate",
    )?;
    cleanup(&pool, repo_id).await?;
    repo.cleanup()?;
    Ok(())
}

struct TempRepo {
    path: PathBuf,
}

impl TempRepo {
    fn create() -> Result<Self, Box<dyn std::error::Error>> {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_nanos()
            .to_string();
        let path = std::env::temp_dir().join(format!("source-prism-cli-index-{suffix}"));
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

fn run_git<const N: usize>(path: &Path, args: [&str; N]) -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new("git").current_dir(path).args(args).output()?;
    if output.status.success() {
        return Ok(());
    }
    Err(std::io::Error::other(String::from_utf8_lossy(&output.stderr).to_string()).into())
}

fn assert_positive(body: &Value, pointer: &str) -> Result<(), Box<dyn std::error::Error>> {
    let value = body
        .pointer(pointer)
        .and_then(Value::as_u64)
        .ok_or_else(|| std::io::Error::other(format!("missing positive {pointer}")))?;
    assert!(value > 0, "{pointer} should be positive");
    Ok(())
}

fn assert_json_array_contains(
    body: &Value,
    pointer: &str,
    expected: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let values = body
        .pointer(pointer)
        .and_then(Value::as_array)
        .ok_or_else(|| std::io::Error::other(format!("missing array {pointer}")))?;
    assert!(
        values.iter().any(|value| value.as_str() == Some(expected)),
        "{pointer} should contain {expected}"
    );
    Ok(())
}

async fn test_pool() -> Result<Option<PgPool>, sqlx::Error> {
    let Ok(database_url) = std::env::var("DATABASE_URL") else {
        return Ok(None);
    };
    PgPool::connect(database_url.as_str()).await.map(Some)
}

async fn active_count(pool: &PgPool, repo_id: &str, table: &str) -> Result<i64, sqlx::Error> {
    let row = sqlx::query(&format!(
        "SELECT count(*)::bigint AS count FROM {table} WHERE repo_id = $1 AND stale_at IS NULL"
    ))
    .bind(repo_id)
    .fetch_one(pool)
    .await?;
    row.try_get("count")
}

async fn edge_count(pool: &PgPool, repo_id: &str, edge_type: &str) -> Result<i64, sqlx::Error> {
    let row = sqlx::query(
        r"
        SELECT count(*)::bigint AS count
        FROM graph_edges
        WHERE repo_id = $1 AND edge_type = $2 AND stale_at IS NULL
        ",
    )
    .bind(repo_id)
    .bind(edge_type)
    .fetch_one(pool)
    .await?;
    row.try_get("count")
}

async fn cleanup(pool: &PgPool, repo_id: &str) -> Result<(), sqlx::Error> {
    for table in [
        "search_sync_outbox",
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

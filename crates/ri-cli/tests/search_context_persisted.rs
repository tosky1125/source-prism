#![allow(missing_docs, reason = "CLI integration test names document behavior.")]

use serde_json::Value;
use sqlx::PgPool;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[tokio::test]
async fn search_context_command_uses_persisted_repo_index() -> TestResult {
    let Some(database_url) = std::env::var("DATABASE_URL").ok() else {
        return Ok(());
    };
    let Some(opensearch_url) = std::env::var("OPENSEARCH_URL").ok() else {
        return Ok(());
    };
    let pool = PgPool::connect(database_url.as_str()).await?;
    let repo = TempRepo::create()?;
    repo.write_file(
        "src/lib.rs",
        r"
pub fn apply_tax(value: i32) -> i32 {
    value + 1
}

#[test]
fn apply_tax_adds_rate() {
    assert_eq!(apply_tax(1), 2);
}
",
    )?;
    repo.commit()?;
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");

    let index_output = Command::new(env!("CARGO_BIN_EXE_ri-cli"))
        .current_dir(&repo_root)
        .env("DATABASE_URL", database_url.as_str())
        .args(["index", "--repo"])
        .arg(repo.path())
        .args(["--sha", "HEAD"])
        .output()?;
    assert!(
        index_output.status.success(),
        "{}",
        String::from_utf8_lossy(&index_output.stderr)
    );
    let index_body = serde_json::from_slice::<Value>(&index_output.stdout)?;
    let repo_id = index_body
        .pointer("/repo_id")
        .and_then(Value::as_str)
        .ok_or_else(|| std::io::Error::other("missing repo_id"))?;
    let generation_id = index_body
        .pointer("/generation_id")
        .and_then(Value::as_str)
        .ok_or_else(|| std::io::Error::other("missing generation_id"))?;
    let rebuild = Command::new(env!("CARGO_BIN_EXE_ri-cli"))
        .current_dir(&repo_root)
        .env("DATABASE_URL", database_url.as_str())
        .env("OPENSEARCH_URL", opensearch_url.as_str())
        .args([
            "search",
            "rebuild",
            "--from-postgres",
            "--generation",
            generation_id,
        ])
        .output()?;
    assert!(
        rebuild.status.success(),
        "{}",
        String::from_utf8_lossy(&rebuild.stderr)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_ri-cli"))
        .current_dir(repo_root)
        .env("DATABASE_URL", database_url.as_str())
        .env("OPENSEARCH_URL", opensearch_url.as_str())
        .args(["search-context", "--repo-id", repo_id, "apply_tax"])
        .output()?;

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let body = serde_json::from_slice::<Value>(&output.stdout)?;
    assert_eq!(
        body.pointer("/kind").and_then(Value::as_str),
        Some("search_context")
    );
    assert_eq!(
        body.pointer("/context_pack/vector_only")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_positive(&body, "/search_chunk_count")?;
    assert_positive(&body, "/bm25_hit_count")?;
    assert_json_array_contains(
        &body,
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
    fn create() -> TestResult<Self> {
        let suffix = unique_suffix()?;
        let path = std::env::temp_dir().join(format!("source-prism-search-context-db-{suffix}"));
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

    fn commit(&self) -> TestResult {
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

fn run_git<const N: usize>(path: &Path, args: [&str; N]) -> TestResult {
    let output = Command::new("git").current_dir(path).args(args).output()?;
    if output.status.success() {
        return Ok(());
    }
    Err(std::io::Error::other(String::from_utf8_lossy(&output.stderr).to_string()).into())
}

fn assert_positive(body: &Value, pointer: &str) -> TestResult {
    let value = body
        .pointer(pointer)
        .and_then(Value::as_i64)
        .ok_or_else(|| std::io::Error::other(format!("missing positive {pointer}")))?;
    assert!(value > 0, "{pointer} should be positive");
    Ok(())
}

fn assert_json_array_contains(body: &Value, pointer: &str, expected: &str) -> TestResult {
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

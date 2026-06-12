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
async fn mcp_call_uses_persisted_repo_index() -> TestResult {
    let Some(database_url) = std::env::var("DATABASE_URL").ok() else {
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

    let repo_id = index_repo(&repo_root, &database_url, repo.path())?;
    let output = Command::new(env!("CARGO_BIN_EXE_ri-cli"))
        .current_dir(&repo_root)
        .env("DATABASE_URL", database_url.as_str())
        .args([
            "mcp",
            "call",
            "--repo-id",
            repo_id.as_str(),
            "--tool",
            "repo.get_impact",
            "--symbol",
            "apply_tax",
        ])
        .output()?;

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let body = serde_json::from_slice::<Value>(&output.stdout)?;
    assert_eq!(
        body.pointer("/kind").and_then(Value::as_str),
        Some("mcp_tool_result")
    );
    assert_eq!(
        body.pointer("/tool").and_then(Value::as_str),
        Some("repo.get_impact")
    );
    assert_eq!(
        body.pointer("/result/symbol/fqn").and_then(Value::as_str),
        Some("apply_tax")
    );
    assert_json_array_contains(&body, "/result/direct_callers", "apply_tax_adds_rate")?;
    cleanup(&pool, repo_id.as_str()).await?;
    repo.cleanup()?;
    Ok(())
}

fn index_repo(repo_root: &Path, database_url: &str, repo_path: &Path) -> TestResult<String> {
    let output = Command::new(env!("CARGO_BIN_EXE_ri-cli"))
        .current_dir(repo_root)
        .env("DATABASE_URL", database_url)
        .args(["index", "--repo"])
        .arg(repo_path)
        .args(["--sha", "HEAD"])
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
    Ok(repo_id.to_owned())
}

struct TempRepo {
    path: PathBuf,
}

impl TempRepo {
    fn create() -> TestResult<Self> {
        let suffix = unique_suffix()?;
        let path = std::env::temp_dir().join(format!("source-prism-mcp-db-{suffix}"));
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
    let mut tx = pool.begin().await?;
    sqlx::query(
        r"
        DELETE FROM job_attempts
        WHERE job_id IN (SELECT job_id FROM jobs WHERE generation_id IN (
            SELECT generation_id FROM index_generations WHERE repo_id = $1
        ))
        ",
    )
    .bind(repo_id)
    .execute(&mut *tx)
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

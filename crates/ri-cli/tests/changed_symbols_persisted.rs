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
async fn changed_symbols_command_uses_persisted_repo_index() -> TestResult {
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
",
    )?;
    repo.commit()?;
    let diff = TempFile::write(
        r"
diff --git a/src/lib.rs b/src/lib.rs
index 1111111..2222222 100644
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -1,3 +1,4 @@
 pub fn apply_tax(value: i32) -> i32 {
+    let fee = 1;
     value + 1
 }
",
    )?;
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let repo_id = index_repo(&repo_root, &database_url, repo.path())?;

    let output = Command::new(env!("CARGO_BIN_EXE_ri-cli"))
        .current_dir(&repo_root)
        .env("DATABASE_URL", database_url.as_str())
        .args(["changed-symbols", "--repo-id", repo_id.as_str(), "--diff"])
        .arg(diff.path())
        .output()?;

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let body = serde_json::from_slice::<Value>(&output.stdout)?;
    assert_eq!(
        body.pointer("/kind").and_then(Value::as_str),
        Some("changed_symbols")
    );
    assert_eq!(
        body.pointer("/repo_id").and_then(Value::as_str),
        Some(repo_id.as_str())
    );
    assert_eq!(
        body.pointer("/changed_symbols/0/symbol/fqn")
            .and_then(Value::as_str),
        Some("apply_tax")
    );
    cleanup(&pool, repo_id.as_str()).await?;
    diff.cleanup()?;
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
        let path = std::env::temp_dir().join(format!("source-prism-changed-symbols-db-{suffix}"));
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

struct TempFile {
    path: PathBuf,
}

impl TempFile {
    fn write(body: &str) -> TestResult<Self> {
        let suffix = unique_suffix()?;
        let path = std::env::temp_dir().join(format!("source-prism-changed-symbols-{suffix}.diff"));
        fs::write(&path, body)?;
        Ok(Self { path })
    }

    fn path(&self) -> &Path {
        self.path.as_path()
    }

    fn cleanup(&self) -> Result<(), std::io::Error> {
        fs::remove_file(&self.path)
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

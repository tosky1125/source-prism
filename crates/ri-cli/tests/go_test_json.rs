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
async fn tests_import_go_test_json_persists_test_run_results()
-> Result<(), Box<dyn std::error::Error>> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };
    let repo = TempRepo::create()?;
    repo.write_file(
        "invoice_test.go",
        "package invoice\n\nfunc TestAddsRate(t *testing.T) {}\n",
    )?;
    repo.commit()?;
    repo.write_file(
        "go-test.json",
        r#"{"Action":"pass","Package":"example.com/invoice","Test":"TestAddsRate","Elapsed":0.005}"#,
    )?;

    let output = Command::new(env!("CARGO_BIN_EXE_ri-cli"))
        .current_dir(repo.path())
        .env(
            "DATABASE_URL",
            std::env::var("DATABASE_URL").unwrap_or_default(),
        )
        .args([
            "tests",
            "import-go-test-json",
            "--repo",
            ".",
            "--sha",
            "HEAD",
            "--go-test-json",
            "go-test.json",
        ])
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
    assert_eq!(
        body.pointer("/framework").and_then(Value::as_str),
        Some("go_test")
    );
    assert_eq!(
        body.pointer("/imported_results").and_then(Value::as_u64),
        Some(1)
    );
    assert_eq!(active_go_test_result_count(&pool, repo_id).await?, 1);
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
        let path = std::env::temp_dir().join(format!("source-prism-cli-go-test-{suffix}"));
        fs::create_dir_all(&path)?;
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

async fn test_pool() -> Result<Option<PgPool>, sqlx::Error> {
    let Ok(database_url) = std::env::var("DATABASE_URL") else {
        return Ok(None);
    };
    PgPool::connect(database_url.as_str()).await.map(Some)
}

async fn active_go_test_result_count(pool: &PgPool, repo_id: &str) -> Result<i64, sqlx::Error> {
    let row = sqlx::query(
        "SELECT count(*)::bigint AS count FROM test_results results JOIN test_runs runs ON results.test_run_id = runs.test_run_id WHERE results.repo_id = $1 AND runs.framework = 'go_test' AND results.stale_at IS NULL",
    )
    .bind(repo_id)
    .fetch_one(pool)
    .await?;
    row.try_get("count")
}

async fn cleanup(pool: &PgPool, repo_id: &str) -> Result<(), sqlx::Error> {
    for table in [
        "test_results",
        "test_runs",
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

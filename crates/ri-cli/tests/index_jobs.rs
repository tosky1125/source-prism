#![allow(missing_docs, reason = "CLI integration test names document behavior.")]

use serde_json::Value;
use sqlx::{PgPool, Row as _};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

#[tokio::test]
async fn index_command_enqueues_search_sync_job() -> Result<(), Box<dyn std::error::Error>> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };
    let repo = TempRepo::create()?;
    repo.write_file("src/lib.rs", "pub fn indexed_for_search() -> i32 { 1 }\n")?;
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
    let generation_id = body
        .pointer("/generation_id")
        .and_then(Value::as_str)
        .ok_or_else(|| std::io::Error::other("missing generation_id"))?;
    assert_eq!(
        body.pointer("/search_sync_queue").and_then(Value::as_str),
        Some("default")
    );
    assert_eq!(
        body.pointer("/enqueued_search_sync_jobs")
            .and_then(Value::as_u64),
        Some(1)
    );
    let job_id = search_sync_job_id(&pool, generation_id).await?;
    let _parsed = uuid::Uuid::parse_str(&job_id)?;
    cleanup(&pool, repo_id).await?;
    repo.cleanup()?;
    Ok(())
}

struct TempRepo {
    path: PathBuf,
}

impl TempRepo {
    fn create() -> Result<Self, Box<dyn std::error::Error>> {
        let path = std::env::temp_dir().join(format!("source-prism-cli-index-jobs-{}", suffix()?));
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

fn suffix() -> Result<String, std::time::SystemTimeError> {
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

async fn search_sync_job_id(pool: &PgPool, generation_id: &str) -> Result<String, sqlx::Error> {
    let row = sqlx::query(
        r"
        SELECT job_id
        FROM jobs
        WHERE generation_id = $1 AND kind = 'search.sync_once'
        ORDER BY created_at ASC
        LIMIT 1
        ",
    )
    .bind(generation_id)
    .fetch_one(pool)
    .await?;
    row.try_get("job_id")
}

async fn cleanup(pool: &PgPool, repo_id: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        r"
        DELETE FROM jobs
        WHERE generation_id IN (
            SELECT generation_id FROM index_generations WHERE repo_id = $1
        )
        ",
    )
    .bind(repo_id)
    .execute(pool)
    .await?;
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

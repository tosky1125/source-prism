#![allow(missing_docs, reason = "CLI integration test names document behavior.")]

use ri_indexer::{DEFAULT_SEARCH_INDEX, PgSearchSyncStore, SearchSyncInput};
use serde_json::{Value, json};
use sqlx::PgPool;
use std::{
    path::Path,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[tokio::test]
async fn repo_search_drift_command_reports_latest_generation_drift() -> TestResult {
    let Some(database_url) = std::env::var("DATABASE_URL").ok() else {
        return Ok(());
    };
    let Some(opensearch_url) = std::env::var("OPENSEARCH_URL").ok() else {
        return Ok(());
    };
    let pool = PgPool::connect(database_url.as_str()).await?;
    let fixture = Fixture::create(&pool).await?;
    fixture.seed_search_chunk(&pool).await?;
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");

    let rebuild = Command::new(env!("CARGO_BIN_EXE_ri-cli"))
        .current_dir(&repo_root)
        .env("DATABASE_URL", database_url.as_str())
        .env("OPENSEARCH_URL", opensearch_url.as_str())
        .args([
            "search",
            "rebuild",
            "--from-postgres",
            "--generation",
            fixture.generation_id.as_str(),
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
        .args(["repo-search-drift", "--repo-id", fixture.repo_id.as_str()])
        .output()?;

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let body = serde_json::from_slice::<Value>(&output.stdout)?;
    assert_eq!(
        body.pointer("/kind").and_then(Value::as_str),
        Some("repo_search_drift")
    );
    assert_eq!(
        body.pointer("/latest_generation_id")
            .and_then(Value::as_str),
        Some(fixture.generation_id.as_str())
    );
    assert_eq!(
        body.pointer("/expected_documents").and_then(Value::as_i64),
        Some(1)
    );
    assert_eq!(
        body.pointer("/actual_documents").and_then(Value::as_i64),
        Some(1)
    );
    assert_eq!(
        body.pointer("/has_drift").and_then(Value::as_bool),
        Some(false)
    );
    fixture.cleanup(&pool).await?;
    Ok(())
}

struct Fixture {
    repo_id: String,
    commit_sha: String,
    generation_id: String,
}

impl Fixture {
    async fn create(pool: &PgPool) -> TestResult<Self> {
        let suffix = unique_suffix()?;
        let fixture = Self {
            repo_id: format!("cli-repo-search-drift-{suffix}"),
            commit_sha: format!("commit-{suffix}"),
            generation_id: format!("generation-{suffix}"),
        };
        sqlx::query("INSERT INTO repos (repo_id, name) VALUES ($1, $1)")
            .bind(&fixture.repo_id)
            .execute(pool)
            .await?;
        sqlx::query("INSERT INTO commits (repo_id, commit_sha) VALUES ($1, $2)")
            .bind(&fixture.repo_id)
            .bind(&fixture.commit_sha)
            .execute(pool)
            .await?;
        sqlx::query(
            r"
            INSERT INTO index_generations (
                generation_id, repo_id, commit_sha, index_kind, status, finished_at
            )
            VALUES ($1, $2, $3, 'file_manifest', 'succeeded', now())
            ",
        )
        .bind(&fixture.generation_id)
        .bind(&fixture.repo_id)
        .bind(&fixture.commit_sha)
        .execute(pool)
        .await?;
        Ok(fixture)
    }

    async fn seed_search_chunk(&self, pool: &PgPool) -> TestResult {
        let entity_id = format!("chunk-{}", self.generation_id);
        let input = SearchSyncInput::upsert_for_generation(
            &self.repo_id,
            &self.generation_id,
            "symbol_chunk",
            &entity_id,
            DEFAULT_SEARCH_INDEX,
            json!({
                "chunk_id": entity_id,
                "repo_id": self.repo_id,
                "generation_id": self.generation_id,
                "text": "repo search drift smoke"
            }),
        );
        PgSearchSyncStore::new(pool.clone()).enqueue(&input).await?;
        Ok(())
    }

    async fn cleanup(&self, pool: &PgPool) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM search_sync_outbox WHERE repo_id = $1")
            .bind(&self.repo_id)
            .execute(pool)
            .await?;
        sqlx::query("DELETE FROM index_generations WHERE repo_id = $1")
            .bind(&self.repo_id)
            .execute(pool)
            .await?;
        sqlx::query("DELETE FROM commits WHERE repo_id = $1")
            .bind(&self.repo_id)
            .execute(pool)
            .await?;
        sqlx::query("DELETE FROM repos WHERE repo_id = $1")
            .bind(&self.repo_id)
            .execute(pool)
            .await?;
        Ok(())
    }
}

fn unique_suffix() -> Result<String, std::time::SystemTimeError> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_nanos()
        .to_string())
}

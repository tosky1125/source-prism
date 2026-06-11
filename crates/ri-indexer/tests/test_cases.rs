#![allow(
    missing_docs,
    reason = "Integration tests use scenario names instead of API docs."
)]
#![allow(
    clippy::multiple_crate_versions,
    reason = "SQLx TLS dependencies currently pull duplicate platform crates outside this crate's control."
)]

use ri_core::{CommitSha, FilePath, Language, RepoId, SymbolKind};
use ri_indexer::{PgGenerationStore, PgTestCaseStore};
use ri_symbols::{SymbolRange, SymbolRecord, SymbolSpec};
use sqlx::PgPool;
use uuid::Uuid;

#[tokio::test]
async fn active_test_cases_returns_latest_generation_tests()
-> Result<(), Box<dyn std::error::Error>> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };
    let fixture = Fixture::create(&pool).await?;
    let generation = PgGenerationStore::new(pool.clone())
        .begin_generation(
            &fixture.repo_id,
            &fixture.commit_sha,
            "test_cases",
            Some("test"),
        )
        .await?;
    let store = PgTestCaseStore::new(pool.clone());
    let indexed = store
        .replace_test_cases_for_generation(
            &generation.generation_id,
            &[symbol(&fixture, SymbolKind::TestCase, "adds_tax")?],
        )
        .await?;
    PgGenerationStore::new(pool.clone())
        .finish_generation(&generation.generation_id)
        .await?;

    let tests = store.active_test_cases_for_repo(&fixture.repo_id).await?;

    assert_eq!(indexed, 1);
    assert_eq!(tests.len(), 1);
    let test = tests
        .first()
        .ok_or_else(|| std::io::Error::other("expected one test case"))?;
    assert_eq!(test.fqn, "adds_tax");
    assert_eq!(test.file_path, "tests/invoice.rs");
    fixture.cleanup(&pool).await?;
    Ok(())
}

#[derive(Debug)]
struct Fixture {
    repo_id: String,
    commit_sha: String,
}

impl Fixture {
    async fn create(pool: &PgPool) -> Result<Self, sqlx::Error> {
        let suffix = Uuid::now_v7();
        let fixture = Self {
            repo_id: format!("repo-{suffix}"),
            commit_sha: format!("commit-{suffix}"),
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
        Ok(fixture)
    }

    async fn cleanup(&self, pool: &PgPool) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM test_cases WHERE repo_id = $1")
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

async fn test_pool() -> Result<Option<PgPool>, sqlx::Error> {
    let Ok(database_url) = std::env::var("DATABASE_URL") else {
        return Ok(None);
    };
    PgPool::connect(database_url.as_str()).await.map(Some)
}

fn symbol(
    fixture: &Fixture,
    kind: SymbolKind,
    fqn: &str,
) -> Result<SymbolRecord, ri_core::CoreError> {
    Ok(SymbolRecord::new(
        &RepoId::new(&fixture.repo_id)?,
        &CommitSha::new(&fixture.commit_sha)?,
        FilePath::new("tests/invoice.rs")?,
        "hash",
        SymbolSpec::new(Language::Rust, kind, fqn, fqn, SymbolRange::new(1, 0, 2, 0)),
    ))
}

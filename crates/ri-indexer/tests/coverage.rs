#![allow(
    missing_docs,
    reason = "Integration tests use scenario names instead of API docs."
)]
#![allow(
    clippy::multiple_crate_versions,
    reason = "SQLx TLS dependencies currently pull duplicate platform crates outside this crate's control."
)]

use ri_behavior::parse_lcov;
use ri_indexer::{PgCoverageStore, PgGenerationStore};
use sqlx::PgPool;
use uuid::Uuid;

#[tokio::test]
async fn active_coverage_returns_lcov_segments_for_repo() -> Result<(), Box<dyn std::error::Error>>
{
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };
    let fixture = Fixture::create(&pool).await?;
    let generation = PgGenerationStore::new(pool.clone())
        .begin_generation(
            &fixture.repo_id,
            &fixture.commit_sha,
            "coverage",
            Some("test"),
        )
        .await?;
    let report = parse_lcov("SF:src/invoice.rs\nDA:3,1\nend_of_record\n")?;
    let store = PgCoverageStore::new(pool.clone());
    let outcome = store
        .replace_lcov_for_generation(&generation.generation_id, "lcov.info", &report)
        .await?;
    PgGenerationStore::new(pool.clone())
        .finish_generation(&generation.generation_id)
        .await?;

    let segments = store
        .active_coverage_segments_for_repo(&fixture.repo_id)
        .await?;

    assert_eq!(outcome.segment_count, 1);
    assert_eq!(segments.len(), 1);
    let segment = segments
        .first()
        .ok_or_else(|| std::io::Error::other("missing coverage segment"))?;
    assert_eq!(segment.file_path, "src/invoice.rs");
    assert_eq!(segment.start_line, 3);
    assert_eq!(segment.hit_count, 1);
    fixture.cleanup(&pool).await?;
    Ok(())
}

#[tokio::test]
async fn active_coverage_preserves_cobertura_format_for_repo()
-> Result<(), Box<dyn std::error::Error>> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };
    let fixture = Fixture::create(&pool).await?;
    let generation = PgGenerationStore::new(pool.clone())
        .begin_generation(
            &fixture.repo_id,
            &fixture.commit_sha,
            "coverage",
            Some("test"),
        )
        .await?;
    let report = parse_lcov("SF:src/invoice.rs\nDA:3,1\nend_of_record\n")?;
    let store = PgCoverageStore::new(pool.clone());
    let outcome = store
        .replace_cobertura_for_generation(&generation.generation_id, "coverage.xml", &report)
        .await?;
    PgGenerationStore::new(pool.clone())
        .finish_generation(&generation.generation_id)
        .await?;

    let segments = store
        .active_coverage_segments_for_repo(&fixture.repo_id)
        .await?;

    assert_eq!(outcome.segment_count, 1);
    let segment = segments
        .first()
        .ok_or_else(|| std::io::Error::other("missing coverage segment"))?;
    assert_eq!(segment.format, "cobertura");
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
        for table in ["coverage_segments", "index_generations", "commits", "repos"] {
            sqlx::query(&format!("DELETE FROM {table} WHERE repo_id = $1"))
                .bind(&self.repo_id)
                .execute(pool)
                .await?;
        }
        Ok(())
    }
}

async fn test_pool() -> Result<Option<PgPool>, sqlx::Error> {
    let Ok(database_url) = std::env::var("DATABASE_URL") else {
        return Ok(None);
    };
    PgPool::connect(database_url.as_str()).await.map(Some)
}

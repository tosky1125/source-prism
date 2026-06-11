#![allow(
    missing_docs,
    reason = "Integration tests use scenario names instead of API docs."
)]
#![allow(
    clippy::multiple_crate_versions,
    reason = "SQLx TLS dependencies currently pull duplicate platform crates outside this crate's control."
)]

use ri_indexer::{FileManifestInput, PgGenerationStore};
use sqlx::{PgPool, Row as _};
use uuid::Uuid;

#[tokio::test]
async fn successful_generation_stales_missing_previous_file_manifests()
-> Result<(), Box<dyn std::error::Error>> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };
    let fixture = Fixture::create(&pool).await?;
    let store = PgGenerationStore::new(pool.clone());

    let first = store
        .begin_generation(
            &fixture.repo_id,
            &fixture.commit_sha,
            "file_manifest",
            Some("test"),
        )
        .await?;
    store
        .replace_file_manifest_generation(
            &first.generation_id,
            &[
                FileManifestInput::new("src/a.rs", "hash-a-1", 10),
                FileManifestInput::new("src/b.rs", "hash-b-1", 20),
            ],
        )
        .await?;

    let second = store
        .begin_generation(
            &fixture.repo_id,
            &fixture.commit_sha,
            "file_manifest",
            Some("test"),
        )
        .await?;
    store
        .replace_file_manifest_generation(
            &second.generation_id,
            &[FileManifestInput::new("src/a.rs", "hash-a-2", 11)],
        )
        .await?;

    assert_eq!(active_manifest_count(&pool, &fixture).await?, 1);
    assert_eq!(active_path_count(&pool, &fixture, "src/a.rs").await?, 1);
    assert_eq!(active_path_count(&pool, &fixture, "src/b.rs").await?, 0);
    assert_eq!(stale_path_count(&pool, &fixture, "src/b.rs").await?, 1);
    assert_eq!(stale_path_count(&pool, &fixture, "src/a.rs").await?, 1);

    fixture.cleanup(&pool).await?;
    Ok(())
}

#[tokio::test]
async fn failed_generation_leaves_previous_active_file_manifests_untouched()
-> Result<(), Box<dyn std::error::Error>> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };
    let fixture = Fixture::create(&pool).await?;
    let store = PgGenerationStore::new(pool.clone());

    let first = store
        .begin_generation(
            &fixture.repo_id,
            &fixture.commit_sha,
            "file_manifest",
            Some("test"),
        )
        .await?;
    store
        .replace_file_manifest_generation(
            &first.generation_id,
            &[
                FileManifestInput::new("src/a.rs", "hash-a-1", 10),
                FileManifestInput::new("src/b.rs", "hash-b-1", 20),
            ],
        )
        .await?;

    let failed = store
        .begin_generation(
            &fixture.repo_id,
            &fixture.commit_sha,
            "file_manifest",
            Some("test"),
        )
        .await?;
    store
        .fail_generation(&failed.generation_id, "simulated extractor failure")
        .await?;

    assert_eq!(active_manifest_count(&pool, &fixture).await?, 2);
    assert_eq!(active_path_count(&pool, &fixture, "src/a.rs").await?, 1);
    assert_eq!(active_path_count(&pool, &fixture, "src/b.rs").await?, 1);
    assert_eq!(stale_manifest_count(&pool, &fixture).await?, 0);

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
        sqlx::query(
            r"
            INSERT INTO repos (repo_id, name)
            VALUES ($1, $2)
            ",
        )
        .bind(&fixture.repo_id)
        .bind(&fixture.repo_id)
        .execute(pool)
        .await?;
        sqlx::query(
            r"
            INSERT INTO commits (repo_id, commit_sha)
            VALUES ($1, $2)
            ",
        )
        .bind(&fixture.repo_id)
        .bind(&fixture.commit_sha)
        .execute(pool)
        .await?;
        Ok(fixture)
    }

    async fn cleanup(&self, pool: &PgPool) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM file_manifests WHERE repo_id = $1 AND commit_sha = $2")
            .bind(&self.repo_id)
            .bind(&self.commit_sha)
            .execute(pool)
            .await?;
        sqlx::query("DELETE FROM index_generations WHERE repo_id = $1 AND commit_sha = $2")
            .bind(&self.repo_id)
            .bind(&self.commit_sha)
            .execute(pool)
            .await?;
        sqlx::query("DELETE FROM commits WHERE repo_id = $1 AND commit_sha = $2")
            .bind(&self.repo_id)
            .bind(&self.commit_sha)
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

async fn active_manifest_count(pool: &PgPool, fixture: &Fixture) -> Result<i64, sqlx::Error> {
    count_where(pool, fixture, "stale_at IS NULL").await
}

async fn stale_manifest_count(pool: &PgPool, fixture: &Fixture) -> Result<i64, sqlx::Error> {
    count_where(pool, fixture, "stale_at IS NOT NULL").await
}

async fn active_path_count(
    pool: &PgPool,
    fixture: &Fixture,
    file_path: &str,
) -> Result<i64, sqlx::Error> {
    count_path_where(pool, fixture, file_path, "stale_at IS NULL").await
}

async fn stale_path_count(
    pool: &PgPool,
    fixture: &Fixture,
    file_path: &str,
) -> Result<i64, sqlx::Error> {
    count_path_where(pool, fixture, file_path, "stale_at IS NOT NULL").await
}

async fn count_where(
    pool: &PgPool,
    fixture: &Fixture,
    stale_clause: &str,
) -> Result<i64, sqlx::Error> {
    let query = format!(
        "SELECT count(*)::bigint AS count FROM file_manifests WHERE repo_id = $1 AND commit_sha = $2 AND {stale_clause}"
    );
    let row = sqlx::query(query.as_str())
        .bind(&fixture.repo_id)
        .bind(&fixture.commit_sha)
        .fetch_one(pool)
        .await?;
    row.try_get("count")
}

async fn count_path_where(
    pool: &PgPool,
    fixture: &Fixture,
    file_path: &str,
    stale_clause: &str,
) -> Result<i64, sqlx::Error> {
    let query = format!(
        "SELECT count(*)::bigint AS count FROM file_manifests WHERE repo_id = $1 AND commit_sha = $2 AND file_path = $3 AND {stale_clause}"
    );
    let row = sqlx::query(query.as_str())
        .bind(&fixture.repo_id)
        .bind(&fixture.commit_sha)
        .bind(file_path)
        .fetch_one(pool)
        .await?;
    row.try_get("count")
}

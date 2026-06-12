#![allow(
    missing_docs,
    reason = "Integration tests use scenario names instead of API docs."
)]
#![allow(
    clippy::multiple_crate_versions,
    reason = "SQLx TLS dependencies currently pull duplicate platform crates outside this crate's control."
)]

use ri_architecture::{ArchitectureEntity, ArchitectureEntityKind, ArchitectureEntitySpec};
use ri_core::{CommitSha, FilePath, RepoId};
use ri_indexer::{PgArchitectureStore, PgGenerationStore};
use sqlx::PgPool;
use uuid::Uuid;

#[tokio::test]
async fn active_architecture_returns_latest_generation_entities()
-> Result<(), Box<dyn std::error::Error>> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };
    let fixture = Fixture::create(&pool).await?;
    let generation = PgGenerationStore::new(pool.clone())
        .begin_generation(
            &fixture.repo_id,
            &fixture.commit_sha,
            "architecture",
            Some("test"),
        )
        .await?;
    let store = PgArchitectureStore::new(pool.clone());
    let indexed = store
        .replace_architecture_entities_for_generation(
            &generation.generation_id,
            &[entity(&fixture, ArchitectureEntityKind::Codeowners)?],
        )
        .await?;
    PgGenerationStore::new(pool.clone())
        .finish_generation(&generation.generation_id)
        .await?;

    let entities = store
        .active_architecture_entities_for_repo(&fixture.repo_id)
        .await?;

    assert_eq!(indexed, 1);
    assert_eq!(entities.len(), 1);
    let entity = entities
        .first()
        .ok_or_else(|| std::io::Error::other("expected one architecture entity"))?;
    assert_eq!(entity.entity_type, "codeowners");
    assert_eq!(entity.file_path, ".github/CODEOWNERS");
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
        for table in [
            "architecture_entities",
            "index_generations",
            "commits",
            "repos",
        ] {
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

fn entity(
    fixture: &Fixture,
    kind: ArchitectureEntityKind,
) -> Result<ArchitectureEntity, ri_core::CoreError> {
    Ok(ArchitectureEntity::new(
        &RepoId::new(&fixture.repo_id)?,
        &CommitSha::new(&fixture.commit_sha)?,
        ArchitectureEntitySpec::new(
            kind,
            "CODEOWNERS",
            FilePath::new(".github/CODEOWNERS")?,
            1,
            1,
            "hash",
        ),
    ))
}

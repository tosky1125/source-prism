#![allow(missing_docs, reason = "Integration test names document behavior.")]

use axum::{
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode},
};
use ri_api::{AppState, app};
use ri_architecture::{ArchitectureEntity, ArchitectureEntityKind, ArchitectureEntitySpec};
use ri_core::{CommitSha, FilePath, RepoId};
use ri_indexer::{PgArchitectureStore, PgGenerationStore};
use serde_json::Value;
use sqlx::PgPool;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use tower::ServiceExt;
use uuid::Uuid;

#[tokio::test]
async fn repo_architecture_returns_indexed_entities_for_repo_id()
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
    PgArchitectureStore::new(pool.clone())
        .replace_architecture_entities_for_generation(
            &generation.generation_id,
            &[entity(&fixture, ArchitectureEntityKind::OpenApi)?],
        )
        .await?;
    PgGenerationStore::new(pool.clone())
        .finish_generation(&generation.generation_id)
        .await?;
    let app = app(AppState::for_test_database(pool.clone())?);
    let request = Request::builder()
        .method(Method::GET)
        .uri(format!("/v1/repos/{}/architecture", fixture.repo_id))
        .body(Body::empty())?;

    let response = app.oneshot(request).await?;

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), 1_000_000).await?;
    let body = serde_json::from_slice::<Value>(&bytes)?;
    assert_eq!(
        body.pointer("/kind").and_then(Value::as_str),
        Some("architecture")
    );
    assert_eq!(
        body.pointer("/entity_count").and_then(Value::as_u64),
        Some(1)
    );
    assert_eq!(
        body.pointer("/entities/0/entity_type")
            .and_then(Value::as_str),
        Some("openapi")
    );
    fixture.cleanup(&pool).await?;
    Ok(())
}

#[tokio::test]
async fn repo_architecture_returns_local_docs_without_database()
-> Result<(), Box<dyn std::error::Error>> {
    let repo = TempRepo::create()?;
    repo.write_file("docs/adr/0001-source-prism.md", "# ADR\n")?;
    repo.commit()?;
    let app = app(AppState::for_test_repo_path(repo.path().to_path_buf())?);
    let request = Request::builder()
        .method(Method::GET)
        .uri("/v1/repos/local/architecture")
        .body(Body::empty())?;

    let response = app.oneshot(request).await?;

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), 1_000_000).await?;
    let body = serde_json::from_slice::<Value>(&bytes)?;
    assert_eq!(
        body.pointer("/kind").and_then(Value::as_str),
        Some("architecture")
    );
    assert_eq!(
        body.pointer("/entity_count").and_then(Value::as_u64),
        Some(1)
    );
    assert_eq!(
        body.pointer("/entities/0/entity_type")
            .and_then(Value::as_str),
        Some("adr")
    );
    assert_eq!(
        body.pointer("/entities/0/file_path")
            .and_then(Value::as_str),
        Some("docs/adr/0001-source-prism.md")
    );
    repo.cleanup()?;
    Ok(())
}

#[derive(Debug)]
struct Fixture {
    repo_id: String,
    commit_sha: String,
}

struct TempRepo {
    path: PathBuf,
}

impl TempRepo {
    fn create() -> Result<Self, Box<dyn std::error::Error>> {
        let path = std::env::temp_dir().join(format!("source-prism-api-arch-{}", Uuid::now_v7()));
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
        let path = self.path.join(path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, body)
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
            "openapi.yaml",
            FilePath::new("openapi.yaml")?,
            1,
            1,
            "hash",
        ),
    ))
}

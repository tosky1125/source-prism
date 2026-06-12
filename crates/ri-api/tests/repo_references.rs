#![allow(missing_docs, reason = "Integration test names document behavior.")]

use axum::{
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode},
};
use ri_api::{AppState, app};
use ri_core::{CommitSha, FilePath, Language, RepoId, SymbolKind};
use ri_indexer::{
    CallEdgeInput, FileManifestInput, PgGenerationStore, PgGraphStore, PgSymbolStore,
};
use ri_symbols::{SymbolRange, SymbolRecord, SymbolSpec};
use serde_json::Value;
use sqlx::PgPool;
use std::time::{SystemTime, UNIX_EPOCH};
use tower::ServiceExt;

#[tokio::test]
async fn repo_references_return_db_call_edges_for_symbol() -> Result<(), Box<dyn std::error::Error>>
{
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };
    let fixture = Fixture::create(&pool).await?;
    let generation = PgGenerationStore::new(pool.clone())
        .begin_generation(
            &fixture.repo_id,
            &fixture.commit_sha,
            "references",
            Some("test"),
        )
        .await?;
    let manifest = FileManifestInput::new("src/lib.rs", "abc123", 120);
    PgGenerationStore::new(pool.clone())
        .replace_file_manifest_generation(&generation.generation_id, &[manifest])
        .await?;
    let caller = fixture.symbol("apply_tax_adds_rate", SymbolKind::TestCase, 8)?;
    let target = fixture.symbol("apply_tax", SymbolKind::Function, 2)?;
    PgSymbolStore::new(pool.clone())
        .replace_symbol_generation(&generation.generation_id, &[caller.clone(), target.clone()])
        .await?;
    let graph_store = PgGraphStore::new(pool.clone());
    graph_store
        .replace_contains_graph(&generation.generation_id, &[caller.clone(), target.clone()])
        .await?;
    graph_store
        .replace_call_graph(
            &generation.generation_id,
            &[CallEdgeInput::new(
                caller.versioned_symbol_id.to_string(),
                target.versioned_symbol_id.to_string(),
                "src/lib.rs".to_owned(),
                SymbolRange::new(9, 16, 9, 25),
                "apply_tax".to_owned(),
            )],
        )
        .await?;
    let app = app(AppState::for_test_database(pool.clone())?);
    let request = Request::builder()
        .method(Method::GET)
        .uri(format!(
            "/v1/repos/{}/references?symbol=apply_tax",
            fixture.repo_id
        ))
        .body(Body::empty())?;

    let response = app.oneshot(request).await?;

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), 1_000_000).await?;
    let body = serde_json::from_slice::<Value>(&bytes)?;
    assert_eq!(
        body.pointer("/kind").and_then(Value::as_str),
        Some("references")
    );
    assert_eq!(
        body.pointer("/references/0/source_fqn")
            .and_then(Value::as_str),
        Some("apply_tax_adds_rate")
    );
    assert_eq!(
        body.pointer("/references/0/target_fqn")
            .and_then(Value::as_str),
        Some("apply_tax")
    );
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
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| duration.as_nanos());
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

    fn symbol(
        &self,
        fqn: &str,
        kind: SymbolKind,
        start_line: u32,
    ) -> Result<SymbolRecord, ri_core::CoreError> {
        let repo = RepoId::new(&self.repo_id)?;
        let commit = CommitSha::new(&self.commit_sha)?;
        Ok(SymbolRecord::new(
            &repo,
            &commit,
            FilePath::new("src/lib.rs")?,
            "abc123",
            SymbolSpec::new(
                Language::Rust,
                kind,
                fqn,
                fqn,
                SymbolRange::new(start_line, 0, start_line.saturating_add(1), 0),
            ),
        ))
    }

    async fn cleanup(&self, pool: &PgPool) -> Result<(), sqlx::Error> {
        for table in [
            "graph_edges",
            "graph_nodes",
            "symbols",
            "file_manifests",
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

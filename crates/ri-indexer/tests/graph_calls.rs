#![allow(
    missing_docs,
    reason = "Integration tests use scenario names instead of API docs."
)]
#![allow(
    clippy::multiple_crate_versions,
    reason = "SQLx TLS dependencies currently pull duplicate platform crates outside this crate's control."
)]

use ri_core::{CommitSha, FilePath, Language, RepoId, SymbolKind};
use ri_indexer::{CallEdgeInput, PgGenerationStore, PgGraphStore, PgSymbolStore};
use ri_symbols::{SymbolRange, SymbolRecord, SymbolSpec};
use sqlx::PgPool;
use uuid::Uuid;

#[tokio::test]
async fn active_graph_includes_static_call_edges() -> Result<(), Box<dyn std::error::Error>> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };
    let fixture = Fixture::create(&pool).await?;
    let generation = PgGenerationStore::new(pool.clone())
        .begin_generation(&fixture.repo_id, &fixture.commit_sha, "graph", Some("test"))
        .await?;
    let source_symbol = symbol(&fixture, "total")?;
    let target_symbol = symbol(&fixture, "apply_tax")?;
    let symbols = vec![source_symbol.clone(), target_symbol.clone()];
    PgSymbolStore::new(pool.clone())
        .replace_symbol_generation(&generation.generation_id, &symbols)
        .await?;
    let store = PgGraphStore::new(pool.clone());
    store
        .replace_contains_graph(&generation.generation_id, &symbols)
        .await?;

    let calls = store
        .replace_call_graph(
            &generation.generation_id,
            &[CallEdgeInput::new(
                source_symbol.versioned_symbol_id.to_string(),
                target_symbol.versioned_symbol_id.to_string(),
                "src/lib.rs".to_owned(),
                SymbolRange::new(5, 20, 5, 32),
                "apply_tax".to_owned(),
            )],
        )
        .await?;
    PgGenerationStore::new(pool.clone())
        .finish_generation(&generation.generation_id)
        .await?;

    let graph = store.active_graph_for_repo(&fixture.repo_id).await?;

    assert_eq!(calls, 1);
    assert!(graph.edges.iter().any(|edge| {
        edge.edge_type == "calls"
            && edge.resolution_method == "tree_sitter_call_name"
            && edge.evidence_file_path.as_deref() == Some("src/lib.rs")
    }));
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
            "graph_edges",
            "graph_nodes",
            "symbols",
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

fn symbol(fixture: &Fixture, fqn: &str) -> Result<SymbolRecord, ri_core::CoreError> {
    Ok(SymbolRecord::new(
        &RepoId::new(&fixture.repo_id)?,
        &CommitSha::new(&fixture.commit_sha)?,
        FilePath::new("src/lib.rs")?,
        "hash",
        SymbolSpec::new(
            Language::Rust,
            SymbolKind::Function,
            fqn,
            fqn,
            SymbolRange::new(1, 0, 8, 1),
        ),
    ))
}

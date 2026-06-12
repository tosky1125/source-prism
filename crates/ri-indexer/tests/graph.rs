#![allow(
    missing_docs,
    reason = "Integration tests use scenario names instead of API docs."
)]
#![allow(
    clippy::multiple_crate_versions,
    reason = "SQLx TLS dependencies currently pull duplicate platform crates outside this crate's control."
)]

use ri_core::{CommitSha, FilePath, Language, RepoId, SymbolKind};
use ri_indexer::{FileManifestInput, PgGenerationStore, PgGraphStore, PgSymbolStore};
use ri_symbols::{SymbolRange, SymbolRecord, SymbolSpec};
use sqlx::PgPool;
use uuid::Uuid;

#[tokio::test]
async fn active_graph_returns_latest_contains_edges() -> Result<(), Box<dyn std::error::Error>> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };
    let fixture = Fixture::create(&pool).await?;
    let generation = PgGenerationStore::new(pool.clone())
        .begin_generation(&fixture.repo_id, &fixture.commit_sha, "graph", Some("test"))
        .await?;
    let store = PgGraphStore::new(pool.clone());
    store
        .replace_contains_graph(
            &generation.generation_id,
            &[symbol(
                &fixture,
                "src/invoice.rs",
                "InvoiceService::apply_tax",
            )?],
        )
        .await?;
    PgGenerationStore::new(pool.clone())
        .finish_generation(&generation.generation_id)
        .await?;

    let graph = store.active_graph_for_repo(&fixture.repo_id).await?;

    assert_eq!(graph.nodes.len(), 2);
    assert_eq!(graph.edges.len(), 1);
    let edge = graph
        .edges
        .first()
        .ok_or_else(|| std::io::Error::other("expected one graph edge"))?;
    assert_eq!(edge.edge_type, "contains");
    assert_eq!(edge.resolution_method, "tree_sitter_contains");
    assert_eq!(edge.evidence_file_path.as_deref(), Some("src/invoice.rs"));
    fixture.cleanup(&pool).await?;
    Ok(())
}

#[tokio::test]
async fn active_graph_includes_static_test_covers_edges() -> Result<(), Box<dyn std::error::Error>>
{
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };
    let fixture = Fixture::create(&pool).await?;
    let generation = PgGenerationStore::new(pool.clone())
        .begin_generation(&fixture.repo_id, &fixture.commit_sha, "graph", Some("test"))
        .await?;
    let target = symbol_with_kind(
        &fixture,
        SymbolKind::Function,
        "src/invoice.rs",
        "apply_tax",
    )?;
    let test = symbol_with_kind(
        &fixture,
        SymbolKind::TestCase,
        "tests/invoice.rs",
        "apply_tax_adds_rate",
    )?;
    let symbols = vec![target, test];
    PgSymbolStore::new(pool.clone())
        .replace_symbol_generation(&generation.generation_id, &symbols)
        .await?;
    let store = PgGraphStore::new(pool.clone());
    store
        .replace_contains_graph(&generation.generation_id, &symbols)
        .await?;
    let test_covers = store
        .replace_test_covers_graph(&generation.generation_id)
        .await?;
    PgGenerationStore::new(pool.clone())
        .finish_generation(&generation.generation_id)
        .await?;

    let graph = store.active_graph_for_repo(&fixture.repo_id).await?;

    assert_eq!(test_covers, 1);
    assert!(graph.edges.iter().any(|edge| {
        edge.edge_type == "test_covers"
            && edge.resolution_method == "static_test_name_match"
            && edge.evidence_file_path.as_deref() == Some("tests/invoice.rs")
    }));
    fixture.cleanup(&pool).await?;
    Ok(())
}

#[tokio::test]
async fn active_graph_includes_rust_module_import_edges() -> Result<(), Box<dyn std::error::Error>>
{
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };
    let fixture = Fixture::create(&pool).await?;
    let generation_store = PgGenerationStore::new(pool.clone());
    let generation = generation_store
        .begin_generation(&fixture.repo_id, &fixture.commit_sha, "graph", Some("test"))
        .await?;
    generation_store
        .replace_file_manifest_generation(
            &generation.generation_id,
            &[manifest("src/lib.rs"), manifest("src/invoice.rs")],
        )
        .await?;
    let symbols = vec![
        symbol_with_kind(&fixture, SymbolKind::Module, "src/lib.rs", "invoice")?,
        symbol_with_kind(
            &fixture,
            SymbolKind::Function,
            "src/invoice.rs",
            "apply_tax",
        )?,
    ];
    PgSymbolStore::new(pool.clone())
        .replace_symbol_generation(&generation.generation_id, &symbols)
        .await?;
    let store = PgGraphStore::new(pool.clone());
    store
        .replace_contains_graph(&generation.generation_id, &symbols)
        .await?;
    let imports = store
        .replace_import_graph(&generation.generation_id)
        .await?;

    let graph = store.active_graph_for_repo(&fixture.repo_id).await?;

    assert_eq!(imports, 1);
    assert!(graph.edges.iter().any(|edge| {
        edge.edge_type == "imports"
            && edge.resolution_method == "rust_mod_file"
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
        sqlx::query("DELETE FROM graph_edges WHERE repo_id = $1")
            .bind(&self.repo_id)
            .execute(pool)
            .await?;
        sqlx::query("DELETE FROM graph_nodes WHERE repo_id = $1")
            .bind(&self.repo_id)
            .execute(pool)
            .await?;
        sqlx::query("DELETE FROM symbols WHERE repo_id = $1")
            .bind(&self.repo_id)
            .execute(pool)
            .await?;
        sqlx::query("DELETE FROM file_manifests WHERE repo_id = $1")
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

fn manifest(path: &str) -> FileManifestInput {
    let mut input = FileManifestInput::new(path, "hash", 10);
    "rust".clone_into(&mut input.language);
    input
}

async fn test_pool() -> Result<Option<PgPool>, sqlx::Error> {
    let Ok(database_url) = std::env::var("DATABASE_URL") else {
        return Ok(None);
    };
    PgPool::connect(database_url.as_str()).await.map(Some)
}

fn symbol(fixture: &Fixture, path: &str, fqn: &str) -> Result<SymbolRecord, ri_core::CoreError> {
    symbol_with_kind(fixture, SymbolKind::Function, path, fqn)
}

fn symbol_with_kind(
    fixture: &Fixture,
    kind: SymbolKind,
    path: &str,
    fqn: &str,
) -> Result<SymbolRecord, ri_core::CoreError> {
    Ok(SymbolRecord::new(
        &RepoId::new(&fixture.repo_id)?,
        &CommitSha::new(&fixture.commit_sha)?,
        FilePath::new(path)?,
        "hash",
        SymbolSpec::new(Language::Rust, kind, fqn, fqn, SymbolRange::new(1, 0, 3, 1)),
    ))
}

use ri_core::{CommitSha, FilePath, Language, RepoId, SymbolKind};
use ri_indexer::{PgGenerationStore, PgGraphStore, PgSearchSyncStore, PgSymbolStore};
use ri_symbols::{SymbolRange, SymbolRecord, SymbolSpec};
use sqlx::PgPool;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug)]
pub struct Fixture {
    pub repo_id: String,
    commit_sha: String,
}

impl Fixture {
    pub async fn create(pool: &PgPool) -> Result<Self, sqlx::Error> {
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

    pub async fn seed_search_symbol(
        &self,
        pool: &PgPool,
        index_kind: &str,
    ) -> Result<SymbolRecord, Box<dyn std::error::Error>> {
        let generation = PgGenerationStore::new(pool.clone())
            .begin_generation(&self.repo_id, &self.commit_sha, index_kind, Some("test"))
            .await?;
        let symbol = self.symbol("InvoiceService::apply_tax")?;
        PgSymbolStore::new(pool.clone())
            .replace_symbol_generation(&generation.generation_id, std::slice::from_ref(&symbol))
            .await?;
        PgGraphStore::new(pool.clone())
            .replace_contains_graph(&generation.generation_id, std::slice::from_ref(&symbol))
            .await?;
        PgSearchSyncStore::new(pool.clone())
            .enqueue_symbol_chunks(
                &self.repo_id,
                &generation.generation_id,
                std::slice::from_ref(&symbol),
                "source-prism-test",
            )
            .await?;
        PgGenerationStore::new(pool.clone())
            .finish_generation(&generation.generation_id)
            .await?;
        Ok(symbol)
    }

    pub async fn cleanup(&self, pool: &PgPool) -> Result<(), sqlx::Error> {
        for table in [
            "search_sync_outbox",
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

    fn symbol(&self, fqn: &str) -> Result<SymbolRecord, ri_core::CoreError> {
        let repo = RepoId::new(&self.repo_id)?;
        let commit = CommitSha::new(&self.commit_sha)?;
        Ok(SymbolRecord::new(
            &repo,
            &commit,
            FilePath::new("src/invoice.rs")?,
            "hash",
            SymbolSpec::new(
                Language::Rust,
                SymbolKind::Function,
                fqn,
                fqn,
                SymbolRange::new(1, 0, 2, 0),
            ),
        ))
    }
}

pub async fn test_pool() -> Result<Option<PgPool>, sqlx::Error> {
    let Ok(database_url) = std::env::var("DATABASE_URL") else {
        return Ok(None);
    };
    PgPool::connect(database_url.as_str()).await.map(Some)
}

pub fn symbol(path: &str, fqn: &str) -> Result<SymbolRecord, ri_core::CoreError> {
    let repo = RepoId::new("repo")?;
    let commit = CommitSha::new("commit")?;
    Ok(SymbolRecord::new(
        &repo,
        &commit,
        FilePath::new(path)?,
        "hash",
        SymbolSpec::new(
            Language::Rust,
            SymbolKind::Function,
            fqn,
            fqn,
            SymbolRange::new(1, 0, 2, 0),
        ),
    ))
}

#![allow(
    clippy::redundant_pub_crate,
    reason = "Binary crate helper modules share crate-visible command handlers."
)]

use std::collections::BTreeMap;
use std::env;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use ri_context::{ResolvedCallReference, build_context_pack_with_calls};
use ri_core::SymbolId;
use ri_impact::ImpactCallEdge;
use ri_indexer::{
    DEFAULT_SEARCH_INDEX, GraphProjection, OpenSearchClient, OpenSearchTextHit, PgGraphStore,
    PgSearchSyncStore, PgSymbolStore,
};
use serde_json::json;
use sqlx::{PgPool, postgres::PgPoolOptions};

use crate::error::CliError;

const DEFAULT_LIMIT: usize = 8;

pub(crate) async fn search_context_command(
    mut args: impl Iterator<Item = String>,
) -> Result<(), CliError> {
    let request = SearchContextArgs::parse(&mut args)?;
    let output = match request.source {
        SearchContextSource::Worktree(repo) => worktree_context(&repo, &request.query)?,
        SearchContextSource::PersistedRepo(repo_id) => {
            persisted_context(repo_id, &request.query).await?
        }
    };
    print_json(&output)
}

fn worktree_context(repo: &Path, query: &str) -> Result<serde_json::Value, CliError> {
    let evidence = ri_context::extract_repo_index(repo)?;
    let calls = context_call_edges(&evidence.calls);
    let pack =
        build_context_pack_with_calls(&evidence.symbols, calls.as_slice(), query, DEFAULT_LIMIT);
    Ok(json!({
        "status": "ok",
        "kind": "search_context",
        "hit_count": pack.hits.len(),
        "impact_count": pack.impacts.len(),
        "search_chunk_count": 0,
        "bm25_hit_count": 0,
        "bm25_hits": [],
        "context_pack": pack,
    }))
}

async fn persisted_context(repo_id: String, query: &str) -> Result<serde_json::Value, CliError> {
    let pool = database_pool().await?;
    let symbols = PgSymbolStore::new(pool.clone())
        .active_symbols_for_repo(&repo_id)
        .await?;
    let graph = PgGraphStore::new(pool.clone())
        .active_graph_for_repo(&repo_id)
        .await?;
    let calls = graph_call_edges(&graph)?;
    let search_chunk_count = PgSearchSyncStore::new(pool)
        .active_symbol_chunk_count_for_repo(&repo_id)
        .await?;
    let bm25_hits = bm25_hits(&repo_id, query).await?;
    let pack = build_context_pack_with_calls(&symbols, calls.as_slice(), query, DEFAULT_LIMIT);
    Ok(json!({
        "status": "ok",
        "kind": "search_context",
        "repo_id": repo_id,
        "hit_count": pack.hits.len(),
        "impact_count": pack.impacts.len(),
        "search_chunk_count": search_chunk_count,
        "bm25_hit_count": bm25_hits.len(),
        "bm25_hits": bm25_hits,
        "context_pack": pack,
    }))
}

async fn bm25_hits(repo_id: &str, query: &str) -> Result<Vec<OpenSearchTextHit>, CliError> {
    let opensearch_url = env::var("OPENSEARCH_URL").map_err(|_| CliError::MissingEnv {
        key: "OPENSEARCH_URL",
    })?;
    OpenSearchClient::new(opensearch_url.as_str())
        .search_text(DEFAULT_SEARCH_INDEX, repo_id, query, DEFAULT_LIMIT)
        .await
        .map_err(ri_indexer::SearchSyncError::from)
        .map_err(Into::into)
}

async fn database_pool() -> Result<PgPool, CliError> {
    let database_url = env::var("DATABASE_URL").map_err(|_| CliError::MissingEnv {
        key: "DATABASE_URL",
    })?;
    PgPoolOptions::new()
        .max_connections(5)
        .connect(database_url.as_str())
        .await
        .map_err(CliError::from)
}

fn graph_call_edges(graph: &GraphProjection) -> Result<Vec<ImpactCallEdge>, CliError> {
    let subject_by_node = graph
        .nodes
        .iter()
        .filter_map(|node| {
            node.subject_id
                .as_ref()
                .map(|subject_id| (node.graph_node_id.as_str(), subject_id.as_str()))
        })
        .collect::<BTreeMap<_, _>>();
    graph
        .edges
        .iter()
        .filter(|edge| edge.edge_type == "calls")
        .filter_map(|edge| {
            let source = subject_by_node.get(edge.source_node_id.as_str())?;
            let target = subject_by_node.get(edge.target_node_id.as_str())?;
            Some(symbol_call_edge(source, target))
        })
        .collect()
}

fn symbol_call_edge(source: &str, target: &str) -> Result<ImpactCallEdge, CliError> {
    Ok(ImpactCallEdge::new(
        SymbolId::new(source)?,
        SymbolId::new(target)?,
    ))
}

fn context_call_edges(calls: &[ResolvedCallReference]) -> Vec<ImpactCallEdge> {
    calls
        .iter()
        .map(|call| {
            ImpactCallEdge::new(call.source_symbol_id.clone(), call.target_symbol_id.clone())
        })
        .collect()
}

fn print_json(value: &serde_json::Value) -> Result<(), CliError> {
    let stdout = io::stdout();
    let mut lock = stdout.lock();
    serde_json::to_writer_pretty(&mut lock, value)?;
    writeln!(lock)?;
    Ok(())
}

#[derive(Debug)]
enum SearchContextSource {
    Worktree(PathBuf),
    PersistedRepo(String),
}

#[derive(Debug)]
struct SearchContextArgs {
    source: SearchContextSource,
    query: String,
}

impl SearchContextArgs {
    fn parse(args: &mut impl Iterator<Item = String>) -> Result<Self, CliError> {
        let mut repo = None::<PathBuf>;
        let mut repo_id = None::<String>;
        let mut query = None::<String>;

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--repo" => {
                    if repo.is_some() || repo_id.is_some() {
                        return Err(CliError::Usage);
                    }
                    let Some(path) = args.next() else {
                        return Err(CliError::Usage);
                    };
                    repo = Some(PathBuf::from(path));
                }
                "--repo-id" => {
                    if repo.is_some() || repo_id.is_some() {
                        return Err(CliError::Usage);
                    }
                    repo_id = Some(args.next().ok_or(CliError::Usage)?);
                }
                _ if query.is_none() => {
                    query = Some(arg);
                }
                _ => return Err(CliError::Usage),
            }
        }

        let source = match (repo, repo_id) {
            (Some(repo), None) => SearchContextSource::Worktree(repo),
            (None, Some(repo_id)) => SearchContextSource::PersistedRepo(repo_id),
            (None, None) => SearchContextSource::Worktree(PathBuf::from(".")),
            (Some(_), Some(_)) => return Err(CliError::Usage),
        };

        Ok(Self {
            source,
            query: query.ok_or(CliError::Usage)?,
        })
    }
}

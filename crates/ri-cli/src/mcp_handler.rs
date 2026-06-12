#![allow(
    clippy::redundant_pub_crate,
    reason = "Binary crate helper modules share crate-visible command handlers."
)]

use std::{collections::BTreeMap, env, path::Path};

use ri_context::ResolvedCallReference;
use ri_core::{FilePath, SymbolId};
use ri_indexer::{GraphEdgeRecord, GraphNodeRecord, GraphProjection, PgGraphStore, PgSymbolStore};
use ri_mcp::RepositoryToolHandler;
use ri_symbols::SymbolRange;
use sqlx::postgres::PgPoolOptions;

use crate::{error::CliError, mcp::McpRepoSource};

pub(crate) async fn handler_for_source(
    source: &McpRepoSource,
) -> Result<RepositoryToolHandler, CliError> {
    match source {
        McpRepoSource::Worktree(repo) => worktree_handler(repo),
        McpRepoSource::PersistedRepo(repo_id) => persisted_handler(repo_id).await,
    }
}

fn worktree_handler(repo: &Path) -> Result<RepositoryToolHandler, CliError> {
    let evidence = ri_context::extract_repo_index(repo)?;
    Ok(RepositoryToolHandler::new(evidence.symbols, evidence.calls))
}

async fn persisted_handler(repo_id: &str) -> Result<RepositoryToolHandler, CliError> {
    let database_url = env::var("DATABASE_URL").map_err(|_| CliError::MissingEnv {
        key: "DATABASE_URL",
    })?;
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(database_url.as_str())
        .await?;
    let symbols = PgSymbolStore::new(pool.clone())
        .active_symbols_for_repo(repo_id)
        .await?;
    let graph = PgGraphStore::new(pool)
        .active_graph_for_repo(repo_id)
        .await?;
    Ok(RepositoryToolHandler::new(
        symbols,
        graph_resolved_calls(&graph),
    ))
}

fn graph_resolved_calls(graph: &GraphProjection) -> Vec<ResolvedCallReference> {
    let node_by_id = graph
        .nodes
        .iter()
        .map(|node| (node.graph_node_id.as_str(), node))
        .collect::<BTreeMap<_, _>>();
    graph
        .edges
        .iter()
        .filter(|edge| edge.edge_type == "calls")
        .filter_map(|edge| graph_resolved_call(&node_by_id, edge))
        .collect()
}

fn graph_resolved_call(
    node_by_id: &BTreeMap<&str, &GraphNodeRecord>,
    edge: &GraphEdgeRecord,
) -> Option<ResolvedCallReference> {
    let source = node_by_id.get(edge.source_node_id.as_str())?;
    let target = node_by_id.get(edge.target_node_id.as_str())?;
    Some(ResolvedCallReference::new(
        SymbolId::new(source.subject_id.as_ref()?).ok()?,
        SymbolId::new(target.subject_id.as_ref()?).ok()?,
        FilePath::new(edge.evidence_file_path.as_ref()?).ok()?,
        target.display_name.clone(),
        edge_range(edge)?,
    ))
}

fn edge_range(edge: &GraphEdgeRecord) -> Option<SymbolRange> {
    Some(SymbolRange::new(
        u32::try_from(edge.evidence_start_line?).ok()?,
        u32::try_from(edge.evidence_start_col?).ok()?,
        u32::try_from(edge.evidence_end_line?).ok()?,
        u32::try_from(edge.evidence_end_col?).ok()?,
    ))
}

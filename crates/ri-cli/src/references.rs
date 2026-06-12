#![allow(
    clippy::redundant_pub_crate,
    reason = "Binary crate helper modules share crate-visible command handlers."
)]

use std::collections::BTreeMap;
use std::env;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use ri_context::{
    ReferenceDirection, ReferenceEndpoints, ReferenceEvidence, ReferenceReport, SymbolReference,
    find_symbol_references, reference_report, symbol_for_query,
};
use ri_core::Confidence;
use ri_indexer::{GraphProjection, PgGraphStore, PgSymbolStore};
use ri_symbols::{SymbolRange, SymbolRecord};
use serde_json::json;
use sqlx::{PgPool, postgres::PgPoolOptions};

use crate::error::CliError;

pub(crate) async fn references_command(
    mut args: impl Iterator<Item = String>,
) -> Result<(), CliError> {
    let request = ReferenceArgs::parse(&mut args)?;
    let report = match request.source {
        ReferenceSource::Worktree(repo) => worktree_references(&repo, &request.symbol)?,
        ReferenceSource::PersistedRepo(repo_id) => {
            let report = persisted_references(&repo_id, &request.symbol).await?;
            return print_report(Some(repo_id.as_str()), &report);
        }
    };
    print_report(None, &report)
}

fn worktree_references(repo: &Path, symbol: &str) -> Result<ReferenceReport, CliError> {
    let evidence = ri_context::extract_repo_index(repo)?;
    Ok(find_symbol_references(
        evidence.symbols.as_slice(),
        evidence.calls.as_slice(),
        symbol.trim(),
    )?)
}

async fn persisted_references(repo_id: &str, symbol: &str) -> Result<ReferenceReport, CliError> {
    let pool = database_pool().await?;
    let symbols = PgSymbolStore::new(pool.clone())
        .active_symbols_for_repo(repo_id)
        .await?;
    let graph = PgGraphStore::new(pool)
        .active_graph_for_repo(repo_id)
        .await?;
    references_from_graph(symbols.as_slice(), &graph, symbol.trim())
}

fn references_from_graph(
    symbols: &[SymbolRecord],
    graph: &GraphProjection,
    query: &str,
) -> Result<ReferenceReport, CliError> {
    let symbol = symbol_for_query(symbols, query)?;
    let node_by_id = graph
        .nodes
        .iter()
        .map(|node| (node.graph_node_id.as_str(), node))
        .collect::<BTreeMap<_, _>>();
    let references = graph
        .edges
        .iter()
        .filter(|edge| edge.edge_type == "calls" || edge.edge_type == "test_covers")
        .filter_map(|edge| {
            let source = node_by_id.get(edge.source_node_id.as_str())?;
            let target = node_by_id.get(edge.target_node_id.as_str())?;
            let source_subject = source.subject_id.as_ref()?;
            let target_subject = target.subject_id.as_ref()?;
            let direction = if target_subject == symbol.versioned_symbol_id.as_str() {
                ReferenceDirection::Incoming
            } else if source_subject == symbol.versioned_symbol_id.as_str() {
                ReferenceDirection::Outgoing
            } else {
                return None;
            };
            Some(SymbolReference::new(
                direction,
                edge.edge_type.clone(),
                ReferenceEndpoints::new(source.display_name.clone(), target.display_name.clone()),
                ReferenceEvidence::new(
                    edge.evidence_file_path.clone().unwrap_or_default(),
                    evidence_range(edge)?,
                    confidence_tier(edge.confidence),
                    edge.resolution_method.clone(),
                ),
            ))
        })
        .collect();
    Ok(reference_report(symbol, references))
}

fn evidence_range(edge: &ri_indexer::GraphEdgeRecord) -> Option<SymbolRange> {
    Some(SymbolRange::new(
        u32::try_from(edge.evidence_start_line?).ok()?,
        u32::try_from(edge.evidence_start_col?).ok()?,
        u32::try_from(edge.evidence_end_line?).ok()?,
        u32::try_from(edge.evidence_end_col?).ok()?,
    ))
}

fn confidence_tier(confidence: f64) -> Confidence {
    if confidence >= 0.95 {
        Confidence::Exact
    } else if confidence >= 0.80 {
        Confidence::High
    } else if confidence >= 0.50 {
        Confidence::Medium
    } else {
        Confidence::Low
    }
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

fn print_report(repo_id: Option<&str>, report: &ReferenceReport) -> Result<(), CliError> {
    print_json(&json!({
        "status": "ok",
        "kind": report.kind,
        "repo_id": repo_id,
        "symbol": report.symbol,
        "incoming_count": report.incoming_count,
        "outgoing_count": report.outgoing_count,
        "references": report.references,
    }))
}

#[derive(Debug)]
enum ReferenceSource {
    Worktree(PathBuf),
    PersistedRepo(String),
}

#[derive(Debug)]
struct ReferenceArgs {
    source: ReferenceSource,
    symbol: String,
}

impl ReferenceArgs {
    fn parse(args: &mut impl Iterator<Item = String>) -> Result<Self, CliError> {
        let mut repo = None::<PathBuf>;
        let mut repo_id = None::<String>;
        let mut symbol = None::<String>;

        while let Some(flag) = args.next() {
            match flag.as_str() {
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
                "--symbol" => {
                    symbol = args.next();
                }
                _ => return Err(CliError::Usage),
            }
        }

        let source = match (repo, repo_id) {
            (Some(repo), None) => ReferenceSource::Worktree(repo),
            (None, Some(repo_id)) => ReferenceSource::PersistedRepo(repo_id),
            (None, None) => ReferenceSource::Worktree(PathBuf::from(".")),
            (Some(_), Some(_)) => return Err(CliError::Usage),
        };

        Ok(Self {
            source,
            symbol: symbol.ok_or(CliError::Usage)?,
        })
    }
}

fn print_json(value: &serde_json::Value) -> Result<(), CliError> {
    let stdout = io::stdout();
    let mut lock = stdout.lock();
    serde_json::to_writer_pretty(&mut lock, value)?;
    writeln!(lock)?;
    Ok(())
}

#![allow(
    clippy::redundant_pub_crate,
    reason = "Binary crate helper modules share crate-visible command handlers."
)]

use std::{
    collections::BTreeMap,
    env,
    io::{self, Write},
    path::{Path, PathBuf},
};

use ri_behavior::{
    CoverageEvidenceSegment, TestCoverageEdge, build_test_context, build_test_context_with_evidence,
};
use ri_core::{Confidence, SymbolId};
use ri_indexer::{
    CoverageSegmentRecord, GraphProjection, PgCoverageStore, PgGraphStore, PgSymbolStore,
};
use serde_json::json;
use sqlx::{PgPool, postgres::PgPoolOptions};

use crate::{error::CliError, symbols::extract_repo_symbols};

pub(crate) async fn test_context_command(
    mut args: impl Iterator<Item = String>,
) -> Result<(), CliError> {
    let request = TestContextArgs::parse(&mut args)?;
    let test_context = match request.source {
        TestContextSource::Worktree(repo) => worktree_test_context(&repo, &request.symbol)?,
        TestContextSource::PersistedRepo(repo_id) => {
            persisted_test_context(&repo_id, &request.symbol).await?
        }
    };
    print_json(&json!({
        "status": "ok",
        "kind": "test_context",
        "test_context": test_context,
    }))
}

fn worktree_test_context(repo: &Path, symbol: &str) -> Result<ri_behavior::TestContext, CliError> {
    let symbols = extract_repo_symbols(repo)?;
    Ok(build_test_context(&symbols, symbol)?)
}

async fn persisted_test_context(
    repo_id: &str,
    symbol: &str,
) -> Result<ri_behavior::TestContext, CliError> {
    let pool = database_pool().await?;
    let symbols = PgSymbolStore::new(pool.clone())
        .active_symbols_for_repo(repo_id)
        .await?;
    let graph = PgGraphStore::new(pool.clone())
        .active_graph_for_repo(repo_id)
        .await?;
    let coverage_edges = graph_test_coverage_edges(&graph)?;
    let coverage_records = PgCoverageStore::new(pool)
        .active_coverage_segments_for_repo(repo_id)
        .await?;
    let coverage_segments = coverage_records
        .iter()
        .filter_map(coverage_segment_evidence)
        .collect::<Vec<_>>();
    Ok(build_test_context_with_evidence(
        symbols.as_slice(),
        coverage_edges.as_slice(),
        coverage_segments.as_slice(),
        symbol,
    )?)
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

fn graph_test_coverage_edges(graph: &GraphProjection) -> Result<Vec<TestCoverageEdge>, CliError> {
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
        .filter(|edge| edge.edge_type == "test_covers")
        .filter_map(|edge| {
            let source = subject_by_node.get(edge.source_node_id.as_str())?;
            let target = subject_by_node.get(edge.target_node_id.as_str())?;
            Some(test_coverage_edge(
                source,
                target,
                edge.confidence,
                edge.resolution_method.as_str(),
            ))
        })
        .collect()
}

fn test_coverage_edge(
    source: &str,
    target: &str,
    confidence: f64,
    resolution_method: &str,
) -> Result<TestCoverageEdge, CliError> {
    Ok(TestCoverageEdge::new(
        SymbolId::new(source)?,
        SymbolId::new(target)?,
        confidence_tier(confidence),
        format!("graph edge: test_covers via {resolution_method}"),
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

fn coverage_segment_evidence(record: &CoverageSegmentRecord) -> Option<CoverageEvidenceSegment> {
    Some(CoverageEvidenceSegment::new(
        record.file_path.clone(),
        u32::try_from(record.start_line).ok()?,
        u32::try_from(record.end_line).ok()?,
        u32::try_from(record.hit_count).ok()?,
        record.format.clone(),
        record.source_path.clone(),
    ))
}

#[derive(Debug)]
enum TestContextSource {
    Worktree(PathBuf),
    PersistedRepo(String),
}

#[derive(Debug)]
struct TestContextArgs {
    source: TestContextSource,
    symbol: String,
}

impl TestContextArgs {
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
            (Some(repo), None) => TestContextSource::Worktree(repo),
            (None, Some(repo_id)) => TestContextSource::PersistedRepo(repo_id),
            (None, None) => TestContextSource::Worktree(PathBuf::from(".")),
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

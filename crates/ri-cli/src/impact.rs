#![allow(
    clippy::redundant_pub_crate,
    reason = "Binary crate helper modules share crate-visible command handlers."
)]

use std::io::{self, Write};
use std::path::{Path, PathBuf};

use ri_context::ResolvedCallReference;
use ri_impact::{ImpactCallEdge, analyze_symbol_impact_with_calls};
use ri_indexer::{PgGraphStore, PgSymbolStore};
use serde_json::json;
use sqlx::postgres::PgPoolOptions;

use crate::{error::CliError, search_context::graph_call_edges};

pub(crate) async fn impact_command(mut args: impl Iterator<Item = String>) -> Result<(), CliError> {
    let request = ImpactArgs::parse(&mut args)?;
    match request.source {
        ImpactSource::Worktree(repo) => worktree_impact(&repo, &request.symbol, None),
        ImpactSource::PersistedRepo(repo_id) => persisted_impact(&repo_id, &request.symbol).await,
    }
}

fn worktree_impact(repo: &Path, symbol: &str, repo_id: Option<&str>) -> Result<(), CliError> {
    let evidence = ri_context::extract_repo_index(repo)?;
    let calls = impact_call_edges(&evidence.calls);
    let report = analyze_symbol_impact_with_calls(evidence.symbols, calls.as_slice(), symbol)?;
    print_report(repo_id, &report)
}

async fn persisted_impact(repo_id: &str, symbol: &str) -> Result<(), CliError> {
    let database_url = std::env::var("DATABASE_URL").map_err(|_| CliError::MissingEnv {
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
    let calls = graph_call_edges(&graph)?;
    let report = analyze_symbol_impact_with_calls(symbols, calls.as_slice(), symbol)?;
    print_report(Some(repo_id), &report)
}

fn print_report(repo_id: Option<&str>, report: &ri_impact::ImpactReport) -> Result<(), CliError> {
    print_json(&json!({
        "status": "ok",
        "kind": "impact",
        "repo_id": repo_id,
        "symbol": report.symbol,
        "affected_files": report.affected_files,
        "direct_callers": report.direct_callers,
        "direct_callees": report.direct_callees,
        "impact_score": report.impact_score,
        "related": report.related,
    }))
}

#[derive(Debug)]
enum ImpactSource {
    Worktree(PathBuf),
    PersistedRepo(String),
}

#[derive(Debug)]
struct ImpactArgs {
    source: ImpactSource,
    symbol: String,
}

impl ImpactArgs {
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
            (Some(repo), None) => ImpactSource::Worktree(repo),
            (None, Some(repo_id)) => ImpactSource::PersistedRepo(repo_id),
            (None, None) => ImpactSource::Worktree(PathBuf::from(".")),
            (Some(_), Some(_)) => return Err(CliError::Usage),
        };

        Ok(Self {
            source,
            symbol: symbol.ok_or(CliError::Usage)?,
        })
    }
}

fn impact_call_edges(calls: &[ResolvedCallReference]) -> Vec<ImpactCallEdge> {
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

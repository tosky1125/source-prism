#![allow(
    clippy::redundant_pub_crate,
    reason = "Binary crate helper modules share crate-visible command handlers."
)]

use std::env;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use ri_context::ResolvedCallReference;
use ri_impact::{ImpactCallEdge, analyze_symbol_impact_with_calls};
use ri_indexer::{PgGraphStore, PgSymbolStore};
use ri_refactor::plan_refactor;
use serde_json::json;
use sqlx::postgres::PgPoolOptions;

use crate::{error::CliError, search_context::graph_call_edges};

pub(crate) async fn plan_command(mut args: impl Iterator<Item = String>) -> Result<(), CliError> {
    let request = RefactorPlanArgs::parse(&mut args)?;
    match request.source {
        RefactorPlanSource::Worktree(repo) => worktree_plan(&repo, &request.symbol),
        RefactorPlanSource::PersistedRepo(repo_id) => {
            persisted_plan(&repo_id, &request.symbol).await
        }
    }
}

fn worktree_plan(repo: &Path, symbol_query: &str) -> Result<(), CliError> {
    let evidence = ri_context::extract_repo_index(repo)?;
    let calls = impact_call_edges(evidence.calls.as_slice());
    let impact =
        analyze_symbol_impact_with_calls(evidence.symbols, calls.as_slice(), symbol_query)?;
    print_plan(None, &impact)
}

async fn persisted_plan(repo_id: &str, symbol_query: &str) -> Result<(), CliError> {
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
    let calls = graph_call_edges(&graph)?;
    let impact = analyze_symbol_impact_with_calls(symbols, calls.as_slice(), symbol_query)?;
    print_plan(Some(repo_id), &impact)
}

fn print_plan(repo_id: Option<&str>, impact: &ri_impact::ImpactReport) -> Result<(), CliError> {
    let plan = plan_refactor(impact);
    print_json(&json!({
        "status": "ok",
        "kind": "refactor_plan",
        "repo_id": repo_id,
        "plan": plan,
    }))
}

#[derive(Debug)]
struct RefactorPlanArgs {
    source: RefactorPlanSource,
    symbol: String,
}

#[derive(Debug)]
enum RefactorPlanSource {
    Worktree(PathBuf),
    PersistedRepo(String),
}

impl RefactorPlanArgs {
    fn parse(args: &mut impl Iterator<Item = String>) -> Result<Self, CliError> {
        let mut source = None::<RefactorPlanSource>;
        let mut symbol = None::<String>;

        while let Some(flag) = args.next() {
            match flag.as_str() {
                "--repo" => {
                    let repo = args.next().ok_or(CliError::Usage)?;
                    set_source(
                        &mut source,
                        RefactorPlanSource::Worktree(PathBuf::from(repo)),
                    )?;
                }
                "--repo-id" => {
                    let repo_id = args.next().ok_or(CliError::Usage)?;
                    set_source(&mut source, RefactorPlanSource::PersistedRepo(repo_id))?;
                }
                "--symbol" => symbol = Some(args.next().ok_or(CliError::Usage)?),
                _ => return Err(CliError::Usage),
            }
        }

        Ok(Self {
            source: source.unwrap_or_else(|| RefactorPlanSource::Worktree(PathBuf::from("."))),
            symbol: symbol.ok_or(CliError::Usage)?,
        })
    }
}

fn set_source(
    current: &mut Option<RefactorPlanSource>,
    next: RefactorPlanSource,
) -> Result<(), CliError> {
    if current.is_some() {
        return Err(CliError::Usage);
    }
    *current = Some(next);
    Ok(())
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

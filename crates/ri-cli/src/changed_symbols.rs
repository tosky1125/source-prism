#![allow(
    clippy::redundant_pub_crate,
    reason = "Binary crate helper modules share crate-visible command handlers."
)]

use std::{
    env, fs,
    io::{self, Write},
    path::PathBuf,
};

use ri_indexer::PgSymbolStore;
use ri_symbols::{SymbolRecord, changed_symbols_for_diff};
use serde_json::json;
use sqlx::postgres::PgPoolOptions;

use crate::{
    error::CliError,
    symbols::{extract_repo_symbols, symbol_json},
};

pub(crate) async fn changed_symbols_command(
    mut args: impl Iterator<Item = String>,
) -> Result<(), CliError> {
    let request = ChangedSymbolsArgs::parse(&mut args)?;
    let diff = fs::read_to_string(&request.diff_path)?;
    let (repo_id, symbols) = match &request.source {
        ChangedSymbolsSource::Worktree(repo) => (None, extract_repo_symbols(repo)?),
        ChangedSymbolsSource::PersistedRepo(repo_id) => {
            let symbols = persisted_symbols(repo_id).await?;
            (Some(repo_id.as_str()), symbols)
        }
    };
    let (changed_lines, changed_symbols) = changed_symbols_for_diff(&symbols, &diff);
    let changed_symbols = changed_symbols
        .iter()
        .map(|changed| {
            json!({
                "file_path": changed.file_path,
                "line": changed.line,
                "symbol": symbol_json(&changed.symbol),
            })
        })
        .collect::<Vec<_>>();

    print_json(&json!({
        "status": "ok",
        "kind": "changed_symbols",
        "repo_id": repo_id,
        "changed_line_count": changed_lines.len(),
        "matched_symbol_count": changed_symbols.len(),
        "changed_symbols": changed_symbols,
    }))
}

#[derive(Debug)]
struct ChangedSymbolsArgs {
    source: ChangedSymbolsSource,
    diff_path: PathBuf,
}

#[derive(Debug)]
enum ChangedSymbolsSource {
    Worktree(PathBuf),
    PersistedRepo(String),
}

impl ChangedSymbolsArgs {
    fn parse(args: &mut impl Iterator<Item = String>) -> Result<Self, CliError> {
        let mut source = None::<ChangedSymbolsSource>;
        let mut diff_path = None::<PathBuf>;

        while let Some(flag) = args.next() {
            match flag.as_str() {
                "--repo" => {
                    let path = args.next().ok_or(CliError::Usage)?;
                    set_source(
                        &mut source,
                        ChangedSymbolsSource::Worktree(PathBuf::from(path)),
                    )?;
                }
                "--repo-id" => {
                    let repo_id = args.next().ok_or(CliError::Usage)?;
                    set_source(&mut source, ChangedSymbolsSource::PersistedRepo(repo_id))?;
                }
                "--diff" => {
                    let path = args.next().ok_or(CliError::Usage)?;
                    diff_path = Some(PathBuf::from(path));
                }
                _ => return Err(CliError::Usage),
            }
        }

        Ok(Self {
            source: source.unwrap_or_else(|| ChangedSymbolsSource::Worktree(PathBuf::from("."))),
            diff_path: diff_path.ok_or(CliError::Usage)?,
        })
    }
}

fn set_source(
    current: &mut Option<ChangedSymbolsSource>,
    next: ChangedSymbolsSource,
) -> Result<(), CliError> {
    if current.is_some() {
        return Err(CliError::Usage);
    }
    *current = Some(next);
    Ok(())
}

async fn persisted_symbols(repo_id: &str) -> Result<Vec<SymbolRecord>, CliError> {
    let database_url = env::var("DATABASE_URL").map_err(|_| CliError::MissingEnv {
        key: "DATABASE_URL",
    })?;
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(database_url.as_str())
        .await?;
    Ok(PgSymbolStore::new(pool)
        .active_symbols_for_repo(repo_id)
        .await?)
}

fn print_json(value: &serde_json::Value) -> Result<(), CliError> {
    let stdout = io::stdout();
    let mut lock = stdout.lock();
    serde_json::to_writer_pretty(&mut lock, value)?;
    writeln!(lock)?;
    Ok(())
}

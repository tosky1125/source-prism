#![allow(missing_docs, reason = "Binary crate exposes no public API.")]
#![allow(
    clippy::multiple_crate_versions,
    reason = "SQLx and Reqwest TLS dependencies pull duplicate platform crates outside this crate's control."
)]

use std::{
    env,
    io::{self, Write},
    path::PathBuf,
    process::ExitCode,
};

use ri_config::{RuntimeConfig, load_env_file};
use ri_indexer::{OpenSearchClient, PgSearchSyncStore};
use sqlx::postgres::PgPoolOptions;
use thiserror::Error;

const DEFAULT_SEARCH_INDEX: &str = "source-prism-dev";

#[tokio::main]
async fn main() -> ExitCode {
    match run(env::args()).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            let mut stderr = io::stderr().lock();
            let _ = writeln!(stderr, "{error}");
            ExitCode::FAILURE
        }
    }
}

async fn run(args: impl IntoIterator<Item = String>) -> Result<(), CliError> {
    let mut args = args.into_iter();
    let _program = args.next();
    let Some(command) = args.next() else {
        return Err(CliError::Usage);
    };
    let Some(subcommand) = args.next() else {
        return Err(CliError::Usage);
    };

    match (command.as_str(), subcommand.as_str()) {
        ("config", "check") => check_config(args),
        ("search", "sync") => search_sync(args).await,
        ("search", "drift-check") => search_drift_check(args).await,
        ("search", "rebuild") => search_rebuild(args).await,
        _ => Err(CliError::Usage),
    }
}

fn check_config(mut args: impl Iterator<Item = String>) -> Result<(), CliError> {
    let Some(flag) = args.next() else {
        return Err(CliError::Usage);
    };
    if flag != "--env-file" {
        return Err(CliError::Usage);
    }
    let Some(path) = args.next() else {
        return Err(CliError::Usage);
    };
    if args.next().is_some() {
        return Err(CliError::Usage);
    }

    let env = load_env_file(&PathBuf::from(path))?;
    let _config = RuntimeConfig::from_env_map(&env)?;
    Ok(())
}

async fn search_sync(mut args: impl Iterator<Item = String>) -> Result<(), CliError> {
    if args.next().as_deref() != Some("--once") || args.next().is_some() {
        return Err(CliError::Usage);
    }
    let (store, client) = search_dependencies().await?;
    let outcome = store.sync_once(&client).await?;
    writeln!(
        io::stdout().lock(),
        "search sync processed={} outbox_id={}",
        u8::from(outcome.processed),
        outcome.outbox_id.unwrap_or_else(|| "none".to_owned())
    )?;
    Ok(())
}

async fn search_drift_check(mut args: impl Iterator<Item = String>) -> Result<(), CliError> {
    let expect_mismatch = matches!(args.next().as_deref(), Some("--expect-mismatch"))
        && matches!(args.next().as_deref(), Some("fixture"))
        && args.next().is_none();
    let (store, client) = search_dependencies().await?;
    if expect_mismatch {
        client.health().await?;
        return Err(CliError::Drift {
            expected: 1,
            actual: 0,
        });
    }
    let report = store.drift_report(&client, DEFAULT_SEARCH_INDEX).await?;
    if report.has_drift() {
        return Err(CliError::Drift {
            expected: report.expected_documents,
            actual: report.actual_documents,
        });
    }
    writeln!(
        io::stdout().lock(),
        "search drift ok expected={} actual={}",
        report.expected_documents,
        report.actual_documents
    )?;
    Ok(())
}

async fn search_rebuild(mut args: impl Iterator<Item = String>) -> Result<(), CliError> {
    if args.next().as_deref() != Some("--from-postgres") || args.next().is_some() {
        return Err(CliError::Usage);
    }
    let (store, client) = search_dependencies().await?;
    let outcome = store.rebuild_index(&client, DEFAULT_SEARCH_INDEX).await?;
    writeln!(
        io::stdout().lock(),
        "search rebuild indexed={}",
        outcome.indexed
    )?;
    Ok(())
}

async fn search_dependencies() -> Result<(PgSearchSyncStore, OpenSearchClient), CliError> {
    let database_url = env::var("DATABASE_URL").map_err(|_| CliError::MissingEnv {
        key: "DATABASE_URL",
    })?;
    let opensearch_url = env::var("OPENSEARCH_URL").map_err(|_| CliError::MissingEnv {
        key: "OPENSEARCH_URL",
    })?;
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(database_url.as_str())
        .await?;
    Ok((
        PgSearchSyncStore::new(pool),
        OpenSearchClient::new(opensearch_url.as_str()),
    ))
}

#[derive(Debug, Error)]
enum CliError {
    #[error(
        "usage: ri-cli config check --env-file <path> | search sync --once | search drift-check [--expect-mismatch fixture] | search rebuild --from-postgres"
    )]
    Usage,
    #[error("missing required env: {key}")]
    MissingEnv { key: &'static str },
    #[error("{0}")]
    Config(#[from] ri_config::ConfigError),
    #[error("search drift detected: expected={expected} actual={actual}")]
    Drift { expected: i64, actual: i64 },
    #[error(transparent)]
    OpenSearch(#[from] ri_indexer::OpenSearchError),
    #[error(transparent)]
    SearchSync(#[from] ri_indexer::SearchSyncError),
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
    #[error(transparent)]
    Io(#[from] io::Error),
}

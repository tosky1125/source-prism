#![allow(
    clippy::redundant_pub_crate,
    reason = "Binary crate helper modules share crate-visible functions across sibling modules."
)]

use std::{env, io, io::Write};

use ri_indexer::{DEFAULT_SEARCH_INDEX, OpenSearchClient, PgSearchSyncStore};
use sqlx::postgres::PgPoolOptions;

use crate::CliError;

pub(crate) async fn command(mut args: impl Iterator<Item = String>) -> Result<(), CliError> {
    let Some(subcommand) = args.next() else {
        return Err(CliError::Usage);
    };
    match subcommand.as_str() {
        "sync" => sync(args).await,
        "drift-check" => drift_check(args).await,
        "rebuild" => rebuild(args).await,
        _ => Err(CliError::Usage),
    }
}

async fn sync(mut args: impl Iterator<Item = String>) -> Result<(), CliError> {
    if args.next().as_deref() != Some("--once") || args.next().is_some() {
        return Err(CliError::Usage);
    }
    let (store, client) = dependencies().await?;
    let outcome = store.sync_once(&client).await?;
    writeln!(
        io::stdout().lock(),
        "search sync processed={} outbox_id={}",
        u8::from(outcome.processed),
        outcome.outbox_id.unwrap_or_else(|| "none".to_owned())
    )?;
    Ok(())
}

async fn drift_check(mut args: impl Iterator<Item = String>) -> Result<(), CliError> {
    let request = DriftCheckArgs::parse(&mut args)?;
    let (store, client) = dependencies().await?;
    if request.expect_mismatch {
        client.health().await?;
        return Err(CliError::Drift {
            expected: 1,
            actual: 0,
        });
    }
    let report = if let Some(generation_id) = request.generation_id.as_deref() {
        store
            .drift_report_for_generation(&client, DEFAULT_SEARCH_INDEX, generation_id)
            .await?
    } else {
        store.drift_report(&client, DEFAULT_SEARCH_INDEX).await?
    };
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

async fn rebuild(mut args: impl Iterator<Item = String>) -> Result<(), CliError> {
    let request = RebuildArgs::parse(&mut args)?;
    let (store, client) = dependencies().await?;
    let outcome = if let Some(generation_id) = request.generation_id.as_deref() {
        store
            .rebuild_index_for_generation(&client, DEFAULT_SEARCH_INDEX, generation_id)
            .await?
    } else {
        store.rebuild_index(&client, DEFAULT_SEARCH_INDEX).await?
    };
    if let Some(generation_id) = request.generation_id {
        writeln!(
            io::stdout().lock(),
            "search rebuild indexed={} generation={generation_id}",
            outcome.indexed
        )?;
    } else {
        writeln!(
            io::stdout().lock(),
            "search rebuild indexed={}",
            outcome.indexed
        )?;
    }
    Ok(())
}

#[derive(Debug, Default)]
struct DriftCheckArgs {
    expect_mismatch: bool,
    generation_id: Option<String>,
}

impl DriftCheckArgs {
    fn parse(args: &mut impl Iterator<Item = String>) -> Result<Self, CliError> {
        let mut request = Self::default();
        while let Some(flag) = args.next() {
            match flag.as_str() {
                "--expect-mismatch" => {
                    if args.next().as_deref() != Some("fixture") || request.generation_id.is_some()
                    {
                        return Err(CliError::Usage);
                    }
                    request.expect_mismatch = true;
                }
                "--generation" => {
                    if request.expect_mismatch || request.generation_id.is_some() {
                        return Err(CliError::Usage);
                    }
                    request.generation_id = Some(args.next().ok_or(CliError::Usage)?);
                }
                _ => return Err(CliError::Usage),
            }
        }
        Ok(request)
    }
}

#[derive(Debug, Default)]
struct RebuildArgs {
    generation_id: Option<String>,
}

impl RebuildArgs {
    fn parse(args: &mut impl Iterator<Item = String>) -> Result<Self, CliError> {
        if args.next().as_deref() != Some("--from-postgres") {
            return Err(CliError::Usage);
        }
        let mut request = Self::default();
        while let Some(flag) = args.next() {
            match flag.as_str() {
                "--generation" if request.generation_id.is_none() => {
                    request.generation_id = Some(args.next().ok_or(CliError::Usage)?);
                }
                _ => return Err(CliError::Usage),
            }
        }
        Ok(request)
    }
}

async fn dependencies() -> Result<(PgSearchSyncStore, OpenSearchClient), CliError> {
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

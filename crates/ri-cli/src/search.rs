#![allow(
    clippy::redundant_pub_crate,
    reason = "Binary crate helper modules share crate-visible functions across sibling modules."
)]

use std::{env, io, io::Write};

use ri_indexer::{OpenSearchClient, PgSearchSyncStore};
use sqlx::postgres::PgPoolOptions;

use crate::CliError;

const DEFAULT_SEARCH_INDEX: &str = "source-prism-dev";

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
    let expect_mismatch = matches!(args.next().as_deref(), Some("--expect-mismatch"))
        && matches!(args.next().as_deref(), Some("fixture"))
        && args.next().is_none();
    let (store, client) = dependencies().await?;
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

async fn rebuild(mut args: impl Iterator<Item = String>) -> Result<(), CliError> {
    if args.next().as_deref() != Some("--from-postgres") || args.next().is_some() {
        return Err(CliError::Usage);
    }
    let (store, client) = dependencies().await?;
    let outcome = store.rebuild_index(&client, DEFAULT_SEARCH_INDEX).await?;
    writeln!(
        io::stdout().lock(),
        "search rebuild indexed={}",
        outcome.indexed
    )?;
    Ok(())
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

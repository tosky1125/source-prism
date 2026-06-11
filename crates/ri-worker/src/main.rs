#![allow(
    missing_docs,
    reason = "Worker binary delegates to the documented library contract."
)]
#![allow(
    clippy::multiple_crate_versions,
    reason = "SQLx TLS dependencies currently pull duplicate platform crates outside this crate's control."
)]

use clap::Parser;
use ri_worker::{JobQueue, JobRuntime, LeaseConfig, PgJobStore, WorkerId};
use sqlx::postgres::PgPoolOptions;
use std::io::Write as _;
use std::time::Duration;

#[derive(Debug, Parser)]
#[command(name = "ri-worker", about = "Source Prism durable job worker")]
struct Cli {
    #[arg(long)]
    once: bool,
    #[arg(long, env = "DATABASE_URL")]
    database_url: String,
    #[arg(long, env = "RI_WORKER_ID", default_value = "ri-worker-local")]
    worker_id: String,
    #[arg(long, default_value = "default")]
    queue: String,
    #[arg(long, default_value_t = 300)]
    lease_seconds: u64,
    #[arg(long, default_value_t = 1_000)]
    poll_interval_ms: u64,
}

#[derive(Debug, thiserror::Error)]
enum CliError {
    #[error(
        "daemon mode is not implemented yet; requested poll interval was {poll_interval_ms} ms"
    )]
    RunModeRequired { poll_interval_ms: u64 },
    #[error(transparent)]
    Job(#[from] ri_worker::JobError),
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[tokio::main]
async fn main() -> Result<(), CliError> {
    let cli = Cli::parse();
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&cli.database_url)
        .await?;
    let store = PgJobStore::new(pool, JobQueue::parse(&cli.queue)?);
    let runtime = JobRuntime::new(
        store,
        WorkerId::parse(&cli.worker_id)?,
        LeaseConfig::new(Duration::from_secs(cli.lease_seconds)),
    );

    if !cli.once {
        return Err(CliError::RunModeRequired {
            poll_interval_ms: cli.poll_interval_ms,
        });
    }
    let outcome = runtime.run_once().await?;
    writeln!(
        std::io::stdout(),
        "ri-worker once processed={} job_id={}",
        u8::from(outcome.processed),
        outcome
            .job_id
            .map_or_else(|| "none".to_owned(), |job_id| job_id.to_string())
    )?;
    Ok(())
}

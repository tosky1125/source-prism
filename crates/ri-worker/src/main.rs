#![allow(
    missing_docs,
    reason = "Worker binary delegates to the documented library contract."
)]
#![allow(
    clippy::multiple_crate_versions,
    reason = "SQLx TLS dependencies currently pull duplicate platform crates outside this crate's control."
)]

use clap::Parser;
use ri_indexer::{OpenSearchClient, PgSearchSyncStore};
use ri_worker::{EnqueueJob, JobKind, JobQueue, JobRuntime, LeaseConfig, PgJobStore, WorkerId};
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
    #[arg(long)]
    max_polls: Option<u64>,
    #[arg(long)]
    enqueue_noop: bool,
    #[arg(long)]
    enqueue_search_sync: bool,
    #[arg(long)]
    search_outbox_id: Option<String>,
    #[arg(long, env = "OPENSEARCH_URL")]
    opensearch_url: Option<String>,
}

#[derive(Debug, thiserror::Error)]
enum CliError {
    #[error(transparent)]
    Job(#[from] ri_worker::JobError),
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
    #[error(transparent)]
    SearchSync(#[from] ri_indexer::SearchSyncError),
    #[error("missing required config: {key}")]
    MissingConfig { key: &'static str },
    #[error("no search sync outbox was processed")]
    NoSearchOutbox,
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
    let store = PgJobStore::new(pool.clone(), JobQueue::parse(&cli.queue)?);
    let runtime = JobRuntime::new(
        store,
        WorkerId::parse(&cli.worker_id)?,
        LeaseConfig::new(Duration::from_secs(cli.lease_seconds)),
    );
    if cli.enqueue_noop {
        enqueue_noop(&runtime, &cli.queue).await?;
    }
    if cli.enqueue_search_sync {
        enqueue_search_sync(&runtime, &cli.queue, cli.search_outbox_id.as_deref()).await?;
    }
    let search_sync = SearchSyncProcessor::new(pool, cli.opensearch_url.as_deref());

    if cli.once {
        return run_once(&runtime, search_sync.as_ref()).await;
    }

    run_daemon(
        &runtime,
        search_sync.as_ref(),
        cli.max_polls,
        Duration::from_millis(cli.poll_interval_ms),
    )
    .await
}

async fn enqueue_noop(runtime: &JobRuntime<PgJobStore>, queue: &str) -> Result<(), CliError> {
    runtime
        .enqueue(EnqueueJob::new(
            JobQueue::parse(queue)?,
            JobKind::parse("noop")?,
            serde_json::json!({ "source": "ri-worker-cli" }),
        ))
        .await?;
    Ok(())
}

async fn enqueue_search_sync(
    runtime: &JobRuntime<PgJobStore>,
    queue: &str,
    outbox_id: Option<&str>,
) -> Result<(), CliError> {
    runtime
        .enqueue(EnqueueJob::new(
            JobQueue::parse(queue)?,
            JobKind::parse("search.sync_once")?,
            search_sync_payload(outbox_id),
        ))
        .await?;
    Ok(())
}

fn search_sync_payload(outbox_id: Option<&str>) -> serde_json::Value {
    let mut payload = serde_json::Map::new();
    payload.insert(
        "source".to_owned(),
        serde_json::Value::String("ri-worker-cli".to_owned()),
    );
    if let Some(outbox_id) = outbox_id {
        payload.insert(
            "outbox_id".to_owned(),
            serde_json::Value::String(outbox_id.to_owned()),
        );
    }
    serde_json::Value::Object(payload)
}

async fn run_once(
    runtime: &JobRuntime<PgJobStore>,
    search_sync: Option<&SearchSyncProcessor>,
) -> Result<(), CliError> {
    let Some(job) = runtime.lease_next().await? else {
        return print_once(false, None);
    };
    let job_id = job.lease.job_id;
    if job.kind.is_noop() {
        runtime.succeed(job.lease).await?;
    } else if job.kind.is_search_sync_once() {
        let Some(search_sync) = search_sync else {
            runtime
                .fail(job.lease, "missing required config: OPENSEARCH_URL")
                .await?;
            return Err(CliError::MissingConfig {
                key: "OPENSEARCH_URL",
            });
        };
        match search_sync.sync_job(&job.payload).await {
            Ok(true) => runtime.succeed(job.lease).await?,
            Ok(false) => {
                runtime
                    .fail(job.lease, "no search sync outbox was processed")
                    .await?;
                return Err(CliError::NoSearchOutbox);
            }
            Err(error) => {
                let error_message = error.to_string();
                runtime.fail(job.lease, &error_message).await?;
                return Err(error.into());
            }
        }
    } else {
        runtime.fail(job.lease, "unsupported job kind").await?;
    }
    print_once(true, Some(job_id))
}

fn print_once(processed: bool, job_id: Option<ri_worker::JobId>) -> Result<(), CliError> {
    writeln!(
        std::io::stdout(),
        "ri-worker once processed={} job_id={}",
        u8::from(processed),
        job_id.map_or_else(|| "none".to_owned(), |job_id| job_id.to_string())
    )?;
    Ok(())
}

async fn run_daemon(
    runtime: &JobRuntime<PgJobStore>,
    search_sync: Option<&SearchSyncProcessor>,
    max_polls: Option<u64>,
    poll_interval: Duration,
) -> Result<(), CliError> {
    let mut polls = 0_u64;
    let mut processed = 0_u64;

    loop {
        if max_polls.is_some_and(|limit| polls >= limit) {
            break;
        }

        let outcome = run_once_for_daemon(runtime, search_sync).await?;
        polls = polls.saturating_add(1);
        processed = processed.saturating_add(u64::from(outcome));
        if !outcome && max_polls.is_none_or(|limit| polls < limit) {
            tokio::time::sleep(poll_interval).await;
        }
    }

    writeln!(
        std::io::stdout(),
        "ri-worker daemon polls={polls} processed={processed}"
    )?;
    Ok(())
}

async fn run_once_for_daemon(
    runtime: &JobRuntime<PgJobStore>,
    search_sync: Option<&SearchSyncProcessor>,
) -> Result<bool, CliError> {
    let Some(job) = runtime.lease_next().await? else {
        return Ok(false);
    };
    if job.kind.is_noop() {
        runtime.succeed(job.lease).await?;
    } else if job.kind.is_search_sync_once() {
        let Some(search_sync) = search_sync else {
            runtime
                .fail(job.lease, "missing required config: OPENSEARCH_URL")
                .await?;
            return Err(CliError::MissingConfig {
                key: "OPENSEARCH_URL",
            });
        };
        match search_sync.sync_job(&job.payload).await {
            Ok(true) => runtime.succeed(job.lease).await?,
            Ok(false) => {
                runtime
                    .fail(job.lease, "no search sync outbox was processed")
                    .await?;
                return Err(CliError::NoSearchOutbox);
            }
            Err(error) => {
                let error_message = error.to_string();
                runtime.fail(job.lease, &error_message).await?;
                return Err(error.into());
            }
        }
    } else {
        runtime.fail(job.lease, "unsupported job kind").await?;
    }
    Ok(true)
}

#[derive(Debug)]
struct SearchSyncProcessor {
    store: PgSearchSyncStore,
    client: OpenSearchClient,
}

impl SearchSyncProcessor {
    fn new(pool: sqlx::PgPool, opensearch_url: Option<&str>) -> Option<Self> {
        let url = opensearch_url?;
        Some(Self {
            store: PgSearchSyncStore::new(pool),
            client: OpenSearchClient::new(url),
        })
    }

    async fn sync_job(
        &self,
        payload: &serde_json::Value,
    ) -> Result<bool, ri_indexer::SearchSyncError> {
        if let Some(outbox_id) = payload.get("outbox_id").and_then(serde_json::Value::as_str) {
            self.store
                .sync_one_by_id(&self.client, outbox_id)
                .await
                .map(|outcome| outcome.processed)
        } else {
            self.store
                .sync_once(&self.client)
                .await
                .map(|outcome| outcome.processed)
        }
    }
}

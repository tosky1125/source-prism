#![allow(
    clippy::redundant_pub_crate,
    reason = "Binary crate helper modules share crate-visible command handlers."
)]

use std::{
    env,
    io::{self, Write},
};

use ri_indexer::{DEFAULT_SEARCH_INDEX, OpenSearchClient, PgSearchSyncStore};
use serde_json::json;
use sqlx::{PgPool, Row as _, postgres::PgPoolOptions};

use crate::CliError;

pub(crate) async fn command(mut args: impl Iterator<Item = String>) -> Result<(), CliError> {
    let Some(flag) = args.next() else {
        return Err(CliError::Usage);
    };
    if flag != "--repo-id" {
        return Err(CliError::Usage);
    }
    let Some(repo_id) = args.next() else {
        return Err(CliError::Usage);
    };
    if args.next().is_some() {
        return Err(CliError::Usage);
    }
    let pool = database_pool().await?;
    let client = opensearch_client()?;
    let report = repo_search_drift_report(&pool, &client, repo_id.as_str()).await?;
    print_json(&report)
}

async fn repo_search_drift_report(
    pool: &PgPool,
    client: &OpenSearchClient,
    repo_id: &str,
) -> Result<serde_json::Value, CliError> {
    let Some(generation_id) = latest_generation_id(pool, repo_id).await? else {
        return Ok(json!({
            "status": "ok",
            "kind": "repo_search_drift",
            "repo_id": repo_id,
            "latest_generation_id": null,
            "expected_documents": 0,
            "actual_documents": 0,
            "has_drift": false,
        }));
    };
    let report = PgSearchSyncStore::new(pool.clone())
        .drift_report_for_repo_generation(client, DEFAULT_SEARCH_INDEX, repo_id, &generation_id)
        .await?;
    Ok(json!({
        "status": "ok",
        "kind": "repo_search_drift",
        "repo_id": repo_id,
        "latest_generation_id": generation_id,
        "expected_documents": report.expected_documents,
        "actual_documents": report.actual_documents,
        "has_drift": report.has_drift(),
    }))
}

async fn latest_generation_id(pool: &PgPool, repo_id: &str) -> Result<Option<String>, CliError> {
    sqlx::query(
        r"
        SELECT generation_id
        FROM index_generations
        WHERE repo_id = $1
        ORDER BY started_at DESC, generation_id DESC
        LIMIT 1
        ",
    )
    .bind(repo_id)
    .fetch_optional(pool)
    .await?
    .map(|row| row.try_get("generation_id"))
    .transpose()
    .map_err(CliError::from)
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

fn opensearch_client() -> Result<OpenSearchClient, CliError> {
    let opensearch_url = env::var("OPENSEARCH_URL").map_err(|_| CliError::MissingEnv {
        key: "OPENSEARCH_URL",
    })?;
    Ok(OpenSearchClient::new(opensearch_url.as_str()))
}

fn print_json(value: &serde_json::Value) -> Result<(), CliError> {
    let stdout = io::stdout();
    let mut lock = stdout.lock();
    serde_json::to_writer_pretty(&mut lock, value)?;
    writeln!(lock)?;
    Ok(())
}

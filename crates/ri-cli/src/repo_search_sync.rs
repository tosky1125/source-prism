#![allow(
    clippy::redundant_pub_crate,
    reason = "Binary crate helper modules share crate-visible command handlers."
)]

use std::{
    env,
    io::{self, Write},
};

use serde_json::json;
use sqlx::{PgPool, Row as _, postgres::PgPoolOptions};

use crate::{CliError, run_outbox::search_sync_outbox_state_counts};

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
    let status = repo_search_sync_status(&pool, repo_id.as_str()).await?;
    print_json(&status)
}

async fn repo_search_sync_status(
    pool: &PgPool,
    repo_id: &str,
) -> Result<serde_json::Value, CliError> {
    let generation = latest_generation(pool, repo_id).await?;
    let Some(generation) = generation else {
        return Ok(json!({
            "status": "ok",
            "kind": "repo_search_sync",
            "repo_id": repo_id,
            "latest_generation_id": null,
            "latest_commit_sha": null,
            "latest_run_status": null,
            "outbox_state_counts": zero_state_counts(),
            "job_state_counts": zero_state_counts(),
        }));
    };
    let generation_id = generation.try_get::<String, _>("generation_id")?;
    Ok(json!({
        "status": "ok",
        "kind": "repo_search_sync",
        "repo_id": repo_id,
        "latest_generation_id": generation_id,
        "latest_commit_sha": generation.try_get::<String, _>("commit_sha")?,
        "latest_run_status": generation.try_get::<String, _>("status")?,
        "outbox_state_counts": search_sync_outbox_state_counts(pool, generation_id.as_str()).await?,
        "job_state_counts": job_state_counts(pool, generation_id.as_str()).await?,
    }))
}

async fn latest_generation(
    pool: &PgPool,
    repo_id: &str,
) -> Result<Option<sqlx::postgres::PgRow>, CliError> {
    sqlx::query(
        r"
        SELECT generation_id, commit_sha, status
        FROM index_generations
        WHERE repo_id = $1
        ORDER BY started_at DESC, generation_id DESC
        LIMIT 1
        ",
    )
    .bind(repo_id)
    .fetch_optional(pool)
    .await
    .map_err(CliError::from)
}

async fn job_state_counts(
    pool: &PgPool,
    generation_id: &str,
) -> Result<serde_json::Value, CliError> {
    let row = sqlx::query(
        r"
        SELECT
            count(*) FILTER (WHERE state = 'queued')::bigint AS queued,
            count(*) FILTER (WHERE state = 'leased')::bigint AS leased,
            count(*) FILTER (WHERE state = 'succeeded')::bigint AS succeeded,
            count(*) FILTER (WHERE state = 'failed')::bigint AS failed,
            count(*) FILTER (WHERE state = 'dead_lettered')::bigint AS dead_lettered,
            count(*) FILTER (WHERE state = 'cancelled')::bigint AS cancelled,
            count(*)::bigint AS total
        FROM jobs
        WHERE generation_id = $1
          AND kind = 'search.sync_once'
        ",
    )
    .bind(generation_id)
    .fetch_one(pool)
    .await?;
    Ok(json!({
        "queued": row.try_get::<i64, _>("queued")?,
        "leased": row.try_get::<i64, _>("leased")?,
        "succeeded": row.try_get::<i64, _>("succeeded")?,
        "failed": row.try_get::<i64, _>("failed")?,
        "dead_lettered": row.try_get::<i64, _>("dead_lettered")?,
        "cancelled": row.try_get::<i64, _>("cancelled")?,
        "total": row.try_get::<i64, _>("total")?,
    }))
}

fn zero_state_counts() -> serde_json::Value {
    json!({
        "queued": 0,
        "leased": 0,
        "succeeded": 0,
        "failed": 0,
        "dead_lettered": 0,
        "cancelled": 0,
        "total": 0,
    })
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

fn print_json(value: &serde_json::Value) -> Result<(), CliError> {
    let stdout = io::stdout();
    let mut lock = stdout.lock();
    serde_json::to_writer_pretty(&mut lock, value)?;
    writeln!(lock)?;
    Ok(())
}

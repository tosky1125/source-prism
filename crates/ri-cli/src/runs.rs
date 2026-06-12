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
    let runs = repo_runs(&pool, repo_id.as_str()).await?;
    print_json(&json!({
        "status": "ok",
        "kind": "repo_runs",
        "repo_id": repo_id,
        "run_count": runs.len(),
        "runs": runs,
    }))
}

pub(crate) async fn run_command(mut args: impl Iterator<Item = String>) -> Result<(), CliError> {
    let Some(flag) = args.next() else {
        return Err(CliError::Usage);
    };
    if flag != "--run-id" {
        return Err(CliError::Usage);
    }
    let Some(run_id) = args.next() else {
        return Err(CliError::Usage);
    };
    if args.next().is_some() {
        return Err(CliError::Usage);
    }
    let pool = database_pool().await?;
    let run = run_by_id(&pool, run_id.as_str()).await?;
    print_json(&json!({
        "status": "ok",
        "kind": "run",
        "run": run,
    }))
}

async fn repo_runs(pool: &PgPool, repo_id: &str) -> Result<Vec<serde_json::Value>, CliError> {
    let rows = sqlx::query(
        r"
        SELECT
            g.generation_id,
            g.commit_sha,
            g.index_kind,
            g.status,
            g.started_at::text AS started_at,
            g.finished_at::text AS finished_at,
            (
                SELECT count(*)::bigint FROM file_manifests AS item
                WHERE item.generation_id = g.generation_id
            ) AS file_manifest_count,
            (
                SELECT count(*)::bigint FROM symbols AS item
                WHERE item.generation_id = g.generation_id
            ) AS symbol_count,
            (
                SELECT count(*)::bigint FROM graph_edges AS item
                WHERE item.generation_id = g.generation_id
            ) AS graph_edge_count,
            (
                SELECT count(*)::bigint FROM search_sync_outbox AS item
                WHERE item.generation_id = g.generation_id
            ) AS search_chunk_count,
            (
                SELECT count(*)::bigint FROM jobs AS item
                WHERE item.generation_id = g.generation_id
                  AND item.kind = 'search.sync_once'
            ) AS search_sync_job_count,
            (
                SELECT count(*)::bigint FROM test_cases AS item
                WHERE item.generation_id = g.generation_id
            ) AS test_case_count
        FROM index_generations AS g
        WHERE g.repo_id = $1
        ORDER BY g.started_at DESC
        LIMIT 20
        ",
    )
    .bind(repo_id)
    .fetch_all(pool)
    .await?;

    let mut runs = Vec::with_capacity(rows.len());
    for row in rows {
        let run_id = row.try_get::<String, _>("generation_id")?;
        let search_sync_job_details = search_sync_jobs(pool, run_id.as_str()).await?;
        runs.push(json!({
            "run_id": run_id,
            "commit_sha": row.try_get::<String, _>("commit_sha")?,
            "index_kind": row.try_get::<String, _>("index_kind")?,
            "status": row.try_get::<String, _>("status")?,
            "started_at": row.try_get::<String, _>("started_at")?,
            "finished_at": row.try_get::<Option<String>, _>("finished_at")?,
            "evidence": {
                "file_manifests": row.try_get::<i64, _>("file_manifest_count")?,
                "symbols": row.try_get::<i64, _>("symbol_count")?,
                "graph_edges": row.try_get::<i64, _>("graph_edge_count")?,
                "search_chunks": row.try_get::<i64, _>("search_chunk_count")?,
                "search_sync_jobs": row.try_get::<i64, _>("search_sync_job_count")?,
                "search_sync_job_details": search_sync_job_details,
                "test_cases": row.try_get::<i64, _>("test_case_count")?,
            }
        }));
    }
    Ok(runs)
}

async fn run_by_id(pool: &PgPool, run_id: &str) -> Result<serde_json::Value, CliError> {
    let row = sqlx::query(
        r"
        SELECT
            g.generation_id,
            g.repo_id,
            g.commit_sha,
            g.index_kind,
            g.status,
            g.started_at::text AS started_at,
            g.finished_at::text AS finished_at,
            (
                SELECT count(*)::bigint FROM file_manifests AS item
                WHERE item.generation_id = g.generation_id
            ) AS file_manifest_count,
            (
                SELECT count(*)::bigint FROM symbols AS item
                WHERE item.generation_id = g.generation_id
            ) AS symbol_count,
            (
                SELECT count(*)::bigint FROM graph_edges AS item
                WHERE item.generation_id = g.generation_id
            ) AS graph_edge_count,
            (
                SELECT count(*)::bigint FROM search_sync_outbox AS item
                WHERE item.generation_id = g.generation_id
            ) AS search_chunk_count,
            (
                SELECT count(*)::bigint FROM jobs AS item
                WHERE item.generation_id = g.generation_id
                  AND item.kind = 'search.sync_once'
            ) AS search_sync_job_count,
            (
                SELECT count(*)::bigint FROM test_cases AS item
                WHERE item.generation_id = g.generation_id
            ) AS test_case_count
        FROM index_generations AS g
        WHERE g.generation_id = $1
        ",
    )
    .bind(run_id)
    .fetch_one(pool)
    .await?;
    let search_sync_job_details = search_sync_jobs(pool, run_id).await?;
    Ok(json!({
        "run_id": row.try_get::<String, _>("generation_id")?,
        "repo_id": row.try_get::<String, _>("repo_id")?,
        "commit_sha": row.try_get::<String, _>("commit_sha")?,
        "index_kind": row.try_get::<String, _>("index_kind")?,
        "status": row.try_get::<String, _>("status")?,
        "started_at": row.try_get::<String, _>("started_at")?,
        "finished_at": row.try_get::<Option<String>, _>("finished_at")?,
        "evidence": {
            "file_manifests": row.try_get::<i64, _>("file_manifest_count")?,
            "symbols": row.try_get::<i64, _>("symbol_count")?,
            "graph_edges": row.try_get::<i64, _>("graph_edge_count")?,
            "search_chunks": row.try_get::<i64, _>("search_chunk_count")?,
            "search_sync_jobs": row.try_get::<i64, _>("search_sync_job_count")?,
            "search_sync_job_details": search_sync_job_details,
            "test_cases": row.try_get::<i64, _>("test_case_count")?,
        }
    }))
}

async fn search_sync_jobs(
    pool: &PgPool,
    generation_id: &str,
) -> Result<Vec<serde_json::Value>, CliError> {
    let rows = sqlx::query(
        r"
        SELECT job_id, state, attempt_count
        FROM jobs
        WHERE generation_id = $1
          AND kind = 'search.sync_once'
        ORDER BY created_at ASC
        ",
    )
    .bind(generation_id)
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|row| {
            Ok(json!({
                "job_id": row.try_get::<String, _>("job_id")?,
                "state": row.try_get::<String, _>("state")?,
                "attempt_count": row.try_get::<i32, _>("attempt_count")?,
            }))
        })
        .collect()
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

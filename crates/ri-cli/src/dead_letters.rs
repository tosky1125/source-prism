#![allow(
    clippy::redundant_pub_crate,
    reason = "Binary crate helper modules share crate-visible command handlers."
)]

use std::{
    env,
    io::{self, Write},
};

use ri_worker::{JobQueue, PgJobStore};
use serde_json::json;
use sqlx::{PgPool, postgres::PgPoolOptions};

use crate::CliError;

const DEAD_LETTER_LIMIT: i64 = 50;

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
    let dead_letters = PgJobStore::new(pool, JobQueue::default())
        .dead_letters_for_repo(repo_id.as_str(), DEAD_LETTER_LIMIT)
        .await?;
    print_json(&json!({
        "status": "ok",
        "kind": "repo_dead_letters",
        "repo_id": repo_id,
        "dead_letter_count": dead_letters.len(),
        "dead_letters": dead_letters,
    }))
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

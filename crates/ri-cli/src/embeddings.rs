#![allow(
    clippy::redundant_pub_crate,
    reason = "Binary crate helper modules share crate-visible command handlers."
)]

use std::{env, io, io::Write};

use ri_embedding::{EmbeddingCacheInput, EmbeddingVector, PgEmbeddingCache};
use serde_json::json;
use sqlx::postgres::PgPoolOptions;

use crate::CliError;

pub(crate) async fn command(mut args: impl Iterator<Item = String>) -> Result<(), CliError> {
    let Some(subcommand) = args.next() else {
        return Err(CliError::Usage);
    };
    match subcommand.as_str() {
        "cache-put" => cache_put(args).await,
        _ => Err(CliError::Usage),
    }
}

async fn cache_put(args: impl Iterator<Item = String>) -> Result<(), CliError> {
    let args = CachePutArgs::parse(args)?;
    let input = EmbeddingCacheInput::parse(
        args.provider.as_str(),
        args.model.as_str(),
        args.input_kind.as_str(),
        args.input.as_str(),
        args.dimensions,
    )?;
    let vector = EmbeddingVector::parse_csv(args.vector.as_str())?;
    let cache = PgEmbeddingCache::new(pg_pool().await?);
    let write = cache.store_or_touch(&input, &vector).await?;
    let entry = write.entry;
    print_json(&json!({
        "kind": "embedding_cache",
        "status": "ok",
        "cache_hit": write.cache_hit,
        "cache_key": entry.cache_key,
        "provider": entry.provider,
        "model": entry.model,
        "input_sha256": entry.input_sha256,
        "input_kind": entry.input_kind,
        "dimensions": entry.dimensions,
    }))
}

#[derive(Debug)]
struct CachePutArgs {
    provider: String,
    model: String,
    input_kind: String,
    dimensions: i32,
    input: String,
    vector: String,
}

impl CachePutArgs {
    fn parse(mut args: impl Iterator<Item = String>) -> Result<Self, CliError> {
        let provider = parse_value(&mut args, "--provider")?;
        let model = parse_value(&mut args, "--model")?;
        let input_kind = parse_value(&mut args, "--kind")?;
        let dimensions = parse_value(&mut args, "--dimensions")?
            .parse::<i32>()
            .map_err(|_| CliError::Usage)?;
        let input = parse_value(&mut args, "--input")?;
        let vector = parse_value(&mut args, "--vector")?;
        if args.next().is_some() {
            return Err(CliError::Usage);
        }
        Ok(Self {
            provider,
            model,
            input_kind,
            dimensions,
            input,
            vector,
        })
    }
}

fn parse_value(args: &mut impl Iterator<Item = String>, flag: &str) -> Result<String, CliError> {
    if args.next().as_deref() != Some(flag) {
        return Err(CliError::Usage);
    }
    args.next().ok_or(CliError::Usage)
}

async fn pg_pool() -> Result<sqlx::PgPool, CliError> {
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

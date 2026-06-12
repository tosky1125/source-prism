#![allow(
    clippy::redundant_pub_crate,
    reason = "Binary crate helper modules share crate-visible command handlers."
)]

use std::{
    env, fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

use ri_behavior::{parse_junit_xml, parse_lcov};
use ri_core::CommitSha;
use ri_git::{discover_worktree, resolve_commit_sha};
use ri_indexer::{PgCoverageStore, PgGenerationStore, PgTestRunStore};
use serde_json::json;
use sqlx::{PgPool, postgres::PgPoolOptions};

use crate::{CliError, index::repo_id_for_worktree};

pub(crate) async fn command(mut args: impl Iterator<Item = String>) -> Result<(), CliError> {
    let Some(subcommand) = args.next() else {
        return Err(CliError::Usage);
    };
    match subcommand.as_str() {
        "import-junit" => import_junit(args).await,
        "import-lcov" => import_lcov(args).await,
        _ => Err(CliError::Usage),
    }
}

async fn import_junit(mut args: impl Iterator<Item = String>) -> Result<(), CliError> {
    let parsed = ImportJunitArgs::parse(&mut args)?;
    let pool = database_pool().await?;
    let worktree = discover_worktree(&parsed.repo_path)?;
    let repo_id = repo_id_for_worktree(&worktree)?;
    let commit_sha = resolve_commit_sha(&parsed.repo_path, &parsed.sha)?;
    let _commit = CommitSha::new(&commit_sha)?;
    upsert_repo_commit(&pool, &repo_id, &worktree, &commit_sha).await?;

    let junit = fs::read_to_string(&parsed.junit_path)?;
    let report = parse_junit_xml(&junit)?;
    let generation_store = PgGenerationStore::new(pool.clone());
    let generation = generation_store
        .begin_generation(
            &repo_id,
            &commit_sha,
            "test_results",
            Some("ri-cli-junit-v1"),
        )
        .await?;
    let outcome = PgTestRunStore::new(pool)
        .replace_junit_run_for_generation(
            &generation.generation_id,
            parsed.junit_path.to_string_lossy().as_ref(),
            &report,
        )
        .await?;
    generation_store
        .finish_generation(&generation.generation_id)
        .await?;
    print_json(&json!({
        "status": "ok",
        "kind": "test_runs",
        "repo_id": repo_id,
        "commit_sha": commit_sha,
        "generation_id": generation.generation_id,
        "test_run_id": outcome.test_run_id,
        "imported_results": outcome.result_count,
    }))
}

async fn import_lcov(mut args: impl Iterator<Item = String>) -> Result<(), CliError> {
    let parsed = ImportCoverageArgs::parse(&mut args)?;
    let pool = database_pool().await?;
    let worktree = discover_worktree(&parsed.repo_path)?;
    let repo_id = repo_id_for_worktree(&worktree)?;
    let commit_sha = resolve_commit_sha(&parsed.repo_path, &parsed.sha)?;
    let _commit = CommitSha::new(&commit_sha)?;
    upsert_repo_commit(&pool, &repo_id, &worktree, &commit_sha).await?;

    let lcov = fs::read_to_string(&parsed.lcov_path)?;
    let report = parse_lcov(&lcov)?;
    let generation_store = PgGenerationStore::new(pool.clone());
    let generation = generation_store
        .begin_generation(&repo_id, &commit_sha, "coverage", Some("ri-cli-lcov-v1"))
        .await?;
    let outcome = PgCoverageStore::new(pool)
        .replace_lcov_for_generation(
            &generation.generation_id,
            parsed.lcov_path.to_string_lossy().as_ref(),
            &report,
        )
        .await?;
    generation_store
        .finish_generation(&generation.generation_id)
        .await?;
    print_json(&json!({
        "status": "ok",
        "kind": "coverage",
        "repo_id": repo_id,
        "commit_sha": commit_sha,
        "generation_id": generation.generation_id,
        "imported_segments": outcome.segment_count,
    }))
}

#[derive(Debug)]
struct ImportJunitArgs {
    repo_path: PathBuf,
    sha: String,
    junit_path: PathBuf,
}

#[derive(Debug)]
struct ImportCoverageArgs {
    repo_path: PathBuf,
    sha: String,
    lcov_path: PathBuf,
}

impl ImportJunitArgs {
    fn parse(args: &mut impl Iterator<Item = String>) -> Result<Self, CliError> {
        let mut repo_path = None;
        let mut sha = None;
        let mut junit_path = None;
        while let Some(flag) = args.next() {
            match flag.as_str() {
                "--repo" => repo_path = args.next().map(PathBuf::from),
                "--sha" => sha = args.next(),
                "--junit" => junit_path = args.next().map(PathBuf::from),
                _ => return Err(CliError::Usage),
            }
        }
        Ok(Self {
            repo_path: repo_path.ok_or(CliError::Usage)?,
            sha: sha.ok_or(CliError::Usage)?,
            junit_path: junit_path.ok_or(CliError::Usage)?,
        })
    }
}

impl ImportCoverageArgs {
    fn parse(args: &mut impl Iterator<Item = String>) -> Result<Self, CliError> {
        let mut repo_path = None;
        let mut sha = None;
        let mut lcov_path = None;
        while let Some(flag) = args.next() {
            match flag.as_str() {
                "--repo" => repo_path = args.next().map(PathBuf::from),
                "--sha" => sha = args.next(),
                "--lcov" => lcov_path = args.next().map(PathBuf::from),
                _ => return Err(CliError::Usage),
            }
        }
        Ok(Self {
            repo_path: repo_path.ok_or(CliError::Usage)?,
            sha: sha.ok_or(CliError::Usage)?,
            lcov_path: lcov_path.ok_or(CliError::Usage)?,
        })
    }
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

async fn upsert_repo_commit(
    pool: &PgPool,
    repo_id: &str,
    worktree: &Path,
    commit_sha: &str,
) -> Result<(), CliError> {
    let repo_name = worktree
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("local-repo");
    sqlx::query(
        r"
        INSERT INTO repos (repo_id, name)
        VALUES ($1, $2)
        ON CONFLICT (repo_id) DO UPDATE SET updated_at = now()
        ",
    )
    .bind(repo_id)
    .bind(repo_name)
    .execute(pool)
    .await?;
    sqlx::query(
        r"
        INSERT INTO commits (repo_id, commit_sha)
        VALUES ($1, $2)
        ON CONFLICT (repo_id, commit_sha) DO NOTHING
        ",
    )
    .bind(repo_id)
    .bind(commit_sha)
    .execute(pool)
    .await?;
    Ok(())
}

fn print_json(value: &serde_json::Value) -> Result<(), CliError> {
    let stdout = io::stdout();
    let mut lock = stdout.lock();
    serde_json::to_writer_pretty(&mut lock, value)?;
    writeln!(lock)?;
    Ok(())
}

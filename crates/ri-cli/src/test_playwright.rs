#![allow(
    clippy::redundant_pub_crate,
    reason = "Binary crate helper modules share crate-visible command handlers."
)]

use std::{fs, path::PathBuf};

use ri_behavior::parse_playwright_json;
use ri_core::CommitSha;
use ri_git::{discover_worktree, resolve_commit_sha};
use ri_indexer::{PgGenerationStore, PgTestRunStore};
use serde_json::json;
use sqlx::PgPool;

use crate::{CliError, index::repo_id_for_worktree, tests::print_json};

pub(crate) async fn import(
    mut args: impl Iterator<Item = String>,
    pool: PgPool,
) -> Result<(), CliError> {
    let parsed = ImportPlaywrightArgs::parse(&mut args)?;
    let worktree = discover_worktree(&parsed.repo_path)?;
    let repo_id = repo_id_for_worktree(&worktree)?;
    let commit_sha = resolve_commit_sha(&parsed.repo_path, &parsed.sha)?;
    let _commit = CommitSha::new(&commit_sha)?;
    crate::test_junit::upsert_repo_commit(&pool, &repo_id, &worktree, &commit_sha).await?;

    let body = fs::read_to_string(&parsed.playwright_path)?;
    let report = parse_playwright_json(&body)?;
    let generation_store = PgGenerationStore::new(pool.clone());
    let generation = generation_store
        .begin_generation(
            &repo_id,
            &commit_sha,
            "test_results",
            Some("ri-cli-playwright-json-v1"),
        )
        .await?;
    let outcome = PgTestRunStore::new(pool)
        .replace_playwright_run_for_generation(
            &generation.generation_id,
            parsed.playwright_path.to_string_lossy().as_ref(),
            &report,
        )
        .await?;
    generation_store
        .finish_generation(&generation.generation_id)
        .await?;
    print_json(&json!({
        "status": "ok",
        "kind": "test_runs",
        "framework": "playwright",
        "repo_id": repo_id,
        "commit_sha": commit_sha,
        "generation_id": generation.generation_id,
        "test_run_id": outcome.test_run_id,
        "imported_results": outcome.result_count,
    }))
}

#[derive(Debug)]
struct ImportPlaywrightArgs {
    repo_path: PathBuf,
    sha: String,
    playwright_path: PathBuf,
}

impl ImportPlaywrightArgs {
    fn parse(args: &mut impl Iterator<Item = String>) -> Result<Self, CliError> {
        let mut repo_path = None;
        let mut sha = None;
        let mut playwright_path = None;
        while let Some(flag) = args.next() {
            match flag.as_str() {
                "--repo" => repo_path = args.next().map(PathBuf::from),
                "--sha" => sha = args.next(),
                "--playwright-json" => playwright_path = args.next().map(PathBuf::from),
                _ => return Err(CliError::Usage),
            }
        }
        Ok(Self {
            repo_path: repo_path.ok_or(CliError::Usage)?,
            sha: sha.ok_or(CliError::Usage)?,
            playwright_path: playwright_path.ok_or(CliError::Usage)?,
        })
    }
}

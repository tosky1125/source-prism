#![allow(
    clippy::redundant_pub_crate,
    reason = "Binary crate helper modules share crate-visible command handlers."
)]

use std::{
    env, fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

use ri_behavior::{CoverageReport, parse_cobertura_xml, parse_jacoco_xml, parse_lcov};
use ri_core::CommitSha;
use ri_git::{discover_worktree, resolve_commit_sha};
use ri_indexer::{PgCoverageStore, PgGenerationStore};
use serde_json::json;
use sqlx::{PgPool, postgres::PgPoolOptions};

use crate::{CliError, index::repo_id_for_worktree};

pub(crate) async fn command(mut args: impl Iterator<Item = String>) -> Result<(), CliError> {
    let Some(subcommand) = args.next() else {
        return Err(CliError::Usage);
    };
    match subcommand.as_str() {
        "import-junit" => crate::test_junit::import(args, database_pool().await?).await,
        "import-pytest-json" => crate::test_pytest::import(args, database_pool().await?).await,
        "import-playwright-json" => {
            crate::test_playwright::import(args, database_pool().await?).await
        }
        "import-go-test-json" => crate::test_go::import(args, database_pool().await?).await,
        "import-lcov" => import_lcov(args).await,
        "import-cobertura" => import_cobertura(args).await,
        "import-jacoco" => import_jacoco(args).await,
        _ => Err(CliError::Usage),
    }
}

async fn import_lcov(args: impl Iterator<Item = String>) -> Result<(), CliError> {
    import_coverage(args, CoverageImport::Lcov).await
}

async fn import_cobertura(args: impl Iterator<Item = String>) -> Result<(), CliError> {
    import_coverage(args, CoverageImport::Cobertura).await
}

async fn import_jacoco(args: impl Iterator<Item = String>) -> Result<(), CliError> {
    import_coverage(args, CoverageImport::Jacoco).await
}

async fn import_coverage(
    mut args: impl Iterator<Item = String>,
    import: CoverageImport,
) -> Result<(), CliError> {
    let parsed = ImportCoverageArgs::parse(&mut args, import.path_flag())?;
    let pool = database_pool().await?;
    let worktree = discover_worktree(&parsed.repo_path)?;
    let repo_id = repo_id_for_worktree(&worktree)?;
    let commit_sha = resolve_commit_sha(&parsed.repo_path, &parsed.sha)?;
    let _commit = CommitSha::new(&commit_sha)?;
    upsert_repo_commit(&pool, &repo_id, &worktree, &commit_sha).await?;

    let coverage = fs::read_to_string(&parsed.coverage_path)?;
    let report = import.parse_report(&coverage)?;
    let generation_store = PgGenerationStore::new(pool.clone());
    let generation = generation_store
        .begin_generation(&repo_id, &commit_sha, "coverage", Some(import.extractor()))
        .await?;
    let store = PgCoverageStore::new(pool);
    let outcome = match import {
        CoverageImport::Lcov => {
            store
                .replace_lcov_for_generation(
                    &generation.generation_id,
                    parsed.coverage_path.to_string_lossy().as_ref(),
                    &report,
                )
                .await?
        }
        CoverageImport::Cobertura => {
            store
                .replace_cobertura_for_generation(
                    &generation.generation_id,
                    parsed.coverage_path.to_string_lossy().as_ref(),
                    &report,
                )
                .await?
        }
        CoverageImport::Jacoco => {
            store
                .replace_jacoco_for_generation(
                    &generation.generation_id,
                    parsed.coverage_path.to_string_lossy().as_ref(),
                    &report,
                )
                .await?
        }
    };
    generation_store
        .finish_generation(&generation.generation_id)
        .await?;
    print_json(&json!({
        "status": "ok",
        "kind": "coverage",
        "format": import.format(),
        "repo_id": repo_id,
        "commit_sha": commit_sha,
        "generation_id": generation.generation_id,
        "imported_segments": outcome.segment_count,
    }))
}

#[derive(Clone, Copy, Debug)]
enum CoverageImport {
    Lcov,
    Cobertura,
    Jacoco,
}

impl CoverageImport {
    const fn path_flag(self) -> &'static str {
        match self {
            Self::Lcov => "--lcov",
            Self::Cobertura => "--cobertura",
            Self::Jacoco => "--jacoco",
        }
    }

    const fn format(self) -> &'static str {
        match self {
            Self::Lcov => "lcov",
            Self::Cobertura => "cobertura",
            Self::Jacoco => "jacoco",
        }
    }

    const fn extractor(self) -> &'static str {
        match self {
            Self::Lcov => "ri-cli-lcov-v1",
            Self::Cobertura => "ri-cli-cobertura-v1",
            Self::Jacoco => "ri-cli-jacoco-v1",
        }
    }

    fn parse_report(self, body: &str) -> Result<CoverageReport, CliError> {
        Ok(match self {
            Self::Lcov => parse_lcov(body)?,
            Self::Cobertura => parse_cobertura_xml(body)?,
            Self::Jacoco => parse_jacoco_xml(body)?,
        })
    }
}

#[derive(Debug)]
struct ImportCoverageArgs {
    repo_path: PathBuf,
    sha: String,
    coverage_path: PathBuf,
}

impl ImportCoverageArgs {
    fn parse(args: &mut impl Iterator<Item = String>, path_flag: &str) -> Result<Self, CliError> {
        let mut repo_path = None;
        let mut sha = None;
        let mut coverage_path = None;
        while let Some(flag) = args.next() {
            match flag.as_str() {
                "--repo" => repo_path = args.next().map(PathBuf::from),
                "--sha" => sha = args.next(),
                value if value == path_flag => coverage_path = args.next().map(PathBuf::from),
                _ => return Err(CliError::Usage),
            }
        }
        Ok(Self {
            repo_path: repo_path.ok_or(CliError::Usage)?,
            sha: sha.ok_or(CliError::Usage)?,
            coverage_path: coverage_path.ok_or(CliError::Usage)?,
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

pub(crate) fn print_json(value: &serde_json::Value) -> Result<(), CliError> {
    let stdout = io::stdout();
    let mut lock = stdout.lock();
    serde_json::to_writer_pretty(&mut lock, value)?;
    writeln!(lock)?;
    Ok(())
}

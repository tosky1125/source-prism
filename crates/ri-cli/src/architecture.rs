#![allow(
    clippy::redundant_pub_crate,
    reason = "Binary crate helper modules share crate-visible command handlers."
)]

use std::{
    env,
    io::{self, Write},
    path::{Path, PathBuf},
};

use ri_architecture::extract_architecture_entities_for;
use ri_core::{CommitSha, RepoId};
use ri_git::{LocalManifest, discover_worktree, resolve_commit_sha};
use ri_indexer::{ArchitectureEntityRecord, PgArchitectureStore};
use serde_json::json;
use sqlx::postgres::PgPoolOptions;

use crate::error::CliError;

pub(crate) async fn architecture_command(
    mut args: impl Iterator<Item = String>,
) -> Result<(), CliError> {
    match ArchitectureArgs::parse(&mut args)? {
        ArchitectureArgs::Worktree(repo_path) => {
            let (repo_id, body) = worktree_architecture(&repo_path)?;
            print_architecture(repo_id.as_str(), body.as_slice())
        }
        ArchitectureArgs::PersistedRepo(repo_id) => {
            let entities = persisted_architecture(&repo_id).await?;
            let body = entities
                .iter()
                .map(record_json)
                .collect::<Vec<serde_json::Value>>();
            print_architecture(repo_id.as_str(), body.as_slice())
        }
    }
}

#[derive(Debug)]
enum ArchitectureArgs {
    Worktree(PathBuf),
    PersistedRepo(String),
}

impl ArchitectureArgs {
    fn parse(args: &mut impl Iterator<Item = String>) -> Result<Self, CliError> {
        let Some(flag) = args.next() else {
            return Err(CliError::Usage);
        };
        let Some(value) = args.next() else {
            return Err(CliError::Usage);
        };
        if args.next().is_some() {
            return Err(CliError::Usage);
        }

        match flag.as_str() {
            "--repo" => Ok(Self::Worktree(PathBuf::from(value))),
            "--repo-id" => Ok(Self::PersistedRepo(value)),
            _ => Err(CliError::Usage),
        }
    }
}

fn worktree_architecture(repo_path: &Path) -> Result<(String, Vec<serde_json::Value>), CliError> {
    let worktree = discover_worktree(repo_path)?;
    let repo_id = format!("local:{}", worktree.canonicalize()?.display());
    let repo = RepoId::new(&repo_id)?;
    let commit = CommitSha::new(resolve_commit_sha(repo_path, "HEAD")?)?;
    let manifest = LocalManifest::extract(repo_path)?;
    let entities = extract_architecture_entities_for(repo_path, &repo, &commit, &manifest)?;
    let body = entities
        .iter()
        .map(|entity| {
            json!({
                "architecture_entity_id": entity.entity_id,
                "stable_entity_id": entity.stable_entity_id,
                "entity_type": entity.kind.as_str(),
                "name": entity.name,
                "file_path": entity.file_path,
                "start_line": entity.start_line,
                "end_line": entity.end_line,
                "content_hash": entity.content_hash,
            })
        })
        .collect::<Vec<_>>();
    Ok((repo_id, body))
}

async fn persisted_architecture(repo_id: &str) -> Result<Vec<ArchitectureEntityRecord>, CliError> {
    let database_url = env::var("DATABASE_URL").map_err(|_| CliError::MissingEnv {
        key: "DATABASE_URL",
    })?;
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(database_url.as_str())
        .await?;
    Ok(PgArchitectureStore::new(pool)
        .active_architecture_entities_for_repo(repo_id)
        .await?)
}

fn record_json(entity: &ArchitectureEntityRecord) -> serde_json::Value {
    json!({
        "architecture_entity_id": entity.architecture_entity_id,
        "stable_entity_id": entity.stable_entity_id,
        "entity_type": entity.entity_type,
        "name": entity.name,
        "file_path": entity.file_path,
        "start_line": entity.start_line,
        "end_line": entity.end_line,
        "content_hash": entity.content_hash,
    })
}

fn print_architecture(repo_id: &str, body: &[serde_json::Value]) -> Result<(), CliError> {
    print_json(&json!({
        "status": "ok",
        "kind": "architecture",
        "repo_id": repo_id,
        "entity_count": body.len(),
        "entities": body,
    }))
}

fn print_json(value: &serde_json::Value) -> Result<(), CliError> {
    let stdout = io::stdout();
    let mut lock = stdout.lock();
    serde_json::to_writer_pretty(&mut lock, value)?;
    writeln!(lock)?;
    Ok(())
}

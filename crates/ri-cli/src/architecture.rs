#![allow(
    clippy::redundant_pub_crate,
    reason = "Binary crate helper modules share crate-visible command handlers."
)]

use std::{
    io::{self, Write},
    path::PathBuf,
};

use ri_architecture::extract_architecture_entities_for;
use ri_core::{CommitSha, RepoId};
use ri_git::{LocalManifest, discover_worktree, resolve_commit_sha};
use serde_json::json;

use crate::error::CliError;

pub(crate) fn architecture_command(mut args: impl Iterator<Item = String>) -> Result<(), CliError> {
    let Some(flag) = args.next() else {
        return Err(CliError::Usage);
    };
    if flag != "--repo" {
        return Err(CliError::Usage);
    }
    let Some(path) = args.next() else {
        return Err(CliError::Usage);
    };
    if args.next().is_some() {
        return Err(CliError::Usage);
    }

    let repo_path = PathBuf::from(path);
    let worktree = discover_worktree(&repo_path)?;
    let repo_id = format!("local:{}", worktree.canonicalize()?.display());
    let repo = RepoId::new(&repo_id)?;
    let commit = CommitSha::new(resolve_commit_sha(&repo_path, "HEAD")?)?;
    let manifest = LocalManifest::extract(&repo_path)?;
    let entities = extract_architecture_entities_for(&repo_path, &repo, &commit, &manifest)?;
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

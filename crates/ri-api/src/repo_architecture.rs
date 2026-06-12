use axum::{
    Json,
    extract::{Path, State},
};
use ri_architecture::extract_architecture_entities_for;
use ri_core::{CommitSha, RepoId};
use ri_git::{LocalManifest, discover_worktree, resolve_commit_sha};
use ri_indexer::{ArchitectureEntityRecord, PgArchitectureStore};
use serde::Serialize;

use crate::{AppError, state::AppState};

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct ArchitectureEntity {
    #[serde(rename = "architecture_entity_id")]
    id: String,
    #[serde(rename = "stable_entity_id")]
    stable_id: String,
    entity_type: String,
    name: String,
    file_path: String,
    start_line: i32,
    end_line: i32,
    content_hash: String,
}

impl From<&ArchitectureEntityRecord> for ArchitectureEntity {
    fn from(record: &ArchitectureEntityRecord) -> Self {
        Self {
            id: record.architecture_entity_id.clone(),
            stable_id: record.stable_entity_id.clone(),
            entity_type: record.entity_type.clone(),
            name: record.name.clone(),
            file_path: record.file_path.clone(),
            start_line: record.start_line,
            end_line: record.end_line,
            content_hash: record.content_hash.clone(),
        }
    }
}

impl From<&ri_architecture::ArchitectureEntity> for ArchitectureEntity {
    fn from(entity: &ri_architecture::ArchitectureEntity) -> Self {
        Self {
            id: entity.entity_id.to_string(),
            stable_id: entity.stable_entity_id.to_string(),
            entity_type: entity.kind.as_str().to_owned(),
            name: entity.name.clone(),
            file_path: entity.file_path.to_string(),
            start_line: line_i32(entity.start_line),
            end_line: line_i32(entity.end_line),
            content_hash: entity.content_hash.clone(),
        }
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct RepoArchitectureResponse {
    status: &'static str,
    kind: &'static str,
    repo_id: String,
    entity_count: usize,
    entities: Vec<ArchitectureEntity>,
}

pub(crate) async fn list(
    State(state): State<AppState>,
    Path(repo_id): Path<String>,
) -> Result<Json<RepoArchitectureResponse>, AppError> {
    let entities = if let Some(pool) = state.database.pool.as_ref() {
        let records = PgArchitectureStore::new(pool.clone())
            .active_architecture_entities_for_repo(&repo_id)
            .await?;
        records
            .iter()
            .map(ArchitectureEntity::from)
            .collect::<Vec<_>>()
    } else {
        local_architecture_entities(&state)?
    };
    Ok(Json(RepoArchitectureResponse {
        status: "ok",
        kind: "architecture",
        repo_id,
        entity_count: entities.len(),
        entities,
    }))
}

fn local_architecture_entities(state: &AppState) -> Result<Vec<ArchitectureEntity>, AppError> {
    let repo_path = state.context_repo_path();
    let worktree = discover_worktree(repo_path)?;
    let repo = RepoId::new(format!("local:{}", worktree.display()))?;
    let commit = CommitSha::new(resolve_commit_sha(repo_path, "HEAD")?)?;
    let manifest = LocalManifest::extract(repo_path)?;
    Ok(
        extract_architecture_entities_for(repo_path, &repo, &commit, &manifest)?
            .iter()
            .map(ArchitectureEntity::from)
            .collect(),
    )
}

fn line_i32(value: u32) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}

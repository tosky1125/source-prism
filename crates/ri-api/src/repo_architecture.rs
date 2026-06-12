use axum::{
    Json,
    extract::{Path, State},
};
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
    let pool = state
        .database
        .pool
        .as_ref()
        .ok_or(AppError::DatabaseNotConfigured)?;
    let records = PgArchitectureStore::new(pool.clone())
        .active_architecture_entities_for_repo(&repo_id)
        .await?;
    let entities = records
        .iter()
        .map(ArchitectureEntity::from)
        .collect::<Vec<_>>();
    Ok(Json(RepoArchitectureResponse {
        status: "ok",
        kind: "architecture",
        repo_id,
        entity_count: entities.len(),
        entities,
    }))
}

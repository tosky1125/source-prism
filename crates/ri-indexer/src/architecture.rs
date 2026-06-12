use ri_architecture::ArchitectureEntity;
use ri_core::GenerationId;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row as _};
use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ArchitectureStoreError {
    #[error("index generation not found: {generation_id}")]
    GenerationNotFound { generation_id: String },
    #[error("invalid architecture range value: {field}={value}")]
    InvalidRange { field: &'static str, value: u32 },
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ArchitectureEntityRecord {
    pub architecture_entity_id: String,
    pub stable_entity_id: String,
    pub entity_type: String,
    pub name: String,
    pub file_path: String,
    pub start_line: i32,
    pub end_line: i32,
    pub content_hash: String,
}

#[derive(Debug, Clone)]
pub struct PgArchitectureStore {
    pool: PgPool,
}

impl PgArchitectureStore {
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn replace_architecture_entities_for_generation(
        &self,
        generation_id: &GenerationId,
        entities: &[ArchitectureEntity],
    ) -> Result<u64, ArchitectureStoreError> {
        let generation = self.generation(generation_id).await?;
        let mut transaction = self.pool.begin().await?;
        stale_previous_architecture(&mut transaction, &generation, generation_id).await?;

        let mut indexed = 0_u64;
        for entity in entities {
            let result =
                upsert_architecture_entity(&mut transaction, &generation, generation_id, entity)
                    .await?;
            indexed = indexed.saturating_add(result);
        }

        transaction.commit().await?;
        Ok(indexed)
    }

    pub async fn active_architecture_entities_for_repo(
        &self,
        repo_id: &str,
    ) -> Result<Vec<ArchitectureEntityRecord>, ArchitectureStoreError> {
        let rows = sqlx::query(
            r"
            SELECT architecture_entity_id, stable_entity_id, entity_type, name,
                   file_path, start_line, end_line, content_hash
            FROM architecture_entities
            WHERE repo_id = $1
              AND stale_at IS NULL
              AND generation_id = (
                  SELECT generation_id
                  FROM index_generations
                  WHERE repo_id = $1 AND status = 'succeeded'
                  ORDER BY started_at DESC
                  LIMIT 1
              )
            ORDER BY entity_type, file_path
            ",
        )
        .bind(repo_id)
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(architecture_entity_from_row).collect()
    }

    async fn generation(
        &self,
        generation_id: &GenerationId,
    ) -> Result<StoredGeneration, ArchitectureStoreError> {
        let row = sqlx::query(
            r"
            SELECT repo_id, commit_sha
            FROM index_generations
            WHERE generation_id = $1
            ",
        )
        .bind(generation_id.to_string())
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| ArchitectureStoreError::GenerationNotFound {
            generation_id: generation_id.to_string(),
        })?;
        Ok(StoredGeneration {
            repo_id: row.try_get("repo_id")?,
            commit_sha: row.try_get("commit_sha")?,
        })
    }
}

#[derive(Debug)]
struct StoredGeneration {
    repo_id: String,
    commit_sha: String,
}

async fn stale_previous_architecture(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    generation: &StoredGeneration,
    generation_id: &GenerationId,
) -> Result<(), ArchitectureStoreError> {
    sqlx::query(
        r"
        UPDATE architecture_entities
        SET stale_at = now()
        WHERE repo_id = $1
          AND commit_sha = $2
          AND generation_id <> $3
          AND stale_at IS NULL
        ",
    )
    .bind(&generation.repo_id)
    .bind(&generation.commit_sha)
    .bind(generation_id.to_string())
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

async fn upsert_architecture_entity(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    generation: &StoredGeneration,
    generation_id: &GenerationId,
    entity: &ArchitectureEntity,
) -> Result<u64, ArchitectureStoreError> {
    let start_line = range_value(entity.start_line, "start_line")?;
    let end_line = range_value(entity.end_line, "end_line")?;
    let result = sqlx::query(
        r"
        INSERT INTO architecture_entities (
            architecture_entity_id, stable_entity_id, repo_id, commit_sha, generation_id,
            entity_type, name, file_path, start_line, end_line, content_hash, stale_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, NULL)
        ON CONFLICT (architecture_entity_id) DO UPDATE
        SET stable_entity_id = EXCLUDED.stable_entity_id,
            generation_id = EXCLUDED.generation_id,
            entity_type = EXCLUDED.entity_type,
            name = EXCLUDED.name,
            file_path = EXCLUDED.file_path,
            start_line = EXCLUDED.start_line,
            end_line = EXCLUDED.end_line,
            content_hash = EXCLUDED.content_hash,
            stale_at = NULL
        ",
    )
    .bind(entity.entity_id.to_string())
    .bind(entity.stable_entity_id.to_string())
    .bind(&generation.repo_id)
    .bind(&generation.commit_sha)
    .bind(generation_id.to_string())
    .bind(entity.kind.as_str())
    .bind(&entity.name)
    .bind(entity.file_path.to_string())
    .bind(start_line)
    .bind(end_line)
    .bind(&entity.content_hash)
    .execute(&mut **transaction)
    .await?;
    Ok(result.rows_affected())
}

fn architecture_entity_from_row(
    row: &sqlx::postgres::PgRow,
) -> Result<ArchitectureEntityRecord, ArchitectureStoreError> {
    Ok(ArchitectureEntityRecord {
        architecture_entity_id: row.try_get("architecture_entity_id")?,
        stable_entity_id: row.try_get("stable_entity_id")?,
        entity_type: row.try_get("entity_type")?,
        name: row.try_get("name")?,
        file_path: row.try_get("file_path")?,
        start_line: row.try_get("start_line")?,
        end_line: row.try_get("end_line")?,
        content_hash: row.try_get("content_hash")?,
    })
}

fn range_value(value: u32, field: &'static str) -> Result<i32, ArchitectureStoreError> {
    i32::try_from(value).map_err(|_| ArchitectureStoreError::InvalidRange { field, value })
}

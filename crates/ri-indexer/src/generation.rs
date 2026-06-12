use ri_core::GenerationId;
use sqlx::{PgPool, Row as _};
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum GenerationError {
    #[error("index generation {generation_id} is not started")]
    GenerationNotStarted { generation_id: String },
    #[error("index generation {generation_id} was not found")]
    GenerationNotFound { generation_id: String },
    #[error(transparent)]
    Core(#[from] ri_core::CoreError),
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct GenerationRecord {
    pub generation_id: GenerationId,
    pub repo_id: String,
    pub commit_sha: String,
    pub index_kind: String,
    pub status: GenerationStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum GenerationStatus {
    Started,
    Succeeded,
    Failed,
    Cancelled,
}

impl GenerationStatus {
    fn parse(raw: &str) -> Result<Self, GenerationError> {
        match raw {
            "started" => Ok(Self::Started),
            "succeeded" => Ok(Self::Succeeded),
            "failed" => Ok(Self::Failed),
            "cancelled" => Ok(Self::Cancelled),
            other => Err(GenerationError::GenerationNotStarted {
                generation_id: other.to_owned(),
            }),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PgGenerationStore {
    pub(crate) pool: PgPool,
}

impl PgGenerationStore {
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn begin_generation(
        &self,
        repo_id: &str,
        commit_sha: &str,
        index_kind: &str,
        extractor_version: Option<&str>,
    ) -> Result<GenerationRecord, GenerationError> {
        let generation_id = GenerationId::new(Uuid::now_v7().to_string())?;
        let row = sqlx::query(
            r"
            INSERT INTO index_generations (
                generation_id, repo_id, commit_sha, index_kind, status, extractor_version
            )
            VALUES ($1, $2, $3, $4, 'started', $5)
            RETURNING generation_id, repo_id, commit_sha, index_kind, status
            ",
        )
        .bind(generation_id.to_string())
        .bind(repo_id)
        .bind(commit_sha)
        .bind(index_kind)
        .bind(extractor_version)
        .fetch_one(&self.pool)
        .await?;
        generation_from_row(&row)
    }

    pub async fn finish_generation(
        &self,
        generation_id: &GenerationId,
    ) -> Result<(), GenerationError> {
        let result = sqlx::query(
            r"
            UPDATE index_generations
            SET status = 'succeeded', finished_at = now()
            WHERE generation_id = $1 AND status = 'started'
            ",
        )
        .bind(generation_id.to_string())
        .execute(&self.pool)
        .await?;
        ensure_updated(result.rows_affected(), generation_id)
    }

    pub async fn fail_generation(
        &self,
        generation_id: &GenerationId,
        error: &str,
    ) -> Result<(), GenerationError> {
        let result = sqlx::query(
            r"
            UPDATE index_generations
            SET status = 'failed', failed_at = now(), error = $2
            WHERE generation_id = $1 AND status = 'started'
            ",
        )
        .bind(generation_id.to_string())
        .bind(error)
        .execute(&self.pool)
        .await?;
        ensure_updated(result.rows_affected(), generation_id)
    }

    pub(crate) async fn started_generation(
        &self,
        generation_id: &GenerationId,
    ) -> Result<GenerationRecord, GenerationError> {
        let row = sqlx::query(
            r"
            SELECT generation_id, repo_id, commit_sha, index_kind, status
            FROM index_generations
            WHERE generation_id = $1
            ",
        )
        .bind(generation_id.to_string())
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| GenerationError::GenerationNotFound {
            generation_id: generation_id.to_string(),
        })?;
        let generation = generation_from_row(&row)?;
        if generation.status != GenerationStatus::Started {
            return Err(GenerationError::GenerationNotStarted {
                generation_id: generation_id.to_string(),
            });
        }
        Ok(generation)
    }
}

fn generation_from_row(row: &sqlx::postgres::PgRow) -> Result<GenerationRecord, GenerationError> {
    Ok(GenerationRecord {
        generation_id: GenerationId::new(row.try_get::<String, _>("generation_id")?)?,
        repo_id: row.try_get("repo_id")?,
        commit_sha: row.try_get("commit_sha")?,
        index_kind: row.try_get("index_kind")?,
        status: GenerationStatus::parse(row.try_get::<String, _>("status")?.as_str())?,
    })
}

fn ensure_updated(rows_affected: u64, generation_id: &GenerationId) -> Result<(), GenerationError> {
    if rows_affected == 0 {
        return Err(GenerationError::GenerationNotStarted {
            generation_id: generation_id.to_string(),
        });
    }
    Ok(())
}

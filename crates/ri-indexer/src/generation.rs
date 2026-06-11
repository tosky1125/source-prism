use ri_core::GenerationId;
use sha2::{Digest, Sha256};
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

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "File manifest flags mirror the canonical storage schema."
)]
#[non_exhaustive]
pub struct FileManifestInput {
    pub file_path: String,
    pub language: String,
    pub content_sha256: String,
    pub size_bytes: i64,
    pub mode: Option<String>,
    pub is_binary: bool,
    pub is_generated: bool,
    pub is_vendor: bool,
    pub is_test: bool,
}

impl FileManifestInput {
    pub fn new(file_path: &str, content_sha256: &str, size_bytes: i64) -> Self {
        Self {
            file_path: file_path.to_owned(),
            language: "unknown".to_owned(),
            content_sha256: content_sha256.to_owned(),
            size_bytes,
            mode: None,
            is_binary: false,
            is_generated: false,
            is_vendor: false,
            is_test: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PgGenerationStore {
    pool: PgPool,
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

    pub async fn replace_file_manifest_generation(
        &self,
        generation_id: &GenerationId,
        manifests: &[FileManifestInput],
    ) -> Result<u64, GenerationError> {
        let generation = self.started_generation(generation_id).await?;
        let mut transaction = self.pool.begin().await?;
        let mut inserted = 0_u64;

        for manifest in manifests {
            sqlx::query(
                r"
                UPDATE file_manifests
                SET stale_at = now()
                WHERE repo_id = $1
                  AND commit_sha = $2
                  AND file_path = $3
                  AND stale_at IS NULL
                ",
            )
            .bind(&generation.repo_id)
            .bind(&generation.commit_sha)
            .bind(&manifest.file_path)
            .execute(&mut *transaction)
            .await?;

            let result = sqlx::query(
                r"
                INSERT INTO file_manifests (
                    file_manifest_id, repo_id, commit_sha, generation_id, file_path,
                    language, content_sha256, size_bytes, mode, is_binary, is_generated,
                    is_vendor, is_test
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
                ",
            )
            .bind(file_manifest_id(&generation, manifest))
            .bind(&generation.repo_id)
            .bind(&generation.commit_sha)
            .bind(generation_id.to_string())
            .bind(&manifest.file_path)
            .bind(&manifest.language)
            .bind(&manifest.content_sha256)
            .bind(manifest.size_bytes)
            .bind(&manifest.mode)
            .bind(manifest.is_binary)
            .bind(manifest.is_generated)
            .bind(manifest.is_vendor)
            .bind(manifest.is_test)
            .execute(&mut *transaction)
            .await?;
            inserted = inserted.saturating_add(result.rows_affected());
        }

        sqlx::query(
            r"
            UPDATE file_manifests
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
        .execute(&mut *transaction)
        .await?;

        sqlx::query(
            r"
            UPDATE index_generations
            SET status = 'succeeded', finished_at = now()
            WHERE generation_id = $1 AND status = 'started'
            ",
        )
        .bind(generation_id.to_string())
        .execute(&mut *transaction)
        .await?;

        transaction.commit().await?;
        Ok(inserted)
    }

    async fn started_generation(
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

fn file_manifest_id(generation: &GenerationRecord, manifest: &FileManifestInput) -> String {
    let mut hasher = Sha256::new();
    for part in [
        generation.repo_id.as_str(),
        generation.commit_sha.as_str(),
        generation.generation_id.as_str(),
        manifest.file_path.as_str(),
        manifest.content_sha256.as_str(),
    ] {
        hash_part(&mut hasher, part);
    }
    format!("fm:{}", hex::encode(hasher.finalize()))
}

fn hash_part(hasher: &mut Sha256, part: &str) {
    hasher.update(part.len().to_string().as_bytes());
    hasher.update(b":");
    hasher.update(part.as_bytes());
    hasher.update(b";");
}

use sha2::{Digest, Sha256};
use sqlx::Row as _;

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum FileOverlayStoreError {
    #[error("no succeeded base generation for repo: {repo_id}")]
    MissingBaseGeneration { repo_id: String },
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct FileOverlayInput {
    pub file_path: String,
    pub previous_file_path: Option<String>,
    pub status: FileOverlayStatus,
    pub language: String,
    pub content_sha256: Option<String>,
    pub size_bytes: Option<i64>,
}

impl FileOverlayInput {
    pub fn new(
        file_path: &str,
        previous_file_path: Option<String>,
        status: FileOverlayStatus,
    ) -> Self {
        Self {
            file_path: file_path.to_owned(),
            previous_file_path,
            status,
            language: "unknown".to_owned(),
            content_sha256: None,
            size_bytes: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum FileOverlayStatus {
    Added,
    Modified,
    Deleted,
    Renamed,
    ModeOnly,
}

impl FileOverlayStatus {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Added => "added",
            Self::Modified => "modified",
            Self::Deleted => "deleted",
            Self::Renamed => "renamed",
            Self::ModeOnly => "mode_only",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct OverlayBaseGeneration {
    pub generation_id: String,
    pub commit_sha: String,
}

#[derive(Clone, Debug)]
pub struct PgFileOverlayStore {
    pool: sqlx::PgPool,
}

impl PgFileOverlayStore {
    pub const fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }

    pub async fn latest_base_generation(
        &self,
        repo_id: &str,
    ) -> Result<OverlayBaseGeneration, FileOverlayStoreError> {
        let row = sqlx::query(
            r"
            SELECT generation_id, commit_sha
            FROM index_generations
            WHERE repo_id = $1
              AND status = 'succeeded'
              AND index_kind = 'file_manifest'
            ORDER BY started_at DESC
            LIMIT 1
            ",
        )
        .bind(repo_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| FileOverlayStoreError::MissingBaseGeneration {
            repo_id: repo_id.to_owned(),
        })?;
        Ok(OverlayBaseGeneration {
            generation_id: row.try_get("generation_id")?,
            commit_sha: row.try_get("commit_sha")?,
        })
    }

    pub async fn replace_overlay(
        &self,
        repo_id: &str,
        base: &OverlayBaseGeneration,
        head_sha: &str,
        files: &[FileOverlayInput],
    ) -> Result<u64, FileOverlayStoreError> {
        let mut transaction = self.pool.begin().await?;
        sqlx::query(
            r"
            DELETE FROM file_overlays
            WHERE repo_id = $1 AND base_generation_id = $2 AND head_sha = $3
            ",
        )
        .bind(repo_id)
        .bind(&base.generation_id)
        .bind(head_sha)
        .execute(&mut *transaction)
        .await?;

        let mut inserted = 0_u64;
        for file in files {
            let result = sqlx::query(
                r"
                INSERT INTO file_overlays (
                    file_overlay_id, repo_id, base_generation_id, base_commit_sha,
                    head_sha, file_path, previous_file_path, status, language,
                    content_sha256, size_bytes
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
                ",
            )
            .bind(file_overlay_id(repo_id, base, head_sha, file))
            .bind(repo_id)
            .bind(&base.generation_id)
            .bind(&base.commit_sha)
            .bind(head_sha)
            .bind(&file.file_path)
            .bind(&file.previous_file_path)
            .bind(file.status.as_str())
            .bind(&file.language)
            .bind(&file.content_sha256)
            .bind(file.size_bytes)
            .execute(&mut *transaction)
            .await?;
            inserted = inserted.saturating_add(result.rows_affected());
        }

        transaction.commit().await?;
        Ok(inserted)
    }
}

fn file_overlay_id(
    repo_id: &str,
    base: &OverlayBaseGeneration,
    head_sha: &str,
    file: &FileOverlayInput,
) -> String {
    let mut hasher = Sha256::new();
    for part in [
        repo_id,
        base.generation_id.as_str(),
        head_sha,
        file.file_path.as_str(),
        file.status.as_str(),
    ] {
        hash_part(&mut hasher, part);
    }
    format!("fo:{}", hex::encode(hasher.finalize()))
}

fn hash_part(hasher: &mut Sha256, part: &str) {
    hasher.update(part.len().to_string().as_bytes());
    hasher.update(b":");
    hasher.update(part.as_bytes());
    hasher.update(b";");
}

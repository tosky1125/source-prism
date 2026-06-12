use sqlx::Row as _;

use crate::{GenerationError, PgGenerationStore};

#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct FileManifestRecord {
    pub file_path: String,
    pub language: String,
    pub content_sha256: String,
    pub size_bytes: i64,
    pub is_generated: bool,
    pub is_vendor: bool,
    pub is_test: bool,
}

impl PgGenerationStore {
    pub async fn active_file_manifests_for_repo(
        &self,
        repo_id: &str,
    ) -> Result<Vec<FileManifestRecord>, GenerationError> {
        let rows = sqlx::query(
            r"
            SELECT file_path, language, content_sha256, size_bytes,
                   is_generated, is_vendor, is_test
            FROM file_manifests
            WHERE repo_id = $1
              AND stale_at IS NULL
              AND generation_id = (
                  SELECT generation_id
                  FROM index_generations
                  WHERE repo_id = $1 AND status = 'succeeded'
                  ORDER BY started_at DESC
                  LIMIT 1
              )
            ORDER BY file_path
            ",
        )
        .bind(repo_id)
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(file_manifest_from_row).collect()
    }
}

fn file_manifest_from_row(
    row: &sqlx::postgres::PgRow,
) -> Result<FileManifestRecord, GenerationError> {
    Ok(FileManifestRecord {
        file_path: row.try_get("file_path")?,
        language: row.try_get("language")?,
        content_sha256: row.try_get("content_sha256")?,
        size_bytes: row.try_get("size_bytes")?,
        is_generated: row.try_get("is_generated")?,
        is_vendor: row.try_get("is_vendor")?,
        is_test: row.try_get("is_test")?,
    })
}

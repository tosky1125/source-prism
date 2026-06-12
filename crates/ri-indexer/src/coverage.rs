use ri_behavior::CoverageReport;
use ri_core::GenerationId;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Row as _};
use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum CoverageStoreError {
    #[error("index generation not found: {generation_id}")]
    GenerationNotFound { generation_id: String },
    #[error("invalid coverage value: {field}={value}")]
    InvalidValue { field: &'static str, value: u32 },
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CoverageIngestOutcome {
    pub segment_count: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CoverageSegmentRecord {
    pub coverage_segment_id: String,
    pub source_path: String,
    pub file_path: String,
    pub start_line: i32,
    pub end_line: i32,
    pub hit_count: i32,
    pub format: String,
}

#[derive(Debug, Clone)]
pub struct PgCoverageStore {
    pool: PgPool,
}

impl PgCoverageStore {
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn replace_lcov_for_generation(
        &self,
        generation_id: &GenerationId,
        source_path: &str,
        report: &CoverageReport,
    ) -> Result<CoverageIngestOutcome, CoverageStoreError> {
        let generation = self.generation(generation_id).await?;
        let mut transaction = self.pool.begin().await?;
        stale_previous_segments(&mut transaction, &generation, generation_id, source_path).await?;
        let mut segment_count = 0_u64;
        for file in &report.files {
            for segment in &file.segments {
                let result = upsert_segment(
                    &mut transaction,
                    &generation,
                    generation_id,
                    source_path,
                    file.file_path.as_str(),
                    segment,
                )
                .await?;
                segment_count = segment_count.saturating_add(result);
            }
        }
        transaction.commit().await?;
        Ok(CoverageIngestOutcome { segment_count })
    }

    pub async fn active_coverage_segments_for_repo(
        &self,
        repo_id: &str,
    ) -> Result<Vec<CoverageSegmentRecord>, CoverageStoreError> {
        let rows = sqlx::query(
            r"
            SELECT coverage_segment_id, source_path, file_path, start_line, end_line,
                   hit_count, format
            FROM coverage_segments
            WHERE repo_id = $1 AND stale_at IS NULL
            ORDER BY file_path, start_line, end_line
            ",
        )
        .bind(repo_id)
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(segment_from_row).collect()
    }

    async fn generation(
        &self,
        generation_id: &GenerationId,
    ) -> Result<StoredGeneration, CoverageStoreError> {
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
        .ok_or_else(|| CoverageStoreError::GenerationNotFound {
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

async fn stale_previous_segments(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    generation: &StoredGeneration,
    generation_id: &GenerationId,
    source_path: &str,
) -> Result<(), CoverageStoreError> {
    sqlx::query(
        r"
        UPDATE coverage_segments
        SET stale_at = now()
        WHERE repo_id = $1 AND commit_sha = $2 AND source_path = $3
          AND generation_id <> $4 AND stale_at IS NULL
        ",
    )
    .bind(&generation.repo_id)
    .bind(&generation.commit_sha)
    .bind(source_path)
    .bind(generation_id.to_string())
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

async fn upsert_segment(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    generation: &StoredGeneration,
    generation_id: &GenerationId,
    source_path: &str,
    file_path: &str,
    segment: &ri_behavior::CoverageSegment,
) -> Result<u64, CoverageStoreError> {
    let start_line = coverage_value(segment.start_line, "start_line")?;
    let end_line = coverage_value(segment.end_line, "end_line")?;
    let hit_count = coverage_value(segment.hit_count, "hit_count")?;
    let result = sqlx::query(
        r"
        INSERT INTO coverage_segments (
            coverage_segment_id, repo_id, commit_sha, generation_id, source_path,
            file_path, start_line, end_line, hit_count, format, stale_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'lcov', NULL)
        ON CONFLICT (coverage_segment_id) DO UPDATE
        SET generation_id = EXCLUDED.generation_id,
            hit_count = EXCLUDED.hit_count,
            stale_at = NULL
        ",
    )
    .bind(segment_id(
        generation,
        generation_id,
        source_path,
        file_path,
        segment,
    ))
    .bind(&generation.repo_id)
    .bind(&generation.commit_sha)
    .bind(generation_id.to_string())
    .bind(source_path)
    .bind(file_path)
    .bind(start_line)
    .bind(end_line)
    .bind(hit_count)
    .execute(&mut **transaction)
    .await?;
    Ok(result.rows_affected())
}

fn segment_from_row(
    row: &sqlx::postgres::PgRow,
) -> Result<CoverageSegmentRecord, CoverageStoreError> {
    Ok(CoverageSegmentRecord {
        coverage_segment_id: row.try_get("coverage_segment_id")?,
        source_path: row.try_get("source_path")?,
        file_path: row.try_get("file_path")?,
        start_line: row.try_get("start_line")?,
        end_line: row.try_get("end_line")?,
        hit_count: row.try_get("hit_count")?,
        format: row.try_get("format")?,
    })
}

fn coverage_value(value: u32, field: &'static str) -> Result<i32, CoverageStoreError> {
    i32::try_from(value).map_err(|_| CoverageStoreError::InvalidValue { field, value })
}

fn segment_id(
    generation: &StoredGeneration,
    generation_id: &GenerationId,
    source_path: &str,
    file_path: &str,
    segment: &ri_behavior::CoverageSegment,
) -> String {
    digest(
        "covseg",
        &[
            generation.repo_id.as_str(),
            generation.commit_sha.as_str(),
            generation_id.as_str(),
            source_path,
            file_path,
            segment.start_line.to_string().as_str(),
            segment.end_line.to_string().as_str(),
        ],
    )
}

fn digest(prefix: &str, parts: &[&str]) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update(part.len().to_string().as_bytes());
        hasher.update(b":");
        hasher.update(part.as_bytes());
        hasher.update(b";");
    }
    format!("{prefix}:{}", hex::encode(hasher.finalize()))
}

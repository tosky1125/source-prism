use ri_core::{GenerationId, Language, SymbolKind};
use ri_symbols::{SymbolRange, SymbolRecord};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row as _};
use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum TestCaseStoreError {
    #[error("index generation not found: {generation_id}")]
    GenerationNotFound { generation_id: String },
    #[error("invalid test case range value: {field}={value}")]
    InvalidRange { field: &'static str, value: u32 },
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct TestCaseRecord {
    pub test_case_id: String,
    pub stable_test_id: String,
    pub symbol_id: Option<String>,
    pub file_path: String,
    pub language: String,
    pub name: String,
    pub fqn: String,
    pub range: SymbolRange,
}

impl TestCaseRecord {
    pub fn from_symbol(symbol: &SymbolRecord) -> Self {
        Self {
            test_case_id: symbol.versioned_symbol_id.to_string(),
            stable_test_id: symbol.stable_symbol_id.to_string(),
            symbol_id: Some(symbol.versioned_symbol_id.to_string()),
            file_path: symbol.file_path.as_str().to_owned(),
            language: language_id(symbol.language).to_owned(),
            name: symbol.name.clone(),
            fqn: symbol.fqn.clone(),
            range: symbol.range.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PgTestCaseStore {
    pool: PgPool,
}

impl PgTestCaseStore {
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn replace_test_cases_for_generation(
        &self,
        generation_id: &GenerationId,
        symbols: &[SymbolRecord],
    ) -> Result<u64, TestCaseStoreError> {
        let generation = self.generation(generation_id).await?;
        let mut transaction = self.pool.begin().await?;
        stale_previous_test_cases(&mut transaction, &generation, generation_id).await?;

        let mut indexed = 0_u64;
        for symbol in symbols
            .iter()
            .filter(|symbol| symbol.kind == SymbolKind::TestCase)
        {
            let result =
                upsert_test_case(&mut transaction, &generation, generation_id, symbol).await?;
            indexed = indexed.saturating_add(result);
        }

        transaction.commit().await?;
        Ok(indexed)
    }

    pub async fn active_test_cases_for_repo(
        &self,
        repo_id: &str,
    ) -> Result<Vec<TestCaseRecord>, TestCaseStoreError> {
        let rows = sqlx::query(
            r"
            SELECT test_case_id, stable_test_id, symbol_id, file_path, language,
                   name, fqn, start_line, start_col, end_line, end_col
            FROM test_cases
            WHERE repo_id = $1
              AND stale_at IS NULL
              AND generation_id = (
                  SELECT generation_id
                  FROM index_generations
                  WHERE repo_id = $1 AND status = 'succeeded'
                  ORDER BY started_at DESC
                  LIMIT 1
              )
            ORDER BY file_path, fqn, start_line
            ",
        )
        .bind(repo_id)
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(test_case_from_row).collect()
    }

    async fn generation(
        &self,
        generation_id: &GenerationId,
    ) -> Result<StoredGeneration, TestCaseStoreError> {
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
        .ok_or_else(|| TestCaseStoreError::GenerationNotFound {
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

async fn stale_previous_test_cases(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    generation: &StoredGeneration,
    generation_id: &GenerationId,
) -> Result<(), TestCaseStoreError> {
    sqlx::query(
        r"
        UPDATE test_cases
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

async fn upsert_test_case(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    generation: &StoredGeneration,
    generation_id: &GenerationId,
    symbol: &SymbolRecord,
) -> Result<u64, TestCaseStoreError> {
    let start_line = range_value(symbol.range.start_line, "start_line")?;
    let start_col = range_value(symbol.range.start_column, "start_col")?;
    let end_line = range_value(symbol.range.end_line, "end_line")?;
    let end_col = range_value(symbol.range.end_column, "end_col")?;
    let result = sqlx::query(
        r"
        INSERT INTO test_cases (
            test_case_id, stable_test_id, symbol_id, repo_id, commit_sha, generation_id,
            file_path, language, name, fqn, start_line, start_col, end_line, end_col, stale_at
        )
        VALUES (
            $1, $2,
            (SELECT symbol_id FROM symbols WHERE symbol_id = $1 AND stale_at IS NULL LIMIT 1),
            $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, NULL
        )
        ON CONFLICT (test_case_id) DO UPDATE
        SET stable_test_id = EXCLUDED.stable_test_id,
            symbol_id = EXCLUDED.symbol_id,
            generation_id = EXCLUDED.generation_id,
            file_path = EXCLUDED.file_path,
            language = EXCLUDED.language,
            name = EXCLUDED.name,
            fqn = EXCLUDED.fqn,
            start_line = EXCLUDED.start_line,
            start_col = EXCLUDED.start_col,
            end_line = EXCLUDED.end_line,
            end_col = EXCLUDED.end_col,
            stale_at = NULL
        ",
    )
    .bind(symbol.versioned_symbol_id.to_string())
    .bind(symbol.stable_symbol_id.to_string())
    .bind(&generation.repo_id)
    .bind(&generation.commit_sha)
    .bind(generation_id.to_string())
    .bind(symbol.file_path.to_string())
    .bind(language_id(symbol.language))
    .bind(&symbol.name)
    .bind(&symbol.fqn)
    .bind(start_line)
    .bind(start_col)
    .bind(end_line)
    .bind(end_col)
    .execute(&mut **transaction)
    .await?;
    Ok(result.rows_affected())
}

fn test_case_from_row(row: &sqlx::postgres::PgRow) -> Result<TestCaseRecord, TestCaseStoreError> {
    Ok(TestCaseRecord {
        test_case_id: row.try_get("test_case_id")?,
        stable_test_id: row.try_get("stable_test_id")?,
        symbol_id: row.try_get("symbol_id")?,
        file_path: row.try_get("file_path")?,
        language: row.try_get("language")?,
        name: row.try_get("name")?,
        fqn: row.try_get("fqn")?,
        range: SymbolRange::new(
            u32::try_from(row.try_get::<i32, _>("start_line")?).unwrap_or(u32::MAX),
            u32::try_from(row.try_get::<i32, _>("start_col")?).unwrap_or(u32::MAX),
            u32::try_from(row.try_get::<i32, _>("end_line")?).unwrap_or(u32::MAX),
            u32::try_from(row.try_get::<i32, _>("end_col")?).unwrap_or(u32::MAX),
        ),
    })
}

fn range_value(value: u32, field: &'static str) -> Result<i32, TestCaseStoreError> {
    i32::try_from(value).map_err(|_| TestCaseStoreError::InvalidRange { field, value })
}

const fn language_id(language: Language) -> &'static str {
    match language {
        Language::TypeScript => "typescript",
        Language::JavaScript => "javascript",
        Language::Php => "php",
        Language::Python => "python",
        Language::Java => "java",
        Language::Go => "go",
        Language::Rust => "rust",
        _ => "unknown",
    }
}

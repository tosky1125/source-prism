use ri_core::GenerationId;
use ri_symbols::SymbolRecord;
use sqlx::{PgPool, Row as _};

use super::{
    SymbolStoreError,
    codec::{kind_id, language_id, range_value, symbol_from_row},
};

#[derive(Debug, Clone)]
pub struct PgSymbolStore {
    pool: PgPool,
}

impl PgSymbolStore {
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn replace_symbol_generation(
        &self,
        generation_id: &GenerationId,
        symbols: &[SymbolRecord],
    ) -> Result<u64, SymbolStoreError> {
        let generation = self.generation(generation_id).await?;
        let mut transaction = self.pool.begin().await?;
        stale_previous_symbols(&mut transaction, &generation, generation_id).await?;

        let mut indexed = 0_u64;
        for symbol in symbols {
            let result =
                upsert_symbol(&mut transaction, &generation, generation_id, symbol).await?;
            indexed = indexed.saturating_add(result);
        }

        transaction.commit().await?;
        Ok(indexed)
    }

    pub async fn active_symbols_for_repo(
        &self,
        repo_id: &str,
    ) -> Result<Vec<SymbolRecord>, SymbolStoreError> {
        let rows = sqlx::query(
            r"
            SELECT s.symbol_id, s.stable_symbol_id, s.commit_sha, s.file_path, s.language,
                   s.kind, s.name, s.fqn, s.start_line, s.start_col, s.end_line, s.end_col
            FROM symbols s
            WHERE s.repo_id = $1
              AND s.stale_at IS NULL
              AND s.generation_id = (
                  SELECT generation_id
                  FROM index_generations
                  WHERE repo_id = $1 AND status = 'succeeded'
                  ORDER BY started_at DESC
                  LIMIT 1
              )
            ORDER BY s.file_path, s.fqn, s.start_line
            ",
        )
        .bind(repo_id)
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(symbol_from_row).collect()
    }

    async fn generation(
        &self,
        generation_id: &GenerationId,
    ) -> Result<StoredGeneration, SymbolStoreError> {
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
        .ok_or_else(|| SymbolStoreError::GenerationNotFound {
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

async fn stale_previous_symbols(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    generation: &StoredGeneration,
    generation_id: &GenerationId,
) -> Result<(), SymbolStoreError> {
    sqlx::query(
        r"
        UPDATE symbols
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

async fn upsert_symbol(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    generation: &StoredGeneration,
    generation_id: &GenerationId,
    symbol: &SymbolRecord,
) -> Result<u64, SymbolStoreError> {
    let start_line = range_value(symbol.range.start_line, "start_line")?;
    let start_col = range_value(symbol.range.start_column, "start_col")?;
    let end_line = range_value(symbol.range.end_line, "end_line")?;
    let end_col = range_value(symbol.range.end_column, "end_col")?;
    let result = sqlx::query(
        r"
        INSERT INTO symbols (
            symbol_id, stable_symbol_id, repo_id, commit_sha, generation_id,
            file_manifest_id, file_path, language, kind, name, fqn,
            start_line, start_col, end_line, end_col, content_hash, confidence, stale_at
        )
        VALUES (
            $1, $2, $3, $4, $5,
            (
                SELECT file_manifest_id
                FROM file_manifests
                WHERE repo_id = $3 AND commit_sha = $4 AND file_path = $6 AND stale_at IS NULL
                LIMIT 1
            ),
            $6, $7, $8, $9, $10,
            $11, $12, $13, $14,
            COALESCE((
                SELECT content_sha256
                FROM file_manifests
                WHERE repo_id = $3 AND commit_sha = $4 AND file_path = $6 AND stale_at IS NULL
                LIMIT 1
            ), ''),
            'exact',
            NULL
        )
        ON CONFLICT (symbol_id) DO UPDATE
        SET generation_id = EXCLUDED.generation_id,
            file_manifest_id = EXCLUDED.file_manifest_id,
            file_path = EXCLUDED.file_path,
            language = EXCLUDED.language,
            kind = EXCLUDED.kind,
            name = EXCLUDED.name,
            fqn = EXCLUDED.fqn,
            start_line = EXCLUDED.start_line,
            start_col = EXCLUDED.start_col,
            end_line = EXCLUDED.end_line,
            end_col = EXCLUDED.end_col,
            content_hash = EXCLUDED.content_hash,
            confidence = EXCLUDED.confidence,
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
    .bind(kind_id(symbol.kind))
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

use sqlx::{PgPool, Row as _};

use crate::{
    EmbeddingCacheEntry, EmbeddingCacheError, EmbeddingCacheInput, EmbeddingCacheWrite,
    EmbeddingVector,
};

#[derive(Debug, Clone)]
pub struct PgEmbeddingCache {
    pool: PgPool,
}

impl PgEmbeddingCache {
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn store_or_touch(
        &self,
        input: &EmbeddingCacheInput,
        vector: &EmbeddingVector,
    ) -> Result<EmbeddingCacheWrite, EmbeddingCacheError> {
        ensure_dimensions(input, vector)?;
        if let Some(entry) = self.cached(input).await? {
            touch_cache_entry(&self.pool, entry.cache_key.as_str()).await?;
            return Ok(EmbeddingCacheWrite {
                cache_hit: true,
                entry,
            });
        }
        let entry = insert_cache_entry(&self.pool, input, vector).await?;
        Ok(EmbeddingCacheWrite {
            cache_hit: false,
            entry,
        })
    }

    async fn cached(
        &self,
        input: &EmbeddingCacheInput,
    ) -> Result<Option<EmbeddingCacheEntry>, EmbeddingCacheError> {
        let row = sqlx::query(
            r"
            SELECT cache_key, provider, model, input_sha256, input_kind,
                   dimensions, embedding_f32, metadata
            FROM embedding_cache
            WHERE provider = $1 AND model = $2 AND input_sha256 = $3 AND dimensions = $4
            ",
        )
        .bind(&input.provider)
        .bind(&input.model)
        .bind(input.input_sha256())
        .bind(input.dimensions)
        .fetch_optional(&self.pool)
        .await?;
        row.map(|row| entry_from_row(&row)).transpose()
    }
}

fn ensure_dimensions(
    input: &EmbeddingCacheInput,
    vector: &EmbeddingVector,
) -> Result<(), EmbeddingCacheError> {
    let actual = vector.len();
    if usize::try_from(input.dimensions).ok() == Some(actual) {
        Ok(())
    } else {
        Err(EmbeddingCacheError::DimensionMismatch {
            expected: input.dimensions,
            actual,
        })
    }
}

async fn touch_cache_entry(pool: &PgPool, cache_key: &str) -> Result<(), EmbeddingCacheError> {
    sqlx::query(
        r"
        UPDATE embedding_cache
        SET last_accessed_at = now()
        WHERE cache_key = $1
        ",
    )
    .bind(cache_key)
    .execute(pool)
    .await?;
    Ok(())
}

async fn insert_cache_entry(
    pool: &PgPool,
    input: &EmbeddingCacheInput,
    vector: &EmbeddingVector,
) -> Result<EmbeddingCacheEntry, EmbeddingCacheError> {
    let row = sqlx::query(
        r"
        INSERT INTO embedding_cache (
            cache_key, provider, model, input_sha256, input_kind,
            dimensions, embedding_f32, metadata
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, '{}'::jsonb)
        RETURNING cache_key, provider, model, input_sha256, input_kind,
                  dimensions, embedding_f32, metadata
        ",
    )
    .bind(input.cache_key())
    .bind(&input.provider)
    .bind(&input.model)
    .bind(input.input_sha256())
    .bind(&input.input_kind)
    .bind(input.dimensions)
    .bind(vector.encode_le())
    .fetch_one(pool)
    .await?;
    entry_from_row(&row)
}

fn entry_from_row(row: &sqlx::postgres::PgRow) -> Result<EmbeddingCacheEntry, EmbeddingCacheError> {
    Ok(EmbeddingCacheEntry {
        cache_key: row.try_get("cache_key")?,
        provider: row.try_get("provider")?,
        model: row.try_get("model")?,
        input_sha256: row.try_get("input_sha256")?,
        input_kind: row.try_get("input_kind")?,
        dimensions: row.try_get("dimensions")?,
        vector: EmbeddingVector::decode_le(row.try_get::<Vec<u8>, _>("embedding_f32")?.as_slice())?,
        metadata: row.try_get("metadata")?,
    })
}

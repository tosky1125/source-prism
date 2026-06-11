use ri_core::GenerationId;
use ri_symbols::SymbolRecord;
use serde_json::json;
use sqlx::{PgPool, Row as _};
use std::collections::BTreeSet;

use crate::graph_ids::{contains_edge_id, file_node_id, symbol_node_id};

const CREATED_BY: &str = "ri-api-index-v1";
const EDGE_TYPE_CONTAINS: &str = "contains";
const NODE_TYPE_FILE: &str = "file";
const NODE_TYPE_SYMBOL: &str = "symbol";
const RESOLUTION_METHOD: &str = "tree_sitter_contains";

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum GraphStoreError {
    #[error("index generation {generation_id} was not found")]
    GenerationNotFound { generation_id: String },
    #[error("invalid graph range value: {field}={value}")]
    InvalidRangeValue { field: &'static str, value: u32 },
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct GraphIndexOutcome {
    pub nodes: u64,
    pub edges: u64,
}

#[derive(Debug, Clone)]
pub struct PgGraphStore {
    pub(crate) pool: PgPool,
}

impl PgGraphStore {
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn replace_contains_graph(
        &self,
        generation_id: &GenerationId,
        symbols: &[SymbolRecord],
    ) -> Result<GraphIndexOutcome, GraphStoreError> {
        let generation = self.generation(generation_id).await?;
        let mut transaction = self.pool.begin().await?;
        stale_previous_graph(&mut transaction, &generation, generation_id).await?;

        let file_nodes = file_nodes(symbols);
        let mut nodes = 0_u64;
        for file_path in &file_nodes {
            nodes = nodes.saturating_add(
                upsert_file_node(&mut transaction, &generation, generation_id, file_path).await?,
            );
        }

        let mut edges = 0_u64;
        for symbol in symbols {
            nodes = nodes.saturating_add(
                upsert_symbol_node(&mut transaction, &generation, generation_id, symbol).await?,
            );
            edges = edges.saturating_add(
                upsert_contains_edge(&mut transaction, &generation, generation_id, symbol).await?,
            );
        }

        transaction.commit().await?;
        Ok(GraphIndexOutcome { nodes, edges })
    }

    async fn generation(
        &self,
        generation_id: &GenerationId,
    ) -> Result<StoredGeneration, GraphStoreError> {
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
        .ok_or_else(|| GraphStoreError::GenerationNotFound {
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

fn file_nodes(symbols: &[SymbolRecord]) -> BTreeSet<String> {
    symbols
        .iter()
        .map(|symbol| symbol.file_path.to_string())
        .collect()
}

async fn stale_previous_graph(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    generation: &StoredGeneration,
    generation_id: &GenerationId,
) -> Result<(), GraphStoreError> {
    for table in ["graph_edges", "graph_nodes"] {
        sqlx::query(&format!(
            r"
            UPDATE {table}
            SET stale_at = now()
            WHERE repo_id = $1
              AND commit_sha = $2
              AND generation_id <> $3
              AND stale_at IS NULL
            "
        ))
        .bind(&generation.repo_id)
        .bind(&generation.commit_sha)
        .bind(generation_id.to_string())
        .execute(&mut **transaction)
        .await?;
    }
    Ok(())
}

async fn upsert_file_node(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    generation: &StoredGeneration,
    generation_id: &GenerationId,
    file_path: &str,
) -> Result<u64, GraphStoreError> {
    let node_id = file_node_id(&generation.repo_id, &generation.commit_sha, file_path);
    let result = sqlx::query(
        r"
        INSERT INTO graph_nodes (
            graph_node_id, repo_id, commit_sha, generation_id, node_type,
            subject_id, stable_subject_id, display_name, file_path, stale_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $6, $6, $6, NULL)
        ON CONFLICT (graph_node_id) DO UPDATE
        SET generation_id = EXCLUDED.generation_id,
            display_name = EXCLUDED.display_name,
            file_path = EXCLUDED.file_path,
            stale_at = NULL
        ",
    )
    .bind(node_id)
    .bind(&generation.repo_id)
    .bind(&generation.commit_sha)
    .bind(generation_id.to_string())
    .bind(NODE_TYPE_FILE)
    .bind(file_path)
    .execute(&mut **transaction)
    .await?;
    Ok(result.rows_affected())
}

async fn upsert_symbol_node(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    generation: &StoredGeneration,
    generation_id: &GenerationId,
    symbol: &SymbolRecord,
) -> Result<u64, GraphStoreError> {
    let result = sqlx::query(
        r"
        INSERT INTO graph_nodes (
            graph_node_id, repo_id, commit_sha, generation_id, node_type,
            subject_id, stable_subject_id, display_name, file_path, start_line, end_line, stale_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, NULL)
        ON CONFLICT (graph_node_id) DO UPDATE
        SET generation_id = EXCLUDED.generation_id,
            display_name = EXCLUDED.display_name,
            file_path = EXCLUDED.file_path,
            start_line = EXCLUDED.start_line,
            end_line = EXCLUDED.end_line,
            stale_at = NULL
        ",
    )
    .bind(symbol_node_id(symbol))
    .bind(&generation.repo_id)
    .bind(&generation.commit_sha)
    .bind(generation_id.to_string())
    .bind(NODE_TYPE_SYMBOL)
    .bind(symbol.versioned_symbol_id.to_string())
    .bind(symbol.stable_symbol_id.to_string())
    .bind(&symbol.fqn)
    .bind(symbol.file_path.to_string())
    .bind(range_i32(symbol.range.start_line, "start_line")?)
    .bind(range_i32(symbol.range.end_line, "end_line")?)
    .execute(&mut **transaction)
    .await?;
    Ok(result.rows_affected())
}

async fn upsert_contains_edge(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    generation: &StoredGeneration,
    generation_id: &GenerationId,
    symbol: &SymbolRecord,
) -> Result<u64, GraphStoreError> {
    let source_node_id = file_node_id(
        &generation.repo_id,
        &generation.commit_sha,
        symbol.file_path.as_str(),
    );
    let target_node_id = symbol_node_id(symbol);
    let edge_id = contains_edge_id(
        &generation.repo_id,
        &generation.commit_sha,
        &source_node_id,
        &target_node_id,
        EDGE_TYPE_CONTAINS,
    );
    let result = sqlx::query(
        r"
        INSERT INTO graph_edges (
            edge_id, repo_id, commit_sha, generation_id, source_node_id, target_node_id,
            edge_type, confidence, resolution_method,
            evidence_file_path, evidence_start_line, evidence_start_col, evidence_end_line,
            evidence_end_col, evidence, stale_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, 1.0, $8, $9, $10, $11, $12, $13, $14, NULL)
        ON CONFLICT (edge_id) DO UPDATE
        SET generation_id = EXCLUDED.generation_id,
            confidence = EXCLUDED.confidence,
            resolution_method = EXCLUDED.resolution_method,
            evidence = EXCLUDED.evidence,
            stale_at = NULL
        ",
    )
    .bind(edge_id)
    .bind(&generation.repo_id)
    .bind(&generation.commit_sha)
    .bind(generation_id.to_string())
    .bind(source_node_id)
    .bind(target_node_id)
    .bind(EDGE_TYPE_CONTAINS)
    .bind(RESOLUTION_METHOD)
    .bind(symbol.file_path.to_string())
    .bind(range_i32(symbol.range.start_line, "start_line")?)
    .bind(range_i32(symbol.range.start_column, "start_column")?)
    .bind(range_i32(symbol.range.end_line, "end_line")?)
    .bind(range_i32(symbol.range.end_column, "end_column")?)
    .bind(json!({ "created_by": CREATED_BY }))
    .execute(&mut **transaction)
    .await?;
    Ok(result.rows_affected())
}

fn range_i32(value: u32, field: &'static str) -> Result<i32, GraphStoreError> {
    i32::try_from(value).map_err(|_| GraphStoreError::InvalidRangeValue { field, value })
}

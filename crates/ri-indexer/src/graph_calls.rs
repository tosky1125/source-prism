use ri_core::GenerationId;
use ri_symbols::SymbolRange;
use serde_json::json;
use sqlx::Row as _;

use crate::{
    GraphStoreError, PgGraphStore,
    graph_ids::{graph_edge_id, symbol_node_id_from_versioned_id},
};

const CREATED_BY: &str = "ri-api-index-v1";
const EDGE_TYPE_CALLS: &str = "calls";
const RESOLUTION_METHOD: &str = "tree_sitter_call_name";

#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct CallEdgeInput {
    pub source_symbol_id: String,
    pub target_symbol_id: String,
    pub evidence_file_path: String,
    pub range: SymbolRange,
    pub target_name: String,
}

impl CallEdgeInput {
    pub const fn new(
        source_symbol_id: String,
        target_symbol_id: String,
        evidence_file_path: String,
        range: SymbolRange,
        target_name: String,
    ) -> Self {
        Self {
            source_symbol_id,
            target_symbol_id,
            evidence_file_path,
            range,
            target_name,
        }
    }
}

impl PgGraphStore {
    pub async fn replace_call_graph(
        &self,
        generation_id: &GenerationId,
        calls: &[CallEdgeInput],
    ) -> Result<u64, GraphStoreError> {
        let generation = generation(&self.pool, generation_id).await?;
        let mut transaction = self.pool.begin().await?;
        let mut edges = 0_u64;
        for call in calls {
            edges = edges.saturating_add(
                upsert_call_edge(&mut transaction, &generation, generation_id, call).await?,
            );
        }
        transaction.commit().await?;
        Ok(edges)
    }
}

#[derive(Debug)]
struct StoredGeneration {
    repo_id: String,
    commit_sha: String,
}

async fn generation(
    pool: &sqlx::PgPool,
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
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| GraphStoreError::GenerationNotFound {
        generation_id: generation_id.to_string(),
    })?;
    Ok(StoredGeneration {
        repo_id: row.try_get("repo_id")?,
        commit_sha: row.try_get("commit_sha")?,
    })
}

async fn upsert_call_edge(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    generation: &StoredGeneration,
    generation_id: &GenerationId,
    call: &CallEdgeInput,
) -> Result<u64, GraphStoreError> {
    let source_node_id = symbol_node_id_from_versioned_id(&call.source_symbol_id);
    let target_node_id = symbol_node_id_from_versioned_id(&call.target_symbol_id);
    let result = sqlx::query(
        r"
        INSERT INTO graph_edges (
            edge_id, repo_id, commit_sha, generation_id, source_node_id, target_node_id,
            edge_type, confidence, resolution_method,
            evidence_file_path, evidence_start_line, evidence_start_col, evidence_end_line,
            evidence_end_col, evidence, stale_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, 0.700, $8, $9, $10, $11, $12, $13, $14, NULL)
        ON CONFLICT (edge_id) DO UPDATE
        SET generation_id = EXCLUDED.generation_id,
            confidence = EXCLUDED.confidence,
            resolution_method = EXCLUDED.resolution_method,
            evidence = EXCLUDED.evidence,
            stale_at = NULL
        ",
    )
    .bind(graph_edge_id(
        &generation.repo_id,
        &generation.commit_sha,
        &source_node_id,
        &target_node_id,
        EDGE_TYPE_CALLS,
    ))
    .bind(&generation.repo_id)
    .bind(&generation.commit_sha)
    .bind(generation_id.to_string())
    .bind(source_node_id)
    .bind(target_node_id)
    .bind(EDGE_TYPE_CALLS)
    .bind(RESOLUTION_METHOD)
    .bind(&call.evidence_file_path)
    .bind(range_i32(call.range.start_line, "start_line")?)
    .bind(range_i32(call.range.start_column, "start_column")?)
    .bind(range_i32(call.range.end_line, "end_line")?)
    .bind(range_i32(call.range.end_column, "end_column")?)
    .bind(json!({
        "created_by": CREATED_BY,
        "target_name": call.target_name,
        "source_symbol_id": call.source_symbol_id,
        "target_symbol_id": call.target_symbol_id
    }))
    .execute(&mut **transaction)
    .await?;
    Ok(result.rows_affected())
}

fn range_i32(value: u32, field: &'static str) -> Result<i32, GraphStoreError> {
    i32::try_from(value).map_err(|_| GraphStoreError::InvalidRangeValue { field, value })
}

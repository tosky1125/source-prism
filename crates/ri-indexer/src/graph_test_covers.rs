use ri_core::GenerationId;
use serde_json::json;
use sqlx::Row as _;

use crate::{
    GraphStoreError, PgGraphStore,
    graph_ids::{graph_edge_id, symbol_node_id_from_versioned_id},
};

const CREATED_BY: &str = "ri-api-index-v1";
const EDGE_TYPE_TEST_COVERS: &str = "test_covers";
const RESOLUTION_METHOD: &str = "static_test_name_match";

impl PgGraphStore {
    pub async fn replace_test_covers_graph(
        &self,
        generation_id: &GenerationId,
    ) -> Result<u64, GraphStoreError> {
        let generation = generation(&self.pool, generation_id).await?;
        let symbols = active_symbols_for_generation(&self.pool, generation_id).await?;
        let tests = symbols
            .iter()
            .filter(|symbol| symbol.kind == "test_case")
            .collect::<Vec<_>>();
        let targets = symbols
            .iter()
            .filter(|symbol| symbol.kind != "test_case")
            .collect::<Vec<_>>();

        let mut transaction = self.pool.begin().await?;
        let mut edges = 0_u64;
        for test in tests {
            for target in &targets {
                if normalized_contains(&test.fqn, &target.name) {
                    edges = edges.saturating_add(
                        upsert_test_covers_edge(
                            &mut transaction,
                            &generation,
                            generation_id,
                            test,
                            target,
                        )
                        .await?,
                    );
                }
            }
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

async fn active_symbols_for_generation(
    pool: &sqlx::PgPool,
    generation_id: &GenerationId,
) -> Result<Vec<CoverSymbol>, GraphStoreError> {
    let rows = sqlx::query(
        r"
        SELECT symbol_id, stable_symbol_id, commit_sha, file_path, language,
               kind, name, fqn, start_line, start_col, end_line, end_col
        FROM symbols
        WHERE generation_id = $1 AND stale_at IS NULL
        ORDER BY file_path, fqn, start_line
        ",
    )
    .bind(generation_id.to_string())
    .fetch_all(pool)
    .await?;
    rows.iter().map(cover_symbol_from_row).collect()
}

async fn upsert_test_covers_edge(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    generation: &StoredGeneration,
    generation_id: &GenerationId,
    test: &CoverSymbol,
    target: &CoverSymbol,
) -> Result<u64, GraphStoreError> {
    let source_node_id = symbol_node_id_from_versioned_id(&test.symbol_id);
    let target_node_id = symbol_node_id_from_versioned_id(&target.symbol_id);
    let result = sqlx::query(
        r"
        INSERT INTO graph_edges (
            edge_id, repo_id, commit_sha, generation_id, source_node_id, target_node_id,
            edge_type, confidence, resolution_method,
            evidence_file_path, evidence_start_line, evidence_start_col, evidence_end_line,
            evidence_end_col, evidence, stale_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, 0.650, $8, $9, $10, $11, $12, $13, $14, NULL)
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
        EDGE_TYPE_TEST_COVERS,
    ))
    .bind(&generation.repo_id)
    .bind(&generation.commit_sha)
    .bind(generation_id.to_string())
    .bind(source_node_id)
    .bind(target_node_id)
    .bind(EDGE_TYPE_TEST_COVERS)
    .bind(RESOLUTION_METHOD)
    .bind(&test.file_path)
    .bind(test.start_line)
    .bind(test.start_col)
    .bind(test.end_line)
    .bind(test.end_col)
    .bind(json!({
        "created_by": CREATED_BY,
        "target_symbol": target.fqn,
        "evidence": "test name references target symbol"
    }))
    .execute(&mut **transaction)
    .await?;
    Ok(result.rows_affected())
}

#[derive(Debug)]
struct CoverSymbol {
    symbol_id: String,
    file_path: String,
    kind: String,
    name: String,
    fqn: String,
    start_line: i32,
    start_col: i32,
    end_line: i32,
    end_col: i32,
}

fn cover_symbol_from_row(row: &sqlx::postgres::PgRow) -> Result<CoverSymbol, GraphStoreError> {
    Ok(CoverSymbol {
        symbol_id: row.try_get("symbol_id")?,
        file_path: row.try_get("file_path")?,
        kind: row.try_get("kind")?,
        name: row.try_get("name")?,
        fqn: row.try_get("fqn")?,
        start_line: row.try_get("start_line")?,
        start_col: row.try_get("start_col")?,
        end_line: row.try_get("end_line")?,
        end_col: row.try_get("end_col")?,
    })
}

fn normalized_contains(haystack: &str, needle: &str) -> bool {
    let normalized_haystack = normalize_identifier(haystack);
    let normalized_needle = normalize_identifier(needle);
    !normalized_needle.is_empty() && normalized_haystack.contains(normalized_needle.as_str())
}

fn normalize_identifier(value: &str) -> String {
    value
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .flat_map(char::to_lowercase)
        .collect()
}

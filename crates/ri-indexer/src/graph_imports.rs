use ri_core::GenerationId;
use serde_json::json;
use sqlx::Row as _;
use std::collections::BTreeSet;

use crate::{
    GraphStoreError, PgGraphStore,
    graph_ids::{file_node_id, graph_edge_id},
    graph_import_paths::resolve_rust_module_file,
};

const CREATED_BY: &str = "ri-api-index-v1";
const EDGE_TYPE_IMPORTS: &str = "imports";
const NODE_TYPE_FILE: &str = "file";
const RESOLUTION_METHOD: &str = "rust_mod_file";

impl PgGraphStore {
    pub async fn replace_import_graph(
        &self,
        generation_id: &GenerationId,
    ) -> Result<u64, GraphStoreError> {
        let generation = generation(&self.pool, generation_id).await?;
        let files = active_rust_files_for_generation(&self.pool, generation_id).await?;
        let modules = active_module_symbols_for_generation(&self.pool, generation_id).await?;
        let mut transaction = self.pool.begin().await?;
        let mut edges = 0_u64;
        for module in &modules {
            let Some(target_file_path) =
                resolve_rust_module_file(&files, &module.file_path, &module.name)
            else {
                continue;
            };
            upsert_file_node(
                &mut transaction,
                &generation,
                generation_id,
                module.file_path.as_str(),
            )
            .await?;
            upsert_file_node(
                &mut transaction,
                &generation,
                generation_id,
                &target_file_path,
            )
            .await?;
            edges = edges.saturating_add(
                upsert_import_edge(
                    &mut transaction,
                    &generation,
                    generation_id,
                    module,
                    &target_file_path,
                )
                .await?,
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

async fn active_rust_files_for_generation(
    pool: &sqlx::PgPool,
    generation_id: &GenerationId,
) -> Result<BTreeSet<String>, GraphStoreError> {
    let rows = sqlx::query(
        r"
        SELECT file_path
        FROM file_manifests
        WHERE generation_id = $1
          AND language = 'rust'
          AND stale_at IS NULL
        ",
    )
    .bind(generation_id.to_string())
    .fetch_all(pool)
    .await?;
    rows.iter()
        .map(|row| row.try_get("file_path"))
        .collect::<Result<BTreeSet<_>, sqlx::Error>>()
        .map_err(GraphStoreError::from)
}

async fn active_module_symbols_for_generation(
    pool: &sqlx::PgPool,
    generation_id: &GenerationId,
) -> Result<Vec<ModuleSymbol>, GraphStoreError> {
    let rows = sqlx::query(
        r"
        SELECT file_path, name, start_line, start_col, end_line, end_col
        FROM symbols
        WHERE generation_id = $1
          AND language = 'rust'
          AND kind = 'module'
          AND stale_at IS NULL
        ORDER BY file_path, name, start_line
        ",
    )
    .bind(generation_id.to_string())
    .fetch_all(pool)
    .await?;
    rows.iter().map(module_symbol_from_row).collect()
}

async fn upsert_file_node(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    generation: &StoredGeneration,
    generation_id: &GenerationId,
    file_path: &str,
) -> Result<(), GraphStoreError> {
    sqlx::query(
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
    .bind(file_node_id(
        &generation.repo_id,
        &generation.commit_sha,
        file_path,
    ))
    .bind(&generation.repo_id)
    .bind(&generation.commit_sha)
    .bind(generation_id.to_string())
    .bind(NODE_TYPE_FILE)
    .bind(file_path)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

async fn upsert_import_edge(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    generation: &StoredGeneration,
    generation_id: &GenerationId,
    module: &ModuleSymbol,
    target_file_path: &str,
) -> Result<u64, GraphStoreError> {
    let source_node_id = file_node_id(
        &generation.repo_id,
        &generation.commit_sha,
        module.file_path.as_str(),
    );
    let target_node_id = file_node_id(
        &generation.repo_id,
        &generation.commit_sha,
        target_file_path,
    );
    let result = sqlx::query(
        r"
        INSERT INTO graph_edges (
            edge_id, repo_id, commit_sha, generation_id, source_node_id, target_node_id,
            edge_type, confidence, resolution_method,
            evidence_file_path, evidence_start_line, evidence_start_col, evidence_end_line,
            evidence_end_col, evidence, stale_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, 0.850, $8, $9, $10, $11, $12, $13, $14, NULL)
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
        EDGE_TYPE_IMPORTS,
    ))
    .bind(&generation.repo_id)
    .bind(&generation.commit_sha)
    .bind(generation_id.to_string())
    .bind(source_node_id)
    .bind(target_node_id)
    .bind(EDGE_TYPE_IMPORTS)
    .bind(RESOLUTION_METHOD)
    .bind(&module.file_path)
    .bind(module.start_line)
    .bind(module.start_col)
    .bind(module.end_line)
    .bind(module.end_col)
    .bind(json!({
        "created_by": CREATED_BY,
        "import_path": module.name,
        "target_file": target_file_path
    }))
    .execute(&mut **transaction)
    .await?;
    Ok(result.rows_affected())
}

#[derive(Debug)]
struct ModuleSymbol {
    file_path: String,
    name: String,
    start_line: i32,
    start_col: i32,
    end_line: i32,
    end_col: i32,
}

fn module_symbol_from_row(row: &sqlx::postgres::PgRow) -> Result<ModuleSymbol, GraphStoreError> {
    Ok(ModuleSymbol {
        file_path: row.try_get("file_path")?,
        name: row.try_get("name")?,
        start_line: row.try_get("start_line")?,
        start_col: row.try_get("start_col")?,
        end_line: row.try_get("end_line")?,
        end_col: row.try_get("end_col")?,
    })
}

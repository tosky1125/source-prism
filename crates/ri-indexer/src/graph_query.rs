use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::Row as _;

use crate::{GraphStoreError, PgGraphStore};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct GraphProjection {
    pub nodes: Vec<GraphNodeRecord>,
    pub edges: Vec<GraphEdgeRecord>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct GraphNodeRecord {
    pub graph_node_id: String,
    pub node_type: String,
    pub subject_id: Option<String>,
    pub stable_subject_id: Option<String>,
    pub display_name: String,
    pub file_path: Option<String>,
    pub start_line: Option<i32>,
    pub end_line: Option<i32>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct GraphEdgeRecord {
    pub edge_id: String,
    pub source_node_id: String,
    pub target_node_id: String,
    pub edge_type: String,
    pub confidence: f64,
    pub resolution_method: String,
    pub evidence_file_path: Option<String>,
    pub evidence_start_line: Option<i32>,
    pub evidence_start_col: Option<i32>,
    pub evidence_end_line: Option<i32>,
    pub evidence_end_col: Option<i32>,
    pub evidence: Value,
}

impl PgGraphStore {
    pub async fn active_graph_for_repo(
        &self,
        repo_id: &str,
    ) -> Result<GraphProjection, GraphStoreError> {
        let nodes = self.active_nodes_for_repo(repo_id).await?;
        let edges = self.active_edges_for_repo(repo_id).await?;
        Ok(GraphProjection { nodes, edges })
    }

    async fn active_nodes_for_repo(
        &self,
        repo_id: &str,
    ) -> Result<Vec<GraphNodeRecord>, GraphStoreError> {
        let rows = sqlx::query(
            r"
            SELECT graph_node_id, node_type, subject_id, stable_subject_id, display_name,
                   file_path, start_line, end_line
            FROM graph_nodes
            WHERE repo_id = $1
              AND stale_at IS NULL
              AND generation_id = (
                  SELECT generation_id
                  FROM index_generations
                  WHERE repo_id = $1 AND status = 'succeeded'
                  ORDER BY started_at DESC
                  LIMIT 1
              )
            ORDER BY node_type, file_path, display_name, graph_node_id
            ",
        )
        .bind(repo_id)
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(node_from_row).collect()
    }

    async fn active_edges_for_repo(
        &self,
        repo_id: &str,
    ) -> Result<Vec<GraphEdgeRecord>, GraphStoreError> {
        let rows = sqlx::query(
            r"
            SELECT edge_id, source_node_id, target_node_id, edge_type,
                   confidence::float8 AS confidence, resolution_method,
                   evidence_file_path, evidence_start_line, evidence_start_col,
                   evidence_end_line, evidence_end_col, evidence
            FROM graph_edges
            WHERE repo_id = $1
              AND stale_at IS NULL
              AND generation_id = (
                  SELECT generation_id
                  FROM index_generations
                  WHERE repo_id = $1 AND status = 'succeeded'
                  ORDER BY started_at DESC
                  LIMIT 1
              )
            ORDER BY edge_type, evidence_file_path, source_node_id, target_node_id
            ",
        )
        .bind(repo_id)
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(edge_from_row).collect()
    }
}

fn node_from_row(row: &sqlx::postgres::PgRow) -> Result<GraphNodeRecord, GraphStoreError> {
    Ok(GraphNodeRecord {
        graph_node_id: row.try_get("graph_node_id")?,
        node_type: row.try_get("node_type")?,
        subject_id: row.try_get("subject_id")?,
        stable_subject_id: row.try_get("stable_subject_id")?,
        display_name: row.try_get("display_name")?,
        file_path: row.try_get("file_path")?,
        start_line: row.try_get("start_line")?,
        end_line: row.try_get("end_line")?,
    })
}

fn edge_from_row(row: &sqlx::postgres::PgRow) -> Result<GraphEdgeRecord, GraphStoreError> {
    Ok(GraphEdgeRecord {
        edge_id: row.try_get("edge_id")?,
        source_node_id: row.try_get("source_node_id")?,
        target_node_id: row.try_get("target_node_id")?,
        edge_type: row.try_get("edge_type")?,
        confidence: row.try_get("confidence")?,
        resolution_method: row.try_get("resolution_method")?,
        evidence_file_path: row.try_get("evidence_file_path")?,
        evidence_start_line: row.try_get("evidence_start_line")?,
        evidence_start_col: row.try_get("evidence_start_col")?,
        evidence_end_line: row.try_get("evidence_end_line")?,
        evidence_end_col: row.try_get("evidence_end_col")?,
        evidence: row.try_get("evidence")?,
    })
}

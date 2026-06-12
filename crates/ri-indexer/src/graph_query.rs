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

impl GraphProjection {
    pub const fn new(nodes: Vec<GraphNodeRecord>, edges: Vec<GraphEdgeRecord>) -> Self {
        Self { nodes, edges }
    }
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

impl GraphNodeRecord {
    pub fn file(graph_node_id: impl Into<String>, file_path: impl Into<String>) -> Self {
        let file_path = file_path.into();
        Self {
            graph_node_id: graph_node_id.into(),
            node_type: "file".to_owned(),
            subject_id: Some(file_path.clone()),
            stable_subject_id: Some(file_path.clone()),
            display_name: file_path.clone(),
            file_path: Some(file_path),
            start_line: None,
            end_line: None,
        }
    }

    pub fn symbol(
        graph_node_id: impl Into<String>,
        subject_id: impl Into<String>,
        stable_subject_id: impl Into<String>,
        display_name: impl Into<String>,
    ) -> Self {
        Self {
            graph_node_id: graph_node_id.into(),
            node_type: "symbol".to_owned(),
            subject_id: Some(subject_id.into()),
            stable_subject_id: Some(stable_subject_id.into()),
            display_name: display_name.into(),
            file_path: None,
            start_line: None,
            end_line: None,
        }
    }

    #[must_use]
    pub fn with_location(
        mut self,
        file_path: impl Into<String>,
        start_line: Option<i32>,
        end_line: Option<i32>,
    ) -> Self {
        self.file_path = Some(file_path.into());
        self.start_line = start_line;
        self.end_line = end_line;
        self
    }
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

impl GraphEdgeRecord {
    pub fn new(spec: GraphEdgeRecordSpec) -> Self {
        Self {
            edge_id: spec.edge_id,
            source_node_id: spec.source_node_id,
            target_node_id: spec.target_node_id,
            edge_type: spec.edge_type,
            confidence: spec.confidence,
            resolution_method: spec.resolution_method,
            evidence_file_path: spec.evidence_file_path,
            evidence_start_line: spec.evidence_start_line,
            evidence_start_col: spec.evidence_start_col,
            evidence_end_line: spec.evidence_end_line,
            evidence_end_col: spec.evidence_end_col,
            evidence: spec.evidence,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct GraphEdgeRecordSpec {
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

impl GraphEdgeRecordSpec {
    pub fn new(
        edge_id: impl Into<String>,
        source_node_id: impl Into<String>,
        target_node_id: impl Into<String>,
        edge_type: impl Into<String>,
        confidence: f64,
        resolution_method: impl Into<String>,
    ) -> Self {
        Self {
            edge_id: edge_id.into(),
            source_node_id: source_node_id.into(),
            target_node_id: target_node_id.into(),
            edge_type: edge_type.into(),
            confidence,
            resolution_method: resolution_method.into(),
            evidence_file_path: None,
            evidence_start_line: None,
            evidence_start_col: None,
            evidence_end_line: None,
            evidence_end_col: None,
            evidence: Value::Null,
        }
    }

    #[must_use]
    pub fn with_span(
        mut self,
        file_path: impl Into<String>,
        start_line: Option<i32>,
        start_col: Option<i32>,
        end_line: Option<i32>,
        end_col: Option<i32>,
    ) -> Self {
        self.evidence_file_path = Some(file_path.into());
        self.evidence_start_line = start_line;
        self.evidence_start_col = start_col;
        self.evidence_end_line = end_line;
        self.evidence_end_col = end_col;
        self
    }

    #[must_use]
    pub fn with_evidence(mut self, evidence: Value) -> Self {
        self.evidence = evidence;
        self
    }
}

impl PgGraphStore {
    pub async fn active_graph_for_repo(
        &self,
        repo_id: &str,
    ) -> Result<GraphProjection, GraphStoreError> {
        let nodes = self.active_nodes_for_repo(repo_id).await?;
        let edges = self.active_edges_for_repo(repo_id).await?;
        Ok(GraphProjection::new(nodes, edges))
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

use axum::{
    Json,
    extract::{Path, State},
};
use ri_indexer::{GraphProjection, PgGraphStore};
use serde::Serialize;

use crate::{AppError, state::AppState};

#[derive(Debug, Serialize)]
pub(crate) struct RepoGraphResponse {
    status: &'static str,
    kind: &'static str,
    repo_id: String,
    node_count: usize,
    edge_count: usize,
    graph: GraphProjection,
}

pub(crate) async fn get(
    State(state): State<AppState>,
    Path(repo_id): Path<String>,
) -> Result<Json<RepoGraphResponse>, AppError> {
    let pool = state
        .database
        .pool
        .as_ref()
        .ok_or(AppError::DatabaseNotConfigured)?;
    let graph = PgGraphStore::new(pool.clone())
        .active_graph_for_repo(&repo_id)
        .await?;
    Ok(Json(RepoGraphResponse {
        status: "ok",
        kind: "graph",
        repo_id,
        node_count: graph.nodes.len(),
        edge_count: graph.edges.len(),
        graph,
    }))
}

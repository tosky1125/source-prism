use axum::{
    Json,
    extract::{Path, State},
};
use ri_context::ResolvedCallReference;
use ri_indexer::{
    GraphEdgeRecord, GraphEdgeRecordSpec, GraphNodeRecord, GraphProjection, PgGraphStore,
};
use ri_symbols::SymbolRecord;
use serde::Serialize;
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};

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
    let graph = if let Some(pool) = state.database.pool.as_ref() {
        PgGraphStore::new(pool.clone())
            .active_graph_for_repo(&repo_id)
            .await?
    } else {
        graph_from_local_evidence(&state)?
    };
    Ok(Json(RepoGraphResponse {
        status: "ok",
        kind: "graph",
        repo_id,
        node_count: graph.nodes.len(),
        edge_count: graph.edges.len(),
        graph,
    }))
}

fn graph_from_local_evidence(state: &AppState) -> Result<GraphProjection, AppError> {
    let evidence = state.context_index_evidence()?;
    Ok(graph_from_symbols_and_calls(
        evidence.symbols.as_slice(),
        evidence.calls.as_slice(),
    ))
}

fn graph_from_symbols_and_calls(
    symbols: &[SymbolRecord],
    calls: &[ResolvedCallReference],
) -> GraphProjection {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut edge_ids = BTreeSet::new();
    let mut file_paths = BTreeSet::new();
    for symbol in symbols {
        file_paths.insert(symbol.file_path.to_string());
    }
    for file_path in file_paths {
        nodes.push(file_node(file_path.as_str()));
    }
    let symbol_nodes = symbols
        .iter()
        .map(|symbol| (symbol.versioned_symbol_id.to_string(), symbol))
        .collect::<BTreeMap<_, _>>();
    for symbol in symbols {
        nodes.push(symbol_node(symbol));
        push_unique_edge(&mut edges, &mut edge_ids, contains_edge(symbol));
    }
    for call in calls {
        if symbol_nodes.contains_key(call.source_symbol_id.as_str())
            && symbol_nodes.contains_key(call.target_symbol_id.as_str())
        {
            push_unique_edge(&mut edges, &mut edge_ids, call_edge(call));
        }
    }
    GraphProjection::new(nodes, edges)
}

fn push_unique_edge(
    edges: &mut Vec<GraphEdgeRecord>,
    edge_ids: &mut BTreeSet<String>,
    edge: GraphEdgeRecord,
) {
    if edge_ids.insert(edge.edge_id.clone()) {
        edges.push(edge);
    }
}

fn file_node(file_path: &str) -> GraphNodeRecord {
    GraphNodeRecord::file(file_node_id(file_path), file_path)
}

fn symbol_node(symbol: &SymbolRecord) -> GraphNodeRecord {
    GraphNodeRecord::symbol(
        symbol_node_id(symbol.versioned_symbol_id.as_str()),
        symbol.versioned_symbol_id.to_string(),
        symbol.stable_symbol_id.to_string(),
        symbol.fqn.clone(),
    )
    .with_location(
        symbol.file_path.to_string(),
        Some(line_i32(symbol.range.start_line)),
        Some(line_i32(symbol.range.end_line)),
    )
}

fn contains_edge(symbol: &SymbolRecord) -> GraphEdgeRecord {
    let source = file_node_id(symbol.file_path.as_str());
    let target = symbol_node_id(symbol.versioned_symbol_id.as_str());
    GraphEdgeRecord::new(
        GraphEdgeRecordSpec::new(
            edge_id("contains", source.as_str(), target.as_str()),
            source,
            target,
            "contains",
            1.0,
            "local_tree_sitter_contains",
        )
        .with_span(
            symbol.file_path.to_string(),
            Some(line_i32(symbol.range.start_line)),
            Some(line_i32(symbol.range.start_column)),
            Some(line_i32(symbol.range.end_line)),
            Some(line_i32(symbol.range.end_column)),
        )
        .with_evidence(json!({ "created_by": "ri-api-local-graph-v1" })),
    )
}

fn call_edge(call: &ResolvedCallReference) -> GraphEdgeRecord {
    let source = symbol_node_id(call.source_symbol_id.as_str());
    let target = symbol_node_id(call.target_symbol_id.as_str());
    GraphEdgeRecord::new(
        GraphEdgeRecordSpec::new(
            edge_id("calls", source.as_str(), target.as_str()),
            source,
            target,
            "calls",
            0.7,
            "local_tree_sitter_call_name",
        )
        .with_span(
            call.file_path.to_string(),
            Some(line_i32(call.range.start_line)),
            Some(line_i32(call.range.start_column)),
            Some(line_i32(call.range.end_line)),
            Some(line_i32(call.range.end_column)),
        )
        .with_evidence(json!({
            "created_by": "ri-api-local-graph-v1",
            "target_name": call.target_name,
            "source_symbol_id": call.source_symbol_id,
            "target_symbol_id": call.target_symbol_id
        })),
    )
}

fn file_node_id(file_path: &str) -> String {
    format!("local:file:{file_path}")
}

fn symbol_node_id(versioned_symbol_id: &str) -> String {
    format!("local:symbol:{versioned_symbol_id}")
}

fn edge_id(edge_type: &str, source_node_id: &str, target_node_id: &str) -> String {
    format!("local:edge:{edge_type}:{source_node_id}:{target_node_id}")
}

fn line_i32(value: u32) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}

use ri_core::SymbolId;
use ri_impact::ImpactCallEdge;
use ri_indexer::GraphProjection;
use std::collections::BTreeMap;

use crate::AppError;

pub(crate) fn context_call_edges(
    calls: &[ri_context::ResolvedCallReference],
) -> Vec<ImpactCallEdge> {
    calls
        .iter()
        .map(|call| {
            ImpactCallEdge::new(call.source_symbol_id.clone(), call.target_symbol_id.clone())
        })
        .collect()
}

pub(crate) fn graph_call_edges(graph: &GraphProjection) -> Result<Vec<ImpactCallEdge>, AppError> {
    let subject_by_node = graph
        .nodes
        .iter()
        .filter_map(|node| {
            node.subject_id
                .as_ref()
                .map(|subject_id| (node.graph_node_id.as_str(), subject_id.as_str()))
        })
        .collect::<BTreeMap<_, _>>();
    graph
        .edges
        .iter()
        .filter(|edge| edge.edge_type == "calls")
        .filter_map(|edge| {
            let source = subject_by_node.get(edge.source_node_id.as_str())?;
            let target = subject_by_node.get(edge.target_node_id.as_str())?;
            Some(symbol_call_edge(source, target))
        })
        .collect()
}

fn symbol_call_edge(source: &str, target: &str) -> Result<ImpactCallEdge, AppError> {
    let source_symbol_id =
        SymbolId::new(source).map_err(|error| AppError::Validation(error.to_string()))?;
    let target_symbol_id =
        SymbolId::new(target).map_err(|error| AppError::Validation(error.to_string()))?;
    Ok(ImpactCallEdge::new(source_symbol_id, target_symbol_id))
}

use ri_behavior::TestCoverageEdge;
use ri_core::{Confidence, SymbolId};
use ri_indexer::GraphProjection;
use std::collections::BTreeMap;

use crate::AppError;

pub(crate) fn graph_test_coverage_edges(
    graph: &GraphProjection,
) -> Result<Vec<TestCoverageEdge>, AppError> {
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
        .filter(|edge| edge.edge_type == "test_covers")
        .filter_map(|edge| {
            let source = subject_by_node.get(edge.source_node_id.as_str())?;
            let target = subject_by_node.get(edge.target_node_id.as_str())?;
            Some(test_coverage_edge(
                source,
                target,
                edge.confidence,
                edge.resolution_method.as_str(),
            ))
        })
        .collect()
}

fn test_coverage_edge(
    source: &str,
    target: &str,
    confidence: f64,
    resolution_method: &str,
) -> Result<TestCoverageEdge, AppError> {
    let test_symbol_id =
        SymbolId::new(source).map_err(|error| AppError::Validation(error.to_string()))?;
    let target_symbol_id =
        SymbolId::new(target).map_err(|error| AppError::Validation(error.to_string()))?;
    Ok(TestCoverageEdge::new(
        test_symbol_id,
        target_symbol_id,
        confidence_tier(confidence),
        format!("graph edge: test_covers via {resolution_method}"),
    ))
}

fn confidence_tier(confidence: f64) -> Confidence {
    if confidence >= 0.95 {
        Confidence::Exact
    } else if confidence >= 0.80 {
        Confidence::High
    } else if confidence >= 0.50 {
        Confidence::Medium
    } else {
        Confidence::Low
    }
}

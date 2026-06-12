#![allow(
    clippy::redundant_pub_crate,
    reason = "Binary crate helper modules share crate-visible command handlers."
)]

use std::io::{self, Write};

use serde_json::json;

use crate::CliError;

pub(crate) struct IndexResult {
    pub(crate) repo_id: String,
    pub(crate) commit_sha: String,
    pub(crate) generation_id: String,
    pub(crate) inserted_file_manifests: u64,
    pub(crate) indexed_symbols: u64,
    pub(crate) indexed_graph_nodes: u64,
    pub(crate) indexed_graph_edges: u64,
    pub(crate) indexed_import_edges: u64,
    pub(crate) indexed_call_edges: u64,
    pub(crate) indexed_test_cover_edges: u64,
    pub(crate) indexed_search_chunks: u64,
    pub(crate) search_sync_queue: &'static str,
    pub(crate) enqueued_search_sync_jobs: u64,
    pub(crate) indexed_test_cases: u64,
    pub(crate) indexed_architecture_entities: u64,
}

pub(crate) fn print_index_result(result: &IndexResult) -> Result<(), CliError> {
    let stdout = io::stdout();
    let mut lock = stdout.lock();
    serde_json::to_writer_pretty(
        &mut lock,
        &json!({
            "status": "ok",
            "kind": "index",
            "repo_id": result.repo_id,
            "commit_sha": result.commit_sha,
            "generation_id": result.generation_id,
            "inserted_file_manifests": result.inserted_file_manifests,
            "indexed_symbols": result.indexed_symbols,
            "indexed_graph_nodes": result.indexed_graph_nodes,
            "indexed_graph_edges": result.indexed_graph_edges,
            "indexed_import_edges": result.indexed_import_edges,
            "indexed_call_edges": result.indexed_call_edges,
            "indexed_test_cover_edges": result.indexed_test_cover_edges,
            "indexed_search_chunks": result.indexed_search_chunks,
            "search_sync_queue": result.search_sync_queue,
            "enqueued_search_sync_jobs": result.enqueued_search_sync_jobs,
            "indexed_test_cases": result.indexed_test_cases,
            "indexed_architecture_entities": result.indexed_architecture_entities,
        }),
    )?;
    writeln!(lock)?;
    Ok(())
}

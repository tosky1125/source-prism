#![allow(
    clippy::redundant_pub_crate,
    reason = "Binary crate helper modules share crate-visible command handlers."
)]

use std::io::{self, Write};
use std::path::PathBuf;

use ri_context::{ResolvedCallReference, build_context_pack_with_calls};
use ri_impact::ImpactCallEdge;
use serde_json::json;

use crate::error::CliError;

const DEFAULT_LIMIT: usize = 8;

pub(crate) fn search_context_command(
    mut args: impl Iterator<Item = String>,
) -> Result<(), CliError> {
    let Some(query) = args.next() else {
        return Err(CliError::Usage);
    };
    if args.next().is_some() {
        return Err(CliError::Usage);
    }
    let evidence = ri_context::extract_repo_index(&PathBuf::from("."))?;
    let calls = impact_call_edges(&evidence.calls);
    let pack =
        build_context_pack_with_calls(&evidence.symbols, calls.as_slice(), &query, DEFAULT_LIMIT);
    print_json(&json!({
        "status": "ok",
        "kind": "search_context",
        "context_pack": pack,
    }))
}

fn impact_call_edges(calls: &[ResolvedCallReference]) -> Vec<ImpactCallEdge> {
    calls
        .iter()
        .map(|call| {
            ImpactCallEdge::new(call.source_symbol_id.clone(), call.target_symbol_id.clone())
        })
        .collect()
}

fn print_json(value: &serde_json::Value) -> Result<(), CliError> {
    let stdout = io::stdout();
    let mut lock = stdout.lock();
    serde_json::to_writer_pretty(&mut lock, value)?;
    writeln!(lock)?;
    Ok(())
}

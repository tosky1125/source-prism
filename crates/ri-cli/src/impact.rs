#![allow(
    clippy::redundant_pub_crate,
    reason = "Binary crate helper modules share crate-visible command handlers."
)]

use std::io::{self, Write};
use std::path::PathBuf;

use ri_context::ResolvedCallReference;
use ri_impact::{ImpactCallEdge, analyze_symbol_impact_with_calls};
use serde_json::json;

use crate::error::CliError;

pub(crate) fn impact_command(mut args: impl Iterator<Item = String>) -> Result<(), CliError> {
    let Some(flag) = args.next() else {
        return Err(CliError::Usage);
    };
    if flag != "--symbol" {
        return Err(CliError::Usage);
    }
    let Some(symbol_query) = args.next() else {
        return Err(CliError::Usage);
    };
    if args.next().is_some() {
        return Err(CliError::Usage);
    }

    let evidence = ri_context::extract_repo_index(&PathBuf::from("."))?;
    let calls = impact_call_edges(&evidence.calls);
    let report =
        analyze_symbol_impact_with_calls(evidence.symbols, calls.as_slice(), &symbol_query)?;
    print_json(&json!({
        "status": "ok",
        "kind": "impact",
        "symbol": report.symbol,
        "affected_files": report.affected_files,
        "direct_callers": report.direct_callers,
        "direct_callees": report.direct_callees,
        "impact_score": report.impact_score,
        "related": report.related,
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

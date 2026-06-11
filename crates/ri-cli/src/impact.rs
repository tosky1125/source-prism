#![allow(
    clippy::redundant_pub_crate,
    reason = "Binary crate helper modules share crate-visible command handlers."
)]

use std::io::{self, Write};
use std::path::PathBuf;

use ri_impact::analyze_symbol_impact;
use serde_json::json;

use crate::{error::CliError, symbols::extract_repo_symbols};

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

    let symbols = extract_repo_symbols(&PathBuf::from("."))?;
    let report = analyze_symbol_impact(symbols, &symbol_query)?;
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

fn print_json(value: &serde_json::Value) -> Result<(), CliError> {
    let stdout = io::stdout();
    let mut lock = stdout.lock();
    serde_json::to_writer_pretty(&mut lock, value)?;
    writeln!(lock)?;
    Ok(())
}

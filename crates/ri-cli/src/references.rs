#![allow(
    clippy::redundant_pub_crate,
    reason = "Binary crate helper modules share crate-visible command handlers."
)]

use std::io::{self, Write};
use std::path::PathBuf;

use ri_context::find_symbol_references;
use serde_json::json;

use crate::error::CliError;

pub(crate) fn references_command(mut args: impl Iterator<Item = String>) -> Result<(), CliError> {
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
    let report = find_symbol_references(
        evidence.symbols.as_slice(),
        evidence.calls.as_slice(),
        symbol_query.trim(),
    )?;
    print_json(&json!({
        "status": "ok",
        "kind": report.kind,
        "symbol": report.symbol,
        "incoming_count": report.incoming_count,
        "outgoing_count": report.outgoing_count,
        "references": report.references,
    }))
}

fn print_json(value: &serde_json::Value) -> Result<(), CliError> {
    let stdout = io::stdout();
    let mut lock = stdout.lock();
    serde_json::to_writer_pretty(&mut lock, value)?;
    writeln!(lock)?;
    Ok(())
}

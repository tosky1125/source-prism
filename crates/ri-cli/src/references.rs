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
    let request = ReferenceArgs::parse(&mut args)?;
    let evidence = ri_context::extract_repo_index(&request.repo)?;
    let report = find_symbol_references(
        evidence.symbols.as_slice(),
        evidence.calls.as_slice(),
        request.symbol.trim(),
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

#[derive(Debug)]
struct ReferenceArgs {
    repo: PathBuf,
    symbol: String,
}

impl ReferenceArgs {
    fn parse(args: &mut impl Iterator<Item = String>) -> Result<Self, CliError> {
        let mut repo = PathBuf::from(".");
        let mut symbol = None::<String>;

        while let Some(flag) = args.next() {
            match flag.as_str() {
                "--repo" => {
                    let Some(path) = args.next() else {
                        return Err(CliError::Usage);
                    };
                    repo = PathBuf::from(path);
                }
                "--symbol" => {
                    symbol = args.next();
                }
                _ => return Err(CliError::Usage),
            }
        }

        Ok(Self {
            repo,
            symbol: symbol.ok_or(CliError::Usage)?,
        })
    }
}

fn print_json(value: &serde_json::Value) -> Result<(), CliError> {
    let stdout = io::stdout();
    let mut lock = stdout.lock();
    serde_json::to_writer_pretty(&mut lock, value)?;
    writeln!(lock)?;
    Ok(())
}

#![allow(
    clippy::redundant_pub_crate,
    reason = "Binary crate helper modules share crate-visible command handlers."
)]

use std::io::{self, Write};
use std::path::PathBuf;

use ri_context::build_context_pack;
use serde_json::json;

use crate::{error::CliError, symbols::extract_repo_symbols};

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
    let symbols = extract_repo_symbols(&PathBuf::from("."))?;
    let pack = build_context_pack(&symbols, &query, DEFAULT_LIMIT);
    print_json(&json!({
        "status": "ok",
        "kind": "search_context",
        "context_pack": pack,
    }))
}

fn print_json(value: &serde_json::Value) -> Result<(), CliError> {
    let stdout = io::stdout();
    let mut lock = stdout.lock();
    serde_json::to_writer_pretty(&mut lock, value)?;
    writeln!(lock)?;
    Ok(())
}

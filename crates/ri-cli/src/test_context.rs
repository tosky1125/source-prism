#![allow(
    clippy::redundant_pub_crate,
    reason = "Binary crate helper modules share crate-visible command handlers."
)]

use std::{
    io::{self, Write},
    path::PathBuf,
};

use ri_behavior::build_test_context;
use serde_json::json;

use crate::{error::CliError, symbols::extract_repo_symbols};

pub(crate) fn test_context_command(mut args: impl Iterator<Item = String>) -> Result<(), CliError> {
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
    let test_context = build_test_context(&symbols, &symbol_query)?;
    print_json(&json!({
        "status": "ok",
        "kind": "test_context",
        "test_context": test_context,
    }))
}

fn print_json(value: &serde_json::Value) -> Result<(), CliError> {
    let stdout = io::stdout();
    let mut lock = stdout.lock();
    serde_json::to_writer_pretty(&mut lock, value)?;
    writeln!(lock)?;
    Ok(())
}

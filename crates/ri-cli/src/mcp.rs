#![allow(
    clippy::redundant_pub_crate,
    reason = "Binary crate helper modules share crate-visible command handlers."
)]

use std::io::{self, Write};

use ri_mcp::McpToolCatalog;
use serde_json::json;

use crate::error::CliError;

pub(crate) fn command(mut args: impl Iterator<Item = String>) -> Result<(), CliError> {
    let Some(subcommand) = args.next() else {
        return Err(CliError::Usage);
    };
    if subcommand != "tools" || args.next().is_some() {
        return Err(CliError::Usage);
    }

    print_json(&json!({
        "kind": "mcp_tool_catalog",
        "tools": McpToolCatalog::new().tools()
    }))
}

fn print_json(value: &serde_json::Value) -> Result<(), CliError> {
    let stdout = io::stdout();
    let mut lock = stdout.lock();
    serde_json::to_writer_pretty(&mut lock, value)?;
    writeln!(lock)?;
    Ok(())
}

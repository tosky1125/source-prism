#![allow(
    missing_docs,
    reason = "Example binary is exercised as a JSON smoke surface."
)]

use ri_mcp::McpToolCatalog;
use serde_json::json;
use std::io::Write as _;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = json!({
        "kind": "mcp_tool_catalog",
        "tools": McpToolCatalog::new().tools()
    });
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    serde_json::to_writer_pretty(&mut handle, &output)?;
    handle.write_all(b"\n")?;
    Ok(())
}

#![allow(missing_docs, reason = "CLI integration test names document behavior.")]

use std::process::Command;

use serde_json::Value;

#[test]
fn mcp_tools_command_returns_repo_tool_catalog() -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new(env!("CARGO_BIN_EXE_ri-cli"))
        .args(["mcp", "tools"])
        .output()?;

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let body = serde_json::from_slice::<Value>(&output.stdout)?;
    assert_eq!(
        body.pointer("/kind").and_then(Value::as_str),
        Some("mcp_tool_catalog")
    );
    let names = body
        .pointer("/tools")
        .and_then(Value::as_array)
        .ok_or_else(|| std::io::Error::other("missing tools"))?
        .iter()
        .filter_map(|tool| tool.get("name").and_then(Value::as_str))
        .collect::<Vec<_>>();
    assert_eq!(
        names,
        [
            "repo.get_symbol",
            "repo.find_references",
            "repo.get_impact",
            "repo.search_context"
        ]
    );
    Ok(())
}

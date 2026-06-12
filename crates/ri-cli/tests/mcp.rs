#![allow(missing_docs, reason = "CLI integration test names document behavior.")]

use std::{path::Path, process::Command};

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

#[test]
fn mcp_call_get_symbol_returns_symbol_result() -> Result<(), Box<dyn std::error::Error>> {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");

    let output = Command::new(env!("CARGO_BIN_EXE_ri-cli"))
        .current_dir(repo_root)
        .args([
            "mcp",
            "call",
            "--repo",
            ".",
            "--tool",
            "repo.get_symbol",
            "--symbol",
            "mcp_tools_command_returns_repo_tool_catalog",
        ])
        .output()?;

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let body = serde_json::from_slice::<Value>(&output.stdout)?;
    assert_eq!(
        body.pointer("/kind").and_then(Value::as_str),
        Some("mcp_tool_result")
    );
    assert_eq!(
        body.pointer("/tool").and_then(Value::as_str),
        Some("repo.get_symbol")
    );
    assert_eq!(
        body.pointer("/result/fqn").and_then(Value::as_str),
        Some("mcp_tools_command_returns_repo_tool_catalog")
    );
    Ok(())
}

#[test]
fn mcp_call_search_context_returns_non_vector_context() -> Result<(), Box<dyn std::error::Error>> {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");

    let output = Command::new(env!("CARGO_BIN_EXE_ri-cli"))
        .current_dir(repo_root)
        .args([
            "mcp",
            "call",
            "--repo",
            ".",
            "--tool",
            "repo.search_context",
            "--query",
            "mcp_tools_command_returns_repo_tool_catalog",
            "--limit",
            "2",
        ])
        .output()?;

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let body = serde_json::from_slice::<Value>(&output.stdout)?;
    assert_eq!(
        body.pointer("/tool").and_then(Value::as_str),
        Some("repo.search_context")
    );
    assert_eq!(
        body.pointer("/result/vector_only").and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        body.pointer("/result/hits/0/symbol/fqn")
            .and_then(Value::as_str),
        Some("mcp_tools_command_returns_repo_tool_catalog")
    );
    Ok(())
}

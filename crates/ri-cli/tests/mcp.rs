#![allow(missing_docs, reason = "CLI integration test names document behavior.")]

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

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

#[test]
fn mcp_serve_once_handles_tool_call_request() -> Result<(), Box<dyn std::error::Error>> {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let request = TempJson::write(
        r#"{
          "jsonrpc": "2.0",
          "id": 7,
          "method": "tools/call",
          "params": {
            "name": "repo.get_symbol",
            "arguments": {
              "symbol": "mcp_tools_command_returns_repo_tool_catalog"
            }
          }
        }"#,
    )?;

    let output = Command::new(env!("CARGO_BIN_EXE_ri-cli"))
        .current_dir(repo_root)
        .args(["mcp", "serve", "--repo", ".", "--once", "--request"])
        .arg(request.path())
        .output()?;

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let body = serde_json::from_slice::<Value>(&output.stdout)?;
    assert_eq!(
        body.pointer("/jsonrpc").and_then(Value::as_str),
        Some("2.0")
    );
    assert_eq!(body.pointer("/id").and_then(Value::as_u64), Some(7));
    assert_eq!(
        body.pointer("/result/isError").and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        body.pointer("/result/structuredContent/fqn")
            .and_then(Value::as_str),
        Some("mcp_tools_command_returns_repo_tool_catalog")
    );
    request.cleanup()?;
    Ok(())
}

struct TempJson {
    path: PathBuf,
}

impl TempJson {
    fn write(body: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let suffix = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let path = std::env::temp_dir().join(format!("source-prism-mcp-{suffix}.json"));
        fs::write(&path, body)?;
        Ok(Self { path })
    }

    fn path(&self) -> &Path {
        self.path.as_path()
    }

    fn cleanup(&self) -> Result<(), std::io::Error> {
        fs::remove_file(&self.path)
    }
}

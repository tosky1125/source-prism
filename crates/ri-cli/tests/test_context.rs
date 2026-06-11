#![allow(missing_docs, reason = "CLI integration test names document behavior.")]

use std::{path::Path, process::Command};

use serde_json::Value;

#[test]
fn test_context_command_returns_static_test_evidence() -> Result<(), Box<dyn std::error::Error>> {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");

    let output = Command::new(env!("CARGO_BIN_EXE_ri-cli"))
        .current_dir(repo_root)
        .args([
            "test-context",
            "--symbol",
            "extracts_rust_functions_methods_and_tests",
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
        Some("test_context")
    );
    assert_eq!(
        body.pointer("/test_context/code_execution_allowed")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert!(
        body.pointer("/test_context/related_tests")
            .and_then(Value::as_array)
            .is_some_and(|tests| !tests.is_empty())
    );
    Ok(())
}

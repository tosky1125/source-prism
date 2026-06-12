#![allow(missing_docs, reason = "CLI integration test names document behavior.")]

use std::{path::Path, process::Command};

use serde_json::Value;

#[test]
fn refactor_plan_command_returns_planner_only_json() -> Result<(), Box<dyn std::error::Error>> {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");

    let output = Command::new(env!("CARGO_BIN_EXE_ri-cli"))
        .current_dir(repo_root)
        .args(["refactor", "plan", "--symbol", "search"])
        .output()?;

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let body = serde_json::from_slice::<Value>(&output.stdout)?;
    assert_eq!(
        body.pointer("/kind").and_then(Value::as_str),
        Some("refactor_plan")
    );
    assert_eq!(
        body.pointer("/plan/execution_allowed")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert!(
        body.pointer("/plan/required_gates")
            .and_then(Value::as_array)
            .is_some_and(|gates| !gates.is_empty())
    );
    Ok(())
}

#![allow(missing_docs, reason = "CLI integration test names document behavior.")]

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use serde_json::Value;

#[test]
fn architecture_command_returns_repo_contracts() -> Result<(), Box<dyn std::error::Error>> {
    let repo = TempRepo::create()?;
    repo.write_file(".github/CODEOWNERS", "* @platform\n")?;
    repo.write_file("docs/adr/0001.md", "# Record\n")?;
    repo.write_file("openapi.yaml", "openapi: 3.1.0\n")?;
    repo.commit()?;

    let output = Command::new(env!("CARGO_BIN_EXE_ri-cli"))
        .current_dir(repo.path())
        .args(["architecture", "--repo", "."])
        .output()?;

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let body = serde_json::from_slice::<Value>(&output.stdout)?;
    assert_eq!(
        body.pointer("/kind").and_then(Value::as_str),
        Some("architecture")
    );
    assert_eq!(
        body.pointer("/entity_count").and_then(Value::as_u64),
        Some(3)
    );
    assert_json_array_contains(&body, "/entities", "entity_type", "codeowners")?;
    assert_json_array_contains(&body, "/entities", "entity_type", "adr")?;
    assert_json_array_contains(&body, "/entities", "entity_type", "openapi")?;
    repo.cleanup()?;
    Ok(())
}

#[test]
fn impact_command_accepts_invoice_service_smoke_symbol() -> Result<(), Box<dyn std::error::Error>> {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");

    let output = Command::new(env!("CARGO_BIN_EXE_ri-cli"))
        .current_dir(repo_root)
        .args(["impact", "--symbol", "InvoiceService::applyTax"])
        .output()?;

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let body = serde_json::from_slice::<Value>(&output.stdout)?;
    assert_eq!(
        body.pointer("/kind").and_then(Value::as_str),
        Some("impact")
    );
    assert_eq!(body.pointer("/status").and_then(Value::as_str), Some("ok"));
    assert_eq!(
        body.pointer("/symbol/fqn").and_then(Value::as_str),
        Some("InvoiceService::applyTax")
    );
    Ok(())
}

#[test]
fn impact_command_uses_repo_path_argument() -> Result<(), Box<dyn std::error::Error>> {
    let repo = TempRepo::create()?;
    repo.write_file(
        "src/lib.rs",
        r"
pub fn apply_tax(value: i32) -> i32 {
    value + 1
}
",
    )?;
    repo.commit()?;
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");

    let output = Command::new(env!("CARGO_BIN_EXE_ri-cli"))
        .current_dir(repo_root)
        .args(["impact", "--repo"])
        .arg(repo.path())
        .args(["--symbol", "apply_tax"])
        .output()?;

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let body = serde_json::from_slice::<Value>(&output.stdout)?;
    assert_eq!(
        body.pointer("/symbol/fqn").and_then(Value::as_str),
        Some("apply_tax")
    );
    repo.cleanup()?;
    Ok(())
}

#[test]
fn impact_command_rejects_unknown_smoke_symbol() -> Result<(), Box<dyn std::error::Error>> {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");

    let output = Command::new(env!("CARGO_BIN_EXE_ri-cli"))
        .current_dir(repo_root)
        .args(["impact", "--symbol", "DefinitelyMissing::symbol"])
        .output()?;

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("symbol not found"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

struct TempRepo {
    path: PathBuf,
}

impl TempRepo {
    fn create() -> Result<Self, Box<dyn std::error::Error>> {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_nanos()
            .to_string();
        let path = std::env::temp_dir().join(format!("source-prism-cli-architecture-{suffix}"));
        fs::create_dir_all(path.join(".github"))?;
        fs::create_dir_all(path.join("docs/adr"))?;
        fs::create_dir_all(path.join("src"))?;
        run_git(&path, ["init"])?;
        run_git(
            &path,
            ["config", "user.email", "source-prism@example.invalid"],
        )?;
        run_git(&path, ["config", "user.name", "Source Prism Test"])?;
        Ok(Self { path })
    }

    fn path(&self) -> &Path {
        self.path.as_path()
    }

    fn write_file(&self, path: &str, body: &str) -> Result<(), std::io::Error> {
        fs::write(self.path.join(path), body)
    }

    fn commit(&self) -> Result<(), Box<dyn std::error::Error>> {
        run_git(&self.path, ["add", "."])?;
        run_git(&self.path, ["commit", "-m", "fixture"])?;
        Ok(())
    }

    fn cleanup(&self) -> Result<(), std::io::Error> {
        fs::remove_dir_all(&self.path)
    }
}

fn run_git<const N: usize>(path: &Path, args: [&str; N]) -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new("git").current_dir(path).args(args).output()?;
    if output.status.success() {
        return Ok(());
    }
    Err(std::io::Error::other(String::from_utf8_lossy(&output.stderr).to_string()).into())
}

fn assert_json_array_contains(
    body: &Value,
    pointer: &str,
    field: &str,
    expected: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let values = body
        .pointer(pointer)
        .and_then(Value::as_array)
        .ok_or_else(|| std::io::Error::other(format!("missing array {pointer}")))?;
    assert!(
        values
            .iter()
            .any(|value| value.get(field).and_then(Value::as_str) == Some(expected)),
        "{pointer} should contain {field}={expected}"
    );
    Ok(())
}

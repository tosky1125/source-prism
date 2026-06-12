#![allow(missing_docs, reason = "CLI integration test names document behavior.")]

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use serde_json::Value;

#[test]
fn search_context_command_uses_repo_path_argument() -> Result<(), Box<dyn std::error::Error>> {
    let repo = TempRepo::create()?;
    repo.write_file(
        "src/lib.rs",
        r"
pub fn apply_tax(value: i32) -> i32 {
    value + 1
}

#[test]
fn apply_tax_adds_rate() {
    assert_eq!(apply_tax(1), 2);
}
",
    )?;
    repo.commit()?;
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");

    let output = Command::new(env!("CARGO_BIN_EXE_ri-cli"))
        .current_dir(repo_root)
        .args(["search-context", "--repo"])
        .arg(repo.path())
        .arg("apply_tax")
        .output()?;

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let body = serde_json::from_slice::<Value>(&output.stdout)?;
    assert_eq!(
        body.pointer("/kind").and_then(Value::as_str),
        Some("search_context")
    );
    assert_json_array_contains(
        &body,
        "/context_pack/impacts/0/direct_callers",
        "apply_tax_adds_rate",
    )?;
    repo.cleanup()?;
    Ok(())
}

struct TempRepo {
    path: PathBuf,
}

impl TempRepo {
    fn create() -> Result<Self, Box<dyn std::error::Error>> {
        let suffix = unique_suffix()?;
        let path = std::env::temp_dir().join(format!("source-prism-search-context-{suffix}"));
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

fn unique_suffix() -> Result<String, std::time::SystemTimeError> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_nanos()
        .to_string())
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
    expected: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let values = body
        .pointer(pointer)
        .and_then(Value::as_array)
        .ok_or_else(|| std::io::Error::other(format!("missing array {pointer}")))?;
    assert!(
        values.iter().any(|value| value.as_str() == Some(expected)),
        "{pointer} should contain {expected}"
    );
    Ok(())
}

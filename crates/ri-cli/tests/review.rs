#![allow(missing_docs, reason = "CLI integration test names document behavior.")]

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use serde_json::Value;

#[test]
fn review_verify_command_accepts_evidence_backed_findings() -> Result<(), Box<dyn std::error::Error>>
{
    let fixture = TempJson::write(valid_findings())?;

    let output = Command::new(env!("CARGO_BIN_EXE_ri-cli"))
        .args(["review", "verify", "--input"])
        .arg(fixture.path())
        .output()?;

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let body = serde_json::from_slice::<Value>(&output.stdout)?;
    assert_eq!(
        body.pointer("/kind").and_then(Value::as_str),
        Some("review_verification")
    );
    assert_eq!(
        body.pointer("/verified_count").and_then(Value::as_u64),
        Some(1)
    );
    fixture.cleanup()?;
    Ok(())
}

#[test]
fn review_verify_command_rejects_finding_without_evidence() -> Result<(), Box<dyn std::error::Error>>
{
    let fixture = TempJson::write(missing_evidence_findings())?;

    let output = Command::new(env!("CARGO_BIN_EXE_ri-cli"))
        .args(["review", "verify", "--input"])
        .arg(fixture.path())
        .output()?;

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("evidence"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    fixture.cleanup()?;
    Ok(())
}

#[test]
fn review_github_dry_run_returns_annotations_and_sarif() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = TempJson::write(valid_findings())?;

    let output = Command::new(env!("CARGO_BIN_EXE_ri-cli"))
        .args(["review", "github-dry-run", "--input"])
        .arg(fixture.path())
        .output()?;

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let body = serde_json::from_slice::<Value>(&output.stdout)?;
    assert_eq!(
        body.pointer("/kind").and_then(Value::as_str),
        Some("github_review_dry_run")
    );
    assert_eq!(
        body.pointer("/annotations/0/path").and_then(Value::as_str),
        Some("src/invoice.rs")
    );
    assert_eq!(
        body.pointer("/sarif/version").and_then(Value::as_str),
        Some("2.1.0")
    );
    assert_eq!(
        body.pointer("/sarif/runs/0/results/0/locations/0/physicalLocation/artifactLocation/uri")
            .and_then(Value::as_str),
        Some("src/invoice.rs")
    );
    fixture.cleanup()?;
    Ok(())
}

struct TempJson {
    path: PathBuf,
}

impl TempJson {
    fn write(body: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let suffix = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let path = std::env::temp_dir().join(format!("source-prism-review-{suffix}.json"));
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

const fn valid_findings() -> &'static str {
    r#"[
      {
        "title": "Tax rounding can skip fractional cents",
        "severity": "medium",
        "file_path": "src/invoice.rs",
        "start_line": 12,
        "end_line": 16,
        "evidence": [
          {
            "file_path": "src/invoice.rs",
            "start_line": 12,
            "end_line": 16,
            "summary": "rounding happens before line item aggregation"
          }
        ],
        "impact_path": [
          {
            "source": "InvoiceService::applyTax",
            "relation": "calls",
            "target": "Money::round"
          }
        ],
        "recommendation": "Round only after summing line item tax amounts."
      }
    ]"#
}

const fn missing_evidence_findings() -> &'static str {
    r#"[
      {
        "title": "Tax rounding can skip fractional cents",
        "severity": "medium",
        "file_path": "src/invoice.rs",
        "start_line": 12,
        "end_line": 16,
        "evidence": [],
        "impact_path": [
          {
            "source": "InvoiceService::applyTax",
            "relation": "calls",
            "target": "Money::round"
          }
        ],
        "recommendation": "Round only after summing line item tax amounts."
      }
    ]"#
}

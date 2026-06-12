#![allow(missing_docs, reason = "Integration test names document behavior.")]

use ri_github::{GitHubAnnotationLevel, build_review_dry_run};
use ri_review::{ProposedFinding, verify_findings};

#[test]
fn dry_run_maps_verified_findings_to_annotations_and_sarif()
-> Result<(), Box<dyn std::error::Error>> {
    let findings = serde_json::from_str::<Vec<ProposedFinding>>(VALID_FINDINGS)?;
    let verified = verify_findings(findings.as_slice())?;

    let dry_run = build_review_dry_run(verified.as_slice());

    let annotation = dry_run
        .annotations
        .first()
        .ok_or_else(|| std::io::Error::other("missing annotation"))?;
    assert_eq!(dry_run.annotations.len(), 1);
    assert_eq!(annotation.path, "src/invoice.rs");
    assert_eq!(annotation.annotation_level, GitHubAnnotationLevel::Warning);
    assert_eq!(dry_run.sarif.version, "2.1.0");
    let sarif_run = dry_run
        .sarif
        .runs
        .first()
        .ok_or_else(|| std::io::Error::other("missing sarif run"))?;
    let sarif_result = sarif_run
        .results
        .first()
        .ok_or_else(|| std::io::Error::other("missing sarif result"))?;
    let sarif_location = sarif_result
        .locations
        .first()
        .ok_or_else(|| std::io::Error::other("missing sarif location"))?;
    assert_eq!(
        sarif_location.physical_location.artifact_location.uri,
        "src/invoice.rs"
    );
    Ok(())
}

#[test]
fn dry_run_redacts_secret_like_review_text() -> Result<(), Box<dyn std::error::Error>> {
    let findings = serde_json::from_str::<Vec<ProposedFinding>>(SECRET_FINDINGS)?;
    let verified = verify_findings(findings.as_slice())?;

    let dry_run = build_review_dry_run(verified.as_slice());

    let serialized = serde_json::to_string(&dry_run)?;
    assert!(!serialized.contains("ghp_live_secret"));
    assert!(!serialized.contains("hunter2"));
    assert!(serialized.contains("[redacted]"));
    Ok(())
}

const VALID_FINDINGS: &str = r#"[
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
]"#;

const SECRET_FINDINGS: &str = r#"[
  {
    "title": "Do not print token=ghp_live_secret",
    "severity": "high",
    "file_path": "src/invoice.rs",
    "start_line": 12,
    "end_line": 16,
    "evidence": [
      {
        "file_path": "src/invoice.rs",
        "start_line": 12,
        "end_line": 16,
        "summary": "log output includes password=hunter2"
      }
    ],
    "impact_path": [
      {
        "source": "InvoiceService::applyTax",
        "relation": "calls",
        "target": "Logger::info"
      }
    ],
    "recommendation": "Remove Authorization: Bearer ghp_live_secret before publishing."
  }
]"#;

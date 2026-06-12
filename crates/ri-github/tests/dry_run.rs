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

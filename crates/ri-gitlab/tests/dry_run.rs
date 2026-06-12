#![allow(missing_docs, reason = "Integration test names document behavior.")]

use ri_gitlab::{GitLabCodeQualitySeverity, build_review_dry_run};
use ri_review::{ProposedFinding, verify_findings};

#[test]
fn dry_run_maps_verified_findings_to_discussions_and_code_quality()
-> Result<(), Box<dyn std::error::Error>> {
    let findings = serde_json::from_str::<Vec<ProposedFinding>>(VALID_FINDINGS)?;
    let verified = verify_findings(findings.as_slice())?;

    let dry_run = build_review_dry_run(verified.as_slice());

    let discussion = dry_run
        .discussions
        .first()
        .ok_or_else(|| std::io::Error::other("missing discussion"))?;
    assert_eq!(discussion.position.new_path, "src/invoice.rs");
    assert_eq!(discussion.position.new_line, 12);

    let code_quality = dry_run
        .code_quality
        .first()
        .ok_or_else(|| std::io::Error::other("missing code quality finding"))?;
    assert_eq!(code_quality.location.path, "src/invoice.rs");
    assert_eq!(code_quality.severity, GitLabCodeQualitySeverity::Major);
    assert!(code_quality.fingerprint.starts_with("source-prism:"));
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

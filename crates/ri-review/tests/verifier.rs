#![allow(missing_docs, reason = "Integration test names document behavior.")]

use ri_review::{ProposedFinding, verify_finding};

#[test]
fn verifier_accepts_finding_with_location_evidence_impact_and_recommendation()
-> Result<(), Box<dyn std::error::Error>> {
    let finding = complete_finding()?;

    let verified = verify_finding(&finding)?;

    assert_eq!(verified.file_path.as_str(), "src/invoice.rs");
    assert_eq!(verified.start_line, 12);
    assert_eq!(verified.evidence.len(), 1);
    assert_eq!(verified.impact_path.len(), 1);
    Ok(())
}

#[test]
fn verifier_rejects_finding_without_impact_path() -> Result<(), Box<dyn std::error::Error>> {
    let mut finding = complete_finding()?;
    finding.impact_path.clear();

    let Err(error) = verify_finding(&finding) else {
        return Err(std::io::Error::other("missing impact path must be rejected").into());
    };

    assert!(error.to_string().contains("impact_path"));
    Ok(())
}

#[test]
fn verifier_rejects_finding_without_actionable_recommendation()
-> Result<(), Box<dyn std::error::Error>> {
    let mut finding = complete_finding()?;
    finding.recommendation = "   ".to_owned();

    let Err(error) = verify_finding(&finding) else {
        return Err(std::io::Error::other("empty recommendation must be rejected").into());
    };

    assert!(error.to_string().contains("recommendation"));
    Ok(())
}

fn complete_finding() -> Result<ProposedFinding, serde_json::Error> {
    serde_json::from_str(
        r#"{
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
        }"#,
    )
}

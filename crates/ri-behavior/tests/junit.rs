#![allow(missing_docs, reason = "Integration test names document behavior.")]

use ri_behavior::{TestResultStatus, parse_junit_xml};

#[test]
fn parses_junit_testsuite_results() -> Result<(), Box<dyn std::error::Error>> {
    let xml = r#"
<testsuite name="invoice" tests="3" failures="1" errors="0" skipped="1" time="0.123">
  <testcase classname="invoice.ApplyTax" name="adds_rate" file="tests/invoice.rs" time="0.010"/>
  <testcase classname="invoice.ApplyTax" name="rounds" file="tests/invoice.rs" time="0.020">
    <failure message="expected 11">diff</failure>
  </testcase>
  <testcase classname="invoice.ApplyTax" name="disabled" time="0.000">
    <skipped/>
  </testcase>
</testsuite>
"#;

    let report = parse_junit_xml(xml)?;

    assert_eq!(report.suites.len(), 1);
    assert_eq!(report.total_count(), 3);
    assert_eq!(report.failed_count(), 1);
    assert_eq!(report.skipped_count(), 1);
    let failed = report
        .results()
        .find(|result| result.name == "rounds")
        .ok_or_else(|| std::io::Error::other("missing failed test"))?;
    assert_eq!(failed.status, TestResultStatus::Failed);
    assert_eq!(failed.message.as_deref(), Some("expected 11"));
    Ok(())
}

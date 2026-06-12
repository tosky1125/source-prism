#![allow(missing_docs, reason = "Pytest parser test names document behavior.")]

use ri_behavior::{TestResultStatus, parse_pytest_json};

#[test]
fn parses_pytest_json_test_results() -> Result<(), Box<dyn std::error::Error>> {
    let report = parse_pytest_json(
        r#"
        {
          "tests": [
            {
              "nodeid": "tests/test_invoice.py::TestInvoice::test_adds_rate",
              "outcome": "passed",
              "duration": 0.012
            },
            {
              "nodeid": "tests/test_invoice.py::test_rejects_negative",
              "outcome": "failed",
              "duration": 0.002,
              "call": {"crash": {"message": "assert False"}}
            }
          ]
        }
        "#,
    )?;

    assert_eq!(report.total_count(), 2);
    assert_eq!(report.passed_count(), 1);
    assert_eq!(report.failed_count(), 1);
    let results = report.results().collect::<Vec<_>>();
    let first = results
        .first()
        .ok_or_else(|| std::io::Error::other("missing first pytest result"))?;
    let second = results
        .get(1)
        .ok_or_else(|| std::io::Error::other("missing second pytest result"))?;
    assert_eq!(first.suite_name, "pytest");
    assert_eq!(first.file_path.as_deref(), Some("tests/test_invoice.py"));
    assert_eq!(first.class_name.as_deref(), Some("TestInvoice"));
    assert_eq!(first.name, "test_adds_rate");
    assert_eq!(first.status, TestResultStatus::Passed);
    assert_eq!(second.status, TestResultStatus::Failed);
    assert_eq!(second.message.as_deref(), Some("assert False"));
    Ok(())
}

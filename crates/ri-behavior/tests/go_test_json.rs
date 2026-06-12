#![allow(missing_docs, reason = "Go test parser test names document behavior.")]

use ri_behavior::{TestResultStatus, parse_go_test_json};

#[test]
fn parses_go_test_json_line_results() -> Result<(), Box<dyn std::error::Error>> {
    let report = parse_go_test_json(
        r#"
{"Time":"2026-06-12T03:00:00Z","Action":"run","Package":"example.com/invoice","Test":"TestAddsRate"}
{"Time":"2026-06-12T03:00:00Z","Action":"output","Package":"example.com/invoice","Test":"TestRejectsNegative","Output":"expected error\n"}
{"Time":"2026-06-12T03:00:00Z","Action":"pass","Package":"example.com/invoice","Test":"TestAddsRate","Elapsed":0.012}
{"Time":"2026-06-12T03:00:00Z","Action":"fail","Package":"example.com/invoice","Test":"TestRejectsNegative","Elapsed":0.003}
{"Time":"2026-06-12T03:00:00Z","Action":"skip","Package":"example.com/invoice","Test":"TestSkipped","Elapsed":0}
        "#,
    )?;

    assert_eq!(report.total_count(), 3);
    assert_eq!(report.passed_count(), 1);
    assert_eq!(report.failed_count(), 1);
    assert_eq!(report.skipped_count(), 1);
    let results = report.results().collect::<Vec<_>>();
    let first = results
        .first()
        .ok_or_else(|| std::io::Error::other("missing first go test result"))?;
    let second = results
        .get(1)
        .ok_or_else(|| std::io::Error::other("missing second go test result"))?;
    assert_eq!(first.suite_name, "go_test");
    assert_eq!(first.class_name.as_deref(), Some("example.com/invoice"));
    assert_eq!(first.name, "TestAddsRate");
    assert_eq!(first.fqn, "example.com/invoice::TestAddsRate");
    assert_eq!(first.status, TestResultStatus::Passed);
    assert_eq!(first.duration_ms, Some(12));
    assert_eq!(second.status, TestResultStatus::Failed);
    assert_eq!(second.message.as_deref(), Some("expected error\n"));
    Ok(())
}

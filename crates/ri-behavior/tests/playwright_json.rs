#![allow(
    missing_docs,
    reason = "Playwright parser test names document behavior."
)]

use ri_behavior::{TestResultStatus, parse_playwright_json};

#[test]
fn parses_playwright_json_test_results() -> Result<(), Box<dyn std::error::Error>> {
    let report = parse_playwright_json(
        r#"
        {
          "suites": [
            {
              "title": "chromium",
              "file": "tests/invoice.spec.ts",
              "specs": [
                {
                  "title": "adds tax",
                  "tests": [
                    {
                      "projectName": "chromium",
                      "status": "expected",
                      "results": [{"status": "passed", "duration": 12}]
                    }
                  ]
                },
                {
                  "title": "rejects negative",
                  "tests": [
                    {
                      "projectName": "chromium",
                      "status": "unexpected",
                      "results": [
                        {
                          "status": "failed",
                          "duration": 3,
                          "error": {"message": "expected 400"}
                        }
                      ]
                    }
                  ]
                }
              ]
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
        .ok_or_else(|| std::io::Error::other("missing first playwright result"))?;
    let second = results
        .get(1)
        .ok_or_else(|| std::io::Error::other("missing second playwright result"))?;
    assert_eq!(first.suite_name, "playwright");
    assert_eq!(first.file_path.as_deref(), Some("tests/invoice.spec.ts"));
    assert_eq!(first.class_name.as_deref(), Some("chromium"));
    assert_eq!(first.name, "adds tax");
    assert_eq!(
        first.fqn,
        "tests/invoice.spec.ts::chromium::adds tax::chromium"
    );
    assert_eq!(first.duration_ms, Some(12));
    assert_eq!(first.status, TestResultStatus::Passed);
    assert_eq!(second.status, TestResultStatus::Failed);
    assert_eq!(second.message.as_deref(), Some("expected 400"));
    Ok(())
}

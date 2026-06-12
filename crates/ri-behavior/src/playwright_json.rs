use serde::Deserialize;

use crate::{BehaviorError, JunitReport, TestCaseResult, TestResultStatus, TestSuiteResult};

pub fn parse_playwright_json(json: &str) -> Result<JunitReport, BehaviorError> {
    let raw = serde_json::from_str::<RawPlaywrightReport>(json).map_err(|error| {
        BehaviorError::PlaywrightJson {
            message: error.to_string(),
        }
    })?;
    let mut results = Vec::new();
    for suite in raw.suites {
        collect_suite(suite, None, &[], &mut results);
    }
    Ok(JunitReport {
        suites: vec![TestSuiteResult {
            name: "playwright".to_owned(),
            results,
        }],
    })
}

#[derive(Debug, Deserialize)]
struct RawPlaywrightReport {
    #[serde(default)]
    suites: Vec<RawPlaywrightSuite>,
}

#[derive(Debug, Deserialize)]
struct RawPlaywrightSuite {
    #[serde(default)]
    title: String,
    file: Option<String>,
    #[serde(default)]
    suites: Vec<Self>,
    #[serde(default)]
    specs: Vec<RawPlaywrightSpec>,
}

#[derive(Debug, Deserialize)]
struct RawPlaywrightSpec {
    title: String,
    #[serde(default)]
    tests: Vec<RawPlaywrightTest>,
}

#[derive(Debug, Deserialize)]
struct RawPlaywrightTest {
    #[serde(rename = "projectName", default)]
    project_name: String,
    #[serde(default)]
    status: String,
    #[serde(default)]
    results: Vec<RawPlaywrightResult>,
}

#[derive(Debug, Deserialize)]
struct RawPlaywrightResult {
    #[serde(default)]
    status: String,
    duration: Option<i64>,
    error: Option<RawPlaywrightError>,
}

#[derive(Debug, Deserialize)]
struct RawPlaywrightError {
    message: Option<String>,
}

fn collect_suite(
    suite: RawPlaywrightSuite,
    parent_file: Option<&str>,
    parent_titles: &[String],
    output: &mut Vec<TestCaseResult>,
) {
    let file_path = suite.file.as_deref().or(parent_file);
    let titles = suite_titles(parent_titles, &suite.title);
    for spec in suite.specs {
        collect_spec(spec, file_path, &titles, output);
    }
    for child in suite.suites {
        collect_suite(child, file_path, &titles, output);
    }
}

fn collect_spec(
    spec: RawPlaywrightSpec,
    file_path: Option<&str>,
    suite_titles: &[String],
    output: &mut Vec<TestCaseResult>,
) {
    for test in spec.tests {
        let class_name = class_name(suite_titles);
        let fqn = fqn_for(
            file_path,
            class_name.as_deref(),
            &spec.title,
            &test.project_name,
        );
        output.push(TestCaseResult {
            suite_name: "playwright".to_owned(),
            class_name,
            name: spec.title.clone(),
            fqn,
            file_path: file_path.map(str::to_owned),
            status: status_for(&test),
            duration_ms: duration_ms(test.results.as_slice()),
            message: message_for(test.results.as_slice()),
        });
    }
}

fn suite_titles(parent_titles: &[String], title: &str) -> Vec<String> {
    let mut titles = parent_titles.to_vec();
    if !title.is_empty() {
        titles.push(title.to_owned());
    }
    titles
}

fn class_name(suite_titles: &[String]) -> Option<String> {
    if suite_titles.is_empty() {
        return None;
    }
    Some(suite_titles.join("::"))
}

fn fqn_for(
    file_path: Option<&str>,
    class_name: Option<&str>,
    title: &str,
    project: &str,
) -> String {
    [file_path, class_name, Some(title), non_empty(project)]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join("::")
}

const fn non_empty(value: &str) -> Option<&str> {
    if value.is_empty() { None } else { Some(value) }
}

fn status_for(test: &RawPlaywrightTest) -> TestResultStatus {
    let result_status = test
        .results
        .iter()
        .map(|result| result.status.as_str())
        .fold(None, |current, status| {
            Some(merge_status(current, status_for_value(status)))
        });
    result_status.unwrap_or_else(|| status_for_value(&test.status))
}

const fn merge_status(
    current: Option<TestResultStatus>,
    next: TestResultStatus,
) -> TestResultStatus {
    match (current, next) {
        (Some(TestResultStatus::Error), _) | (_, TestResultStatus::Error) => {
            TestResultStatus::Error
        }
        (Some(TestResultStatus::Failed), _) | (_, TestResultStatus::Failed) => {
            TestResultStatus::Failed
        }
        (Some(TestResultStatus::Skipped), _) | (_, TestResultStatus::Skipped) => {
            TestResultStatus::Skipped
        }
        (Some(TestResultStatus::Passed) | None, TestResultStatus::Passed) => {
            TestResultStatus::Passed
        }
    }
}

const fn status_for_value(value: &str) -> TestResultStatus {
    match value.as_bytes() {
        b"expected" | b"flaky" | b"passed" => TestResultStatus::Passed,
        b"unexpected" | b"failed" => TestResultStatus::Failed,
        b"skipped" => TestResultStatus::Skipped,
        _ => TestResultStatus::Error,
    }
}

fn duration_ms(results: &[RawPlaywrightResult]) -> Option<i64> {
    results
        .iter()
        .filter_map(|result| result.duration)
        .try_fold(0_i64, i64::checked_add)
}

fn message_for(results: &[RawPlaywrightResult]) -> Option<String> {
    results
        .iter()
        .find_map(|result| result.error.as_ref()?.message.clone())
}

use std::collections::BTreeMap;

use serde::Deserialize;
use serde_json::Number;

use crate::{BehaviorError, JunitReport, TestCaseResult, TestResultStatus, TestSuiteResult};

pub fn parse_go_test_json(json: &str) -> Result<JunitReport, BehaviorError> {
    let mut tests = BTreeMap::<String, GoTestAccumulator>::new();
    for (line_index, line) in json.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let event = serde_json::from_str::<RawGoTestEvent>(trimmed).map_err(|error| {
            BehaviorError::GoTestJson {
                message: format!("line {}: {error}", line_index.saturating_add(1)),
            }
        })?;
        apply_event(event, &mut tests);
    }
    Ok(JunitReport {
        suites: vec![TestSuiteResult {
            name: "go_test".to_owned(),
            results: tests
                .into_values()
                .filter_map(GoTestAccumulator::into_result)
                .collect(),
        }],
    })
}

#[derive(Debug, Deserialize)]
struct RawGoTestEvent {
    #[serde(rename = "Action")]
    action: String,
    #[serde(rename = "Package")]
    package: String,
    #[serde(rename = "Test")]
    test: Option<String>,
    #[serde(rename = "Elapsed")]
    elapsed: Option<Number>,
    #[serde(rename = "Output")]
    output: Option<String>,
}

#[derive(Debug)]
struct GoTestAccumulator {
    package: String,
    name: String,
    status: Option<TestResultStatus>,
    duration_ms: Option<i64>,
    message: String,
}

impl GoTestAccumulator {
    const fn new(package: String, name: String) -> Self {
        Self {
            package,
            name,
            status: None,
            duration_ms: None,
            message: String::new(),
        }
    }

    fn into_result(self) -> Option<TestCaseResult> {
        let status = self.status?;
        let message = if self.message.is_empty() {
            None
        } else {
            Some(self.message)
        };
        Some(TestCaseResult {
            suite_name: "go_test".to_owned(),
            class_name: Some(self.package.clone()),
            name: self.name.clone(),
            fqn: format!("{}::{}", self.package, self.name),
            file_path: None,
            status,
            duration_ms: self.duration_ms,
            message,
        })
    }
}

fn apply_event(event: RawGoTestEvent, tests: &mut BTreeMap<String, GoTestAccumulator>) {
    let Some(test_name) = event.test else {
        return;
    };
    let key = format!("{}::{}", event.package, test_name);
    let entry = tests
        .entry(key)
        .or_insert_with(|| GoTestAccumulator::new(event.package, test_name));
    if let Some(output) = event.output {
        entry.message.push_str(output.as_str());
    }
    if let Some(status) = status_for(event.action.as_str()) {
        entry.status = Some(status);
        entry.duration_ms = event.elapsed.as_ref().and_then(number_seconds_to_millis);
    }
}

const fn status_for(action: &str) -> Option<TestResultStatus> {
    match action.as_bytes() {
        b"pass" => Some(TestResultStatus::Passed),
        b"fail" => Some(TestResultStatus::Failed),
        b"skip" => Some(TestResultStatus::Skipped),
        _ => None,
    }
}

fn number_seconds_to_millis(value: &Number) -> Option<i64> {
    seconds_to_millis(value.to_string().as_str())
}

fn seconds_to_millis(value: &str) -> Option<i64> {
    let (seconds, fraction) = value.split_once('.').map_or((value, ""), |parts| parts);
    let whole_ms = seconds.parse::<i64>().ok()?.checked_mul(1_000)?;
    whole_ms.checked_add(millisecond_fraction(fraction)?)
}

fn millisecond_fraction(fraction: &str) -> Option<i64> {
    let millis = fraction
        .chars()
        .take(3)
        .chain(std::iter::repeat('0'))
        .take(3)
        .collect::<String>();
    millis.parse::<i64>().ok()
}

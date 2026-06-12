use serde::Deserialize;

use crate::{BehaviorError, JunitReport, TestCaseResult, TestResultStatus, TestSuiteResult};

pub fn parse_pytest_json(json: &str) -> Result<JunitReport, BehaviorError> {
    let raw = serde_json::from_str::<RawPytestReport>(json).map_err(|error| {
        BehaviorError::PytestJson {
            message: error.to_string(),
        }
    })?;
    Ok(JunitReport {
        suites: vec![TestSuiteResult {
            name: "pytest".to_owned(),
            results: raw.tests.into_iter().map(TestCaseResult::from).collect(),
        }],
    })
}

#[derive(Debug, Deserialize)]
struct RawPytestReport {
    #[serde(default)]
    tests: Vec<RawPytestTest>,
}

#[derive(Debug, Deserialize)]
struct RawPytestTest {
    nodeid: String,
    outcome: String,
    call: Option<RawPytestCall>,
}

#[derive(Debug, Deserialize)]
struct RawPytestCall {
    crash: Option<RawPytestCrash>,
}

#[derive(Debug, Deserialize)]
struct RawPytestCrash {
    message: Option<String>,
}

impl From<RawPytestTest> for TestCaseResult {
    fn from(raw: RawPytestTest) -> Self {
        let node = PytestNodeId::parse(&raw.nodeid);
        Self {
            suite_name: "pytest".to_owned(),
            class_name: node.class_name,
            name: node.name,
            fqn: raw.nodeid,
            file_path: Some(node.file_path),
            status: status_for(raw.outcome.as_str()),
            duration_ms: None,
            message: raw.call.and_then(|call| call.crash?.message),
        }
    }
}

#[derive(Debug)]
struct PytestNodeId {
    file_path: String,
    class_name: Option<String>,
    name: String,
}

impl PytestNodeId {
    fn parse(nodeid: &str) -> Self {
        let mut parts = nodeid.split("::");
        let file_path = parts.next().unwrap_or_default().to_owned();
        let remaining = parts.collect::<Vec<_>>();
        let name = remaining.last().copied().unwrap_or(nodeid).to_owned();
        let class_name = remaining
            .get(remaining.len().saturating_sub(2))
            .filter(|value| value.chars().next().is_some_and(char::is_uppercase))
            .map(|value| (*value).to_owned());
        Self {
            file_path,
            class_name,
            name,
        }
    }
}

const fn status_for(outcome: &str) -> TestResultStatus {
    match outcome.as_bytes() {
        b"passed" => TestResultStatus::Passed,
        b"failed" => TestResultStatus::Failed,
        b"skipped" | b"xfailed" => TestResultStatus::Skipped,
        _ => TestResultStatus::Error,
    }
}

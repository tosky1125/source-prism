use serde::{Deserialize, Serialize};

use crate::BehaviorError;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct JunitReport {
    pub suites: Vec<TestSuiteResult>,
}

impl JunitReport {
    pub fn results(&self) -> impl Iterator<Item = &TestCaseResult> {
        self.suites.iter().flat_map(|suite| suite.results.iter())
    }

    pub fn total_count(&self) -> u32 {
        count_len(self.results().count())
    }

    pub fn passed_count(&self) -> u32 {
        count_len(
            self.results()
                .filter(|result| result.status == TestResultStatus::Passed)
                .count(),
        )
    }

    pub fn failed_count(&self) -> u32 {
        count_len(
            self.results()
                .filter(|result| result.status == TestResultStatus::Failed)
                .count(),
        )
    }

    pub fn error_count(&self) -> u32 {
        count_len(
            self.results()
                .filter(|result| result.status == TestResultStatus::Error)
                .count(),
        )
    }

    pub fn skipped_count(&self) -> u32 {
        count_len(
            self.results()
                .filter(|result| result.status == TestResultStatus::Skipped)
                .count(),
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct TestSuiteResult {
    pub name: String,
    pub results: Vec<TestCaseResult>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct TestCaseResult {
    pub suite_name: String,
    pub class_name: Option<String>,
    pub name: String,
    pub fqn: String,
    pub file_path: Option<String>,
    pub status: TestResultStatus,
    pub duration_ms: Option<i64>,
    pub message: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum TestResultStatus {
    Passed,
    Failed,
    Error,
    Skipped,
}

pub fn parse_junit_xml(xml: &str) -> Result<JunitReport, BehaviorError> {
    let raw_suites = if xml.contains("<testsuites") {
        quick_xml::de::from_str::<RawTestSuites>(xml)
            .map_err(|error| BehaviorError::JunitXml {
                message: error.to_string(),
            })?
            .suites
    } else {
        vec![
            quick_xml::de::from_str::<RawTestSuite>(xml).map_err(|error| {
                BehaviorError::JunitXml {
                    message: error.to_string(),
                }
            })?,
        ]
    };
    Ok(JunitReport {
        suites: raw_suites.into_iter().map(TestSuiteResult::from).collect(),
    })
}

#[derive(Debug, Deserialize)]
struct RawTestSuites {
    #[serde(rename = "testsuite", default)]
    suites: Vec<RawTestSuite>,
}

#[derive(Debug, Deserialize)]
struct RawTestSuite {
    #[serde(rename = "@name", default)]
    name: String,
    #[serde(rename = "testcase", default)]
    cases: Vec<RawTestCase>,
}

#[derive(Debug, Deserialize)]
struct RawTestCase {
    #[serde(rename = "@classname")]
    class_name: Option<String>,
    #[serde(rename = "@name")]
    name: String,
    #[serde(rename = "@file")]
    file_path: Option<String>,
    #[serde(rename = "@time")]
    time_seconds: Option<String>,
    failure: Option<RawOutcome>,
    error: Option<RawOutcome>,
    skipped: Option<RawOutcome>,
}

#[derive(Debug, Deserialize)]
struct RawOutcome {
    #[serde(rename = "@message")]
    message: Option<String>,
}

impl From<RawTestSuite> for TestSuiteResult {
    fn from(raw: RawTestSuite) -> Self {
        let name = raw.name;
        let results = raw
            .cases
            .into_iter()
            .map(|case| TestCaseResult::from_raw(&name, case))
            .collect();
        Self { name, results }
    }
}

impl TestCaseResult {
    fn from_raw(suite_name: &str, raw: RawTestCase) -> Self {
        let status = status_for(&raw);
        let message = message_for(&raw);
        let fqn = fqn_for(raw.class_name.as_deref(), &raw.name);
        Self {
            suite_name: suite_name.to_owned(),
            class_name: raw.class_name,
            name: raw.name,
            fqn,
            file_path: raw.file_path,
            status,
            duration_ms: raw.time_seconds.as_deref().and_then(seconds_to_millis),
            message,
        }
    }
}

const fn status_for(raw: &RawTestCase) -> TestResultStatus {
    if raw.error.is_some() {
        TestResultStatus::Error
    } else if raw.failure.is_some() {
        TestResultStatus::Failed
    } else if raw.skipped.is_some() {
        TestResultStatus::Skipped
    } else {
        TestResultStatus::Passed
    }
}

fn message_for(raw: &RawTestCase) -> Option<String> {
    raw.error
        .as_ref()
        .or(raw.failure.as_ref())
        .or(raw.skipped.as_ref())
        .and_then(|outcome| outcome.message.clone())
}

fn fqn_for(class_name: Option<&str>, name: &str) -> String {
    class_name.map_or_else(
        || name.to_owned(),
        |class| {
            if class.is_empty() {
                name.to_owned()
            } else {
                format!("{class}::{name}")
            }
        },
    )
}

fn seconds_to_millis(value: &str) -> Option<i64> {
    let (seconds, fraction) = value.split_once('.').map_or((value, ""), |parts| parts);
    let whole_ms = seconds.parse::<i64>().ok()?.checked_mul(1_000)?;
    let fraction_ms = millisecond_fraction(fraction)?;
    whole_ms.checked_add(fraction_ms)
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

fn count_len(count: usize) -> u32 {
    u32::try_from(count).unwrap_or(u32::MAX)
}

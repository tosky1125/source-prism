#![allow(
    missing_docs,
    reason = "GitHub payload contracts are serialized fixtures at this milestone."
)]

use ri_review::{FindingSeverity, VerifiedFinding, redact_review_text};
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct GitHubReviewDryRun {
    pub annotations: Vec<GitHubCheckAnnotation>,
    pub sarif: SarifLog,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct GitHubCheckAnnotation {
    pub path: String,
    pub start_line: u32,
    pub end_line: u32,
    pub annotation_level: GitHubAnnotationLevel,
    pub title: String,
    pub message: String,
    pub raw_details: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum GitHubAnnotationLevel {
    Notice,
    Warning,
    Failure,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct SarifLog {
    pub version: &'static str,
    #[serde(rename = "$schema")]
    pub schema: &'static str,
    pub runs: Vec<SarifRun>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct SarifRun {
    pub tool: SarifTool,
    pub results: Vec<SarifResult>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct SarifTool {
    pub driver: SarifDriver,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct SarifDriver {
    pub name: &'static str,
    pub rules: Vec<SarifRule>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct SarifRule {
    pub id: &'static str,
    pub name: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct SarifResult {
    #[serde(rename = "ruleId")]
    pub rule_id: &'static str,
    pub level: SarifLevel,
    pub message: SarifMessage,
    pub locations: Vec<SarifLocation>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum SarifLevel {
    Note,
    Warning,
    Error,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct SarifMessage {
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct SarifLocation {
    #[serde(rename = "physicalLocation")]
    pub physical_location: SarifPhysicalLocation,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct SarifPhysicalLocation {
    #[serde(rename = "artifactLocation")]
    pub artifact_location: SarifArtifactLocation,
    pub region: SarifRegion,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct SarifArtifactLocation {
    pub uri: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct SarifRegion {
    #[serde(rename = "startLine")]
    pub start_line: u32,
    #[serde(rename = "endLine")]
    pub end_line: u32,
}

pub fn build_review_dry_run(findings: &[VerifiedFinding]) -> GitHubReviewDryRun {
    GitHubReviewDryRun {
        annotations: findings.iter().map(check_annotation).collect(),
        sarif: sarif_log(findings),
    }
}

fn check_annotation(finding: &VerifiedFinding) -> GitHubCheckAnnotation {
    GitHubCheckAnnotation {
        path: finding.file_path.to_string(),
        start_line: finding.start_line,
        end_line: finding.end_line,
        annotation_level: annotation_level(finding.severity),
        title: redact_review_text(finding.title.as_str()),
        message: redact_review_text(finding.recommendation.as_str()),
        raw_details: evidence_details(finding),
    }
}

fn sarif_log(findings: &[VerifiedFinding]) -> SarifLog {
    SarifLog {
        version: "2.1.0",
        schema: "https://json.schemastore.org/sarif-2.1.0.json",
        runs: vec![SarifRun {
            tool: SarifTool {
                driver: SarifDriver {
                    name: "Source Prism",
                    rules: vec![SarifRule {
                        id: "source-prism.review_finding",
                        name: "Evidence-backed review finding",
                    }],
                },
            },
            results: findings.iter().map(sarif_result).collect(),
        }],
    }
}

fn sarif_result(finding: &VerifiedFinding) -> SarifResult {
    SarifResult {
        rule_id: "source-prism.review_finding",
        level: sarif_level(finding.severity),
        message: SarifMessage {
            text: redact_review_text(finding.title.as_str()),
        },
        locations: vec![SarifLocation {
            physical_location: SarifPhysicalLocation {
                artifact_location: SarifArtifactLocation {
                    uri: finding.file_path.to_string(),
                },
                region: SarifRegion {
                    start_line: finding.start_line,
                    end_line: finding.end_line,
                },
            },
        }],
    }
}

const fn annotation_level(severity: FindingSeverity) -> GitHubAnnotationLevel {
    match severity {
        FindingSeverity::Low => GitHubAnnotationLevel::Notice,
        FindingSeverity::Medium => GitHubAnnotationLevel::Warning,
        _ => GitHubAnnotationLevel::Failure,
    }
}

const fn sarif_level(severity: FindingSeverity) -> SarifLevel {
    match severity {
        FindingSeverity::Low => SarifLevel::Note,
        FindingSeverity::Medium => SarifLevel::Warning,
        _ => SarifLevel::Error,
    }
}

fn evidence_details(finding: &VerifiedFinding) -> String {
    finding
        .evidence
        .iter()
        .map(|evidence| {
            format!(
                "{}:{}-{} {}",
                evidence.file_path,
                evidence.start_line,
                evidence.end_line,
                redact_review_text(evidence.summary.as_str())
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

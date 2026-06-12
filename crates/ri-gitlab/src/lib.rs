#![allow(
    missing_docs,
    reason = "GitLab payload contracts are serialized fixtures at this milestone."
)]

use ri_review::{FindingSeverity, VerifiedFinding, redact_review_text};
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct GitLabReviewDryRun {
    pub discussions: Vec<GitLabDiscussion>,
    pub code_quality: Vec<GitLabCodeQualityFinding>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct GitLabDiscussion {
    pub body: String,
    pub position: GitLabPosition,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct GitLabPosition {
    pub position_type: &'static str,
    pub new_path: String,
    pub new_line: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct GitLabCodeQualityFinding {
    pub description: String,
    pub check_name: &'static str,
    pub fingerprint: String,
    pub severity: GitLabCodeQualitySeverity,
    pub location: GitLabCodeQualityLocation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum GitLabCodeQualitySeverity {
    Minor,
    Major,
    Critical,
    Blocker,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct GitLabCodeQualityLocation {
    pub path: String,
    pub lines: GitLabCodeQualityLines,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct GitLabCodeQualityLines {
    pub begin: u32,
    pub end: u32,
}

pub fn build_review_dry_run(findings: &[VerifiedFinding]) -> GitLabReviewDryRun {
    GitLabReviewDryRun {
        discussions: findings.iter().map(discussion).collect(),
        code_quality: findings.iter().map(code_quality_finding).collect(),
    }
}

fn discussion(finding: &VerifiedFinding) -> GitLabDiscussion {
    GitLabDiscussion {
        body: discussion_body(finding),
        position: GitLabPosition {
            position_type: "text",
            new_path: finding.file_path.to_string(),
            new_line: finding.start_line,
        },
    }
}

fn code_quality_finding(finding: &VerifiedFinding) -> GitLabCodeQualityFinding {
    GitLabCodeQualityFinding {
        description: redact_review_text(finding.title.as_str()),
        check_name: "source-prism.review_finding",
        fingerprint: fingerprint(finding),
        severity: code_quality_severity(finding.severity),
        location: GitLabCodeQualityLocation {
            path: finding.file_path.to_string(),
            lines: GitLabCodeQualityLines {
                begin: finding.start_line,
                end: finding.end_line,
            },
        },
    }
}

fn discussion_body(finding: &VerifiedFinding) -> String {
    format!(
        "{}\n\nRecommendation: {}\n\nEvidence:\n{}",
        redact_review_text(finding.title.as_str()),
        redact_review_text(finding.recommendation.as_str()),
        evidence_details(finding)
    )
}

const fn code_quality_severity(severity: FindingSeverity) -> GitLabCodeQualitySeverity {
    match severity {
        FindingSeverity::Low => GitLabCodeQualitySeverity::Minor,
        FindingSeverity::Medium => GitLabCodeQualitySeverity::Major,
        FindingSeverity::High => GitLabCodeQualitySeverity::Critical,
        _ => GitLabCodeQualitySeverity::Blocker,
    }
}

fn fingerprint(finding: &VerifiedFinding) -> String {
    format!(
        "source-prism:{}:{}:{}",
        finding.file_path,
        finding.start_line,
        normalize_token(redact_review_text(finding.title.as_str()).as_str())
    )
}

fn normalize_token(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect()
}

fn evidence_details(finding: &VerifiedFinding) -> String {
    finding
        .evidence
        .iter()
        .map(|evidence| {
            format!(
                "- {}:{}-{} {}",
                evidence.file_path,
                evidence.start_line,
                evidence.end_line,
                redact_review_text(evidence.summary.as_str())
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

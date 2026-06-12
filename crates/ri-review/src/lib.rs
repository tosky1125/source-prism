#![allow(
    missing_docs,
    reason = "Review verifier contracts are self-describing at this milestone."
)]

use ri_core::{CoreError, FilePath};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum FindingSeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ProposedFinding {
    pub title: String,
    pub severity: FindingSeverity,
    pub file_path: Option<String>,
    pub start_line: Option<u32>,
    pub end_line: Option<u32>,
    #[serde(default)]
    pub evidence: Vec<ProposedFindingEvidence>,
    #[serde(default)]
    pub impact_path: Vec<ImpactPathStep>,
    #[serde(default)]
    pub recommendation: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ProposedFindingEvidence {
    pub file_path: Option<String>,
    pub start_line: Option<u32>,
    pub end_line: Option<u32>,
    #[serde(default)]
    pub summary: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ImpactPathStep {
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub relation: String,
    #[serde(default)]
    pub target: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct VerifiedFinding {
    pub title: String,
    pub severity: FindingSeverity,
    pub file_path: FilePath,
    pub start_line: u32,
    pub end_line: u32,
    pub evidence: Vec<VerifiedFindingEvidence>,
    pub impact_path: Vec<ImpactPathStep>,
    pub recommendation: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct VerifiedFindingEvidence {
    pub file_path: FilePath,
    pub start_line: u32,
    pub end_line: u32,
    pub summary: String,
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ReviewError {
    #[error("missing required review field: {field}")]
    MissingField { field: &'static str },
    #[error("empty required review field: {field}")]
    EmptyField { field: &'static str },
    #[error("invalid review line range: {field}")]
    InvalidRange { field: &'static str },
    #[error(transparent)]
    Core(#[from] CoreError),
}

pub fn verify_finding(finding: &ProposedFinding) -> Result<VerifiedFinding, ReviewError> {
    let file_path = required_path(finding.file_path.as_deref(), "file_path")?;
    let start_line = required_line(finding.start_line, "start_line")?;
    let end_line = required_line(finding.end_line, "end_line")?;
    ensure_range(start_line, end_line, "location")?;
    let evidence = verify_evidence(finding.evidence.as_slice())?;
    let impact_path = verify_impact_path(finding.impact_path.as_slice())?;
    let recommendation = required_text(finding.recommendation.as_str(), "recommendation")?;
    Ok(VerifiedFinding {
        title: required_text(finding.title.as_str(), "title")?,
        severity: finding.severity,
        file_path,
        start_line,
        end_line,
        evidence,
        impact_path,
        recommendation,
    })
}

pub fn verify_findings(findings: &[ProposedFinding]) -> Result<Vec<VerifiedFinding>, ReviewError> {
    findings.iter().map(verify_finding).collect()
}

fn verify_evidence(
    evidence: &[ProposedFindingEvidence],
) -> Result<Vec<VerifiedFindingEvidence>, ReviewError> {
    if evidence.is_empty() {
        return Err(ReviewError::MissingField { field: "evidence" });
    }
    evidence.iter().map(verify_evidence_item).collect()
}

fn verify_evidence_item(
    evidence: &ProposedFindingEvidence,
) -> Result<VerifiedFindingEvidence, ReviewError> {
    let start_line = required_line(evidence.start_line, "evidence.start_line")?;
    let end_line = required_line(evidence.end_line, "evidence.end_line")?;
    ensure_range(start_line, end_line, "evidence")?;
    Ok(VerifiedFindingEvidence {
        file_path: required_path(evidence.file_path.as_deref(), "evidence.file_path")?,
        start_line,
        end_line,
        summary: required_text(evidence.summary.as_str(), "evidence.summary")?,
    })
}

fn verify_impact_path(steps: &[ImpactPathStep]) -> Result<Vec<ImpactPathStep>, ReviewError> {
    if steps.is_empty() {
        return Err(ReviewError::MissingField {
            field: "impact_path",
        });
    }
    for step in steps {
        required_text(step.source.as_str(), "impact_path.source")?;
        required_text(step.relation.as_str(), "impact_path.relation")?;
        required_text(step.target.as_str(), "impact_path.target")?;
    }
    Ok(steps.to_vec())
}

fn required_path(value: Option<&str>, field: &'static str) -> Result<FilePath, ReviewError> {
    let text = value.ok_or(ReviewError::MissingField { field })?;
    Ok(FilePath::new(required_text(text, field)?)?)
}

fn required_line(value: Option<u32>, field: &'static str) -> Result<u32, ReviewError> {
    let line = value.ok_or(ReviewError::MissingField { field })?;
    if line == 0 {
        return Err(ReviewError::InvalidRange { field });
    }
    Ok(line)
}

fn required_text(value: &str, field: &'static str) -> Result<String, ReviewError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(ReviewError::EmptyField { field });
    }
    Ok(trimmed.to_owned())
}

const fn ensure_range(
    start_line: u32,
    end_line: u32,
    field: &'static str,
) -> Result<(), ReviewError> {
    if end_line < start_line {
        return Err(ReviewError::InvalidRange { field });
    }
    Ok(())
}

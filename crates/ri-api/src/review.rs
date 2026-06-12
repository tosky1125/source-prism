use axum::{Json, extract::State};
use ri_review::{ProposedFinding, VerifiedFinding, verify_findings};
use serde::{Deserialize, Serialize};

use crate::{AppError, state::AppState};

#[derive(Debug, Deserialize)]
pub(crate) struct ReviewVerifyRequest {
    findings: Vec<ProposedFinding>,
}

#[derive(Debug, Serialize)]
pub(crate) struct ReviewVerifyResponse {
    status: &'static str,
    kind: &'static str,
    verified_count: usize,
    findings: Vec<VerifiedFinding>,
}

#[derive(Debug, Serialize)]
pub(crate) struct GitHubDryRunResponse {
    status: &'static str,
    kind: &'static str,
    verified_count: usize,
    annotations: Vec<ri_github::GitHubCheckAnnotation>,
    sarif: ri_github::SarifLog,
}

#[derive(Debug, Serialize)]
pub(crate) struct GitLabDryRunResponse {
    status: &'static str,
    kind: &'static str,
    verified_count: usize,
    discussions: Vec<ri_gitlab::GitLabDiscussion>,
    code_quality: Vec<ri_gitlab::GitLabCodeQualityFinding>,
}

pub(crate) async fn verify(
    State(_state): State<AppState>,
    Json(request): Json<ReviewVerifyRequest>,
) -> Result<Json<ReviewVerifyResponse>, AppError> {
    let findings = verify_findings(request.findings.as_slice())?;
    Ok(Json(ReviewVerifyResponse {
        status: "ok",
        kind: "review_verification",
        verified_count: findings.len(),
        findings,
    }))
}

pub(crate) async fn github_dry_run(
    State(_state): State<AppState>,
    Json(request): Json<ReviewVerifyRequest>,
) -> Result<Json<GitHubDryRunResponse>, AppError> {
    let findings = verify_findings(request.findings.as_slice())?;
    let dry_run = ri_github::build_review_dry_run(findings.as_slice());
    Ok(Json(GitHubDryRunResponse {
        status: "ok",
        kind: "github_review_dry_run",
        verified_count: findings.len(),
        annotations: dry_run.annotations,
        sarif: dry_run.sarif,
    }))
}

pub(crate) async fn gitlab_dry_run(
    State(_state): State<AppState>,
    Json(request): Json<ReviewVerifyRequest>,
) -> Result<Json<GitLabDryRunResponse>, AppError> {
    let findings = verify_findings(request.findings.as_slice())?;
    let dry_run = ri_gitlab::build_review_dry_run(findings.as_slice());
    Ok(Json(GitLabDryRunResponse {
        status: "ok",
        kind: "gitlab_review_dry_run",
        verified_count: findings.len(),
        discussions: dry_run.discussions,
        code_quality: dry_run.code_quality,
    }))
}

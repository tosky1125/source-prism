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

use axum::{
    Json,
    extract::{Path, State},
};
use ri_refactor::{RefactorPlan, plan_refactor};
use serde::{Deserialize, Serialize};

use crate::{AppError, impact::impact_inputs, state::AppState};

#[derive(Debug, Deserialize)]
pub(crate) struct RefactorPlanRequest {
    repo_id: Option<String>,
    symbol: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct RefactorPlanResponse {
    status: &'static str,
    kind: &'static str,
    plan: RefactorPlan,
}

pub(crate) async fn plan(
    State(state): State<AppState>,
    Json(request): Json<RefactorPlanRequest>,
) -> Result<Json<RefactorPlanResponse>, AppError> {
    plan_with_repo(state, request, None).await
}

pub(crate) async fn plan_for_repo(
    State(state): State<AppState>,
    Path(repo_id): Path<String>,
    Json(request): Json<RefactorPlanRequest>,
) -> Result<Json<RefactorPlanResponse>, AppError> {
    plan_with_repo(state, request, Some(repo_id)).await
}

async fn plan_with_repo(
    state: AppState,
    request: RefactorPlanRequest,
    repo_id: Option<String>,
) -> Result<Json<RefactorPlanResponse>, AppError> {
    let symbol = request.symbol.trim();
    if symbol.is_empty() {
        return Err(AppError::Validation("symbol must not be empty".to_owned()));
    }
    let repo_id = repo_id.as_deref().or(request.repo_id.as_deref());
    let (symbols, calls) = impact_inputs(&state, repo_id).await?;
    let impact = ri_impact::analyze_symbol_impact_with_calls(symbols, calls.as_slice(), symbol)?;
    Ok(Json(RefactorPlanResponse {
        status: "ok",
        kind: "refactor_plan",
        plan: plan_refactor(&impact),
    }))
}

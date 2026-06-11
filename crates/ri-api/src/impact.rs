use axum::{Json, extract::State};
use ri_impact::{ImpactReport, analyze_symbol_impact};
use serde::{Deserialize, Serialize};

use crate::{AppError, state::AppState};

#[derive(Debug, Deserialize)]
pub(crate) struct ImpactRequest {
    symbol: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct ImpactResponse {
    status: &'static str,
    kind: &'static str,
    impact: ImpactReport,
}

pub(crate) async fn analyze(
    State(state): State<AppState>,
    Json(request): Json<ImpactRequest>,
) -> Result<Json<ImpactResponse>, AppError> {
    let symbol = request.symbol.trim();
    if symbol.is_empty() {
        return Err(AppError::Validation("symbol must not be empty".to_owned()));
    }
    let symbols = state.context_symbols()?.into_owned();
    Ok(Json(ImpactResponse {
        status: "ok",
        kind: "impact",
        impact: analyze_symbol_impact(symbols, symbol)?,
    }))
}

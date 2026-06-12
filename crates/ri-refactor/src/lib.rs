#![allow(
    missing_docs,
    reason = "Refactor planning contracts are self-describing at this milestone."
)]
#![allow(
    clippy::multiple_crate_versions,
    reason = "Workspace transitive dependencies pull duplicate crate versions outside this crate's control."
)]

use ri_impact::ImpactReport;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RefactorPlan {
    pub symbol: String,
    pub execution_allowed: bool,
    pub execution_policy: RefactorExecutionPolicy,
    pub risk: RefactorRisk,
    pub slices: Vec<RefactorSlice>,
    pub required_gates: Vec<String>,
    pub safety_notes: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum RefactorExecutionPolicy {
    PlannerOnlySandboxRequired,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum RefactorRisk {
    Low,
    Medium,
    High,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RefactorSlice {
    pub title: String,
    pub files: Vec<String>,
    pub rationale: String,
}

pub fn plan_refactor(impact: &ImpactReport) -> RefactorPlan {
    let slices = impact
        .affected_files
        .iter()
        .map(|file| RefactorSlice {
            title: format!("Update {file}"),
            files: vec![file.clone()],
            rationale: rationale_for(file, impact),
        })
        .collect::<Vec<_>>();
    RefactorPlan {
        symbol: impact.symbol.fqn.clone(),
        execution_allowed: false,
        execution_policy: RefactorExecutionPolicy::PlannerOnlySandboxRequired,
        risk: risk_for(impact),
        slices,
        required_gates: required_gates(impact.symbol.fqn.as_str()),
        safety_notes: vec![
            "planner only: no source files are modified".to_owned(),
            "executor remains disabled until sandbox and branch safety gates exist".to_owned(),
        ],
    }
}

fn risk_for(impact: &ImpactReport) -> RefactorRisk {
    if impact.impact_score >= 5 || impact.affected_files.len() >= 4 {
        RefactorRisk::High
    } else if impact.impact_score >= 3 || impact.affected_files.len() >= 2 {
        RefactorRisk::Medium
    } else {
        RefactorRisk::Low
    }
}

fn rationale_for(file: &str, impact: &ImpactReport) -> String {
    if file == impact.symbol.file_path.as_str() {
        return "target symbol definition lives in this file".to_owned();
    }
    "direct caller/callee or same-file related symbol may be affected".to_owned()
}

fn required_gates(symbol: &str) -> Vec<String> {
    vec![
        "cargo fmt --all -- --check".to_owned(),
        "cargo clippy --workspace --all-targets -- -D warnings".to_owned(),
        "cargo test --workspace".to_owned(),
        format!("cargo run -p ri-cli -- impact --symbol {symbol}"),
    ]
}

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
    pub impact_summary: RefactorImpactSummary,
    pub slices: Vec<RefactorSlice>,
    pub required_gates: Vec<RefactorGate>,
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
pub struct RefactorImpactSummary {
    pub impact_score: u32,
    pub affected_files: Vec<String>,
    pub direct_callers: Vec<String>,
    pub direct_callees: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RefactorSlice {
    pub title: String,
    pub files: Vec<String>,
    pub rationale: String,
    pub gate_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RefactorGate {
    pub gate_id: String,
    pub command: String,
    pub required: bool,
    pub blocks_execution: bool,
    pub expected_evidence: String,
}

pub fn plan_refactor(impact: &ImpactReport) -> RefactorPlan {
    let slices = impact
        .affected_files
        .iter()
        .map(|file| RefactorSlice {
            title: format!("Update {file}"),
            files: vec![file.clone()],
            rationale: rationale_for(file, impact),
            gate_ids: slice_gate_ids(),
        })
        .collect::<Vec<_>>();
    RefactorPlan {
        symbol: impact.symbol.fqn.clone(),
        execution_allowed: false,
        execution_policy: RefactorExecutionPolicy::PlannerOnlySandboxRequired,
        risk: risk_for(impact),
        impact_summary: RefactorImpactSummary {
            impact_score: impact.impact_score,
            affected_files: impact.affected_files.clone(),
            direct_callers: impact.direct_callers.clone(),
            direct_callees: impact.direct_callees.clone(),
        },
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

fn slice_gate_ids() -> Vec<String> {
    vec![
        "format".to_owned(),
        "lint".to_owned(),
        "test".to_owned(),
        "impact-diff".to_owned(),
    ]
}

fn required_gates(symbol: &str) -> Vec<RefactorGate> {
    [
        (
            "format",
            "cargo fmt --all -- --check".to_owned(),
            "rustfmt exits 0 with no source rewrite",
        ),
        (
            "lint",
            "cargo clippy --workspace --all-targets -- -D warnings".to_owned(),
            "clippy exits 0 for workspace targets",
        ),
        (
            "test",
            "cargo test --workspace".to_owned(),
            "workspace tests exit 0 after the planned change",
        ),
        (
            "impact-diff",
            format!("cargo run -p ri-cli -- impact --symbol {symbol}"),
            "post-change impact output is reviewed against this plan",
        ),
    ]
    .into_iter()
    .map(|(gate_id, command, expected_evidence)| RefactorGate {
        gate_id: gate_id.to_owned(),
        command,
        required: true,
        blocks_execution: true,
        expected_evidence: expected_evidence.to_owned(),
    })
    .collect()
}

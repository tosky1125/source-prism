#![allow(missing_docs, reason = "Integration test names document behavior.")]

use ri_core::{CommitSha, FilePath, Language, RepoId, SymbolKind};
use ri_impact::{ImpactCallEdge, analyze_symbol_impact_with_calls};
use ri_refactor::{RefactorExecutionPolicy, RefactorRisk, plan_refactor};
use ri_symbols::{SymbolRange, SymbolRecord, SymbolSpec};

#[test]
fn refactor_plan_is_planner_only_and_requires_safety_gates()
-> Result<(), Box<dyn std::error::Error>> {
    let repo = RepoId::new("repo")?;
    let commit = CommitSha::new("commit")?;
    let caller = symbol(&repo, &commit, "charge_invoice", "src/api.rs", 10)?;
    let target = symbol(&repo, &commit, "apply_tax", "src/tax.rs", 3)?;
    let calls = vec![ImpactCallEdge::new(
        caller.versioned_symbol_id.clone(),
        target.versioned_symbol_id.clone(),
    )];
    let impact =
        analyze_symbol_impact_with_calls(vec![caller, target], calls.as_slice(), "apply_tax")?;

    let plan = plan_refactor(&impact);

    assert_eq!(plan.symbol, "apply_tax");
    assert!(!plan.execution_allowed);
    assert_eq!(
        plan.execution_policy,
        RefactorExecutionPolicy::PlannerOnlySandboxRequired
    );
    assert!(
        plan.required_gates
            .iter()
            .any(|gate| gate.command == "cargo test --workspace")
    );
    assert!(
        plan.required_gates
            .iter()
            .all(|gate| gate.required && gate.blocks_execution)
    );
    assert!(
        plan.slices
            .iter()
            .any(|slice| slice.files == ["src/tax.rs"] && !slice.gate_ids.is_empty())
    );
    assert_eq!(plan.impact_summary.direct_callers, vec!["charge_invoice"]);
    assert_eq!(plan.impact_summary.direct_callees, Vec::<String>::new());
    Ok(())
}

#[test]
fn refactor_plan_marks_broad_impact_as_high_risk() -> Result<(), Box<dyn std::error::Error>> {
    let repo = RepoId::new("repo")?;
    let commit = CommitSha::new("commit")?;
    let target = symbol(&repo, &commit, "apply_tax", "src/tax.rs", 3)?;
    let callers = ["src/api.rs", "src/batch.rs", "src/ui.rs", "src/report.rs"]
        .iter()
        .enumerate()
        .map(|(index, path)| symbol(&repo, &commit, &format!("caller_{index}"), path, 10))
        .collect::<Result<Vec<_>, _>>()?;
    let calls = callers
        .iter()
        .map(|caller| {
            ImpactCallEdge::new(
                caller.versioned_symbol_id.clone(),
                target.versioned_symbol_id.clone(),
            )
        })
        .collect::<Vec<_>>();
    let mut symbols = callers;
    symbols.push(target);
    let impact = analyze_symbol_impact_with_calls(symbols, calls.as_slice(), "apply_tax")?;

    let plan = plan_refactor(&impact);

    assert_eq!(plan.risk, RefactorRisk::High);
    assert!(plan.slices.len() >= 4);
    assert_eq!(plan.impact_summary.affected_files.len(), plan.slices.len());
    Ok(())
}

fn symbol(
    repo: &RepoId,
    commit: &CommitSha,
    fqn: &str,
    path: &str,
    start_line: u32,
) -> Result<SymbolRecord, ri_core::CoreError> {
    Ok(SymbolRecord::new(
        repo,
        commit,
        FilePath::new(path)?,
        "hash",
        SymbolSpec::new(
            Language::Rust,
            SymbolKind::Function,
            fqn,
            fqn,
            SymbolRange::new(start_line, 0, start_line, 10),
        ),
    ))
}

#![allow(missing_docs, reason = "Integration test names document behavior.")]

use ri_core::{CommitSha, FilePath, Language, RepoId, SymbolKind};
use ri_impact::analyze_symbol_impact;
use ri_symbols::{SymbolRange, SymbolRecord, SymbolSpec};

#[test]
fn impact_report_includes_same_file_related_symbols() -> Result<(), Box<dyn std::error::Error>> {
    let repo = RepoId::new("repo")?;
    let commit = CommitSha::new("commit")?;
    let file = FilePath::new("src/lib.rs")?;
    let target = symbol(&repo, &commit, file.clone(), "apply_tax", 3);
    let sibling = symbol(&repo, &commit, file, "Invoice", 1);

    let report = analyze_symbol_impact(vec![target, sibling], "apply_tax")?;

    assert_eq!(report.affected_files, vec!["src/lib.rs"]);
    assert_eq!(report.related.len(), 1);
    assert_eq!(
        report
            .related
            .first()
            .map(|relation| relation.target_fqn.as_str()),
        Some("Invoice")
    );
    assert_eq!(report.impact_score, 2);
    Ok(())
}

#[test]
fn impact_report_rejects_unknown_symbol() {
    let report = analyze_symbol_impact(Vec::new(), "missing");

    assert!(report.is_err());
}

fn symbol(
    repo: &RepoId,
    commit: &CommitSha,
    file_path: FilePath,
    fqn: &str,
    start_line: u32,
) -> SymbolRecord {
    SymbolRecord::new(
        repo,
        commit,
        file_path,
        "hash",
        SymbolSpec::new(
            Language::Rust,
            SymbolKind::Function,
            fqn,
            fqn,
            SymbolRange::new(start_line, 0, start_line, 10),
        ),
    )
}

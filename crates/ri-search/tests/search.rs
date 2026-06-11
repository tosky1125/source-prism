#![allow(missing_docs, reason = "Integration test names document behavior.")]

use ri_core::{CommitSha, FilePath, Language, RepoId, SymbolKind};
use ri_search::search_symbols;
use ri_symbols::{SymbolRange, SymbolRecord, SymbolSpec};

#[test]
fn exact_identifier_beats_lexical_path_match() -> Result<(), Box<dyn std::error::Error>> {
    let repo = RepoId::new("repo")?;
    let commit = CommitSha::new("commit")?;
    let target = symbol(&repo, &commit, "src/other.rs", "apply_tax")?;
    let path_match = symbol(&repo, &commit, "src/apply_tax/mod.rs", "helper")?;

    let report = search_symbols(&[path_match, target], "apply_tax", 2);

    assert!(!report.vector_only);
    assert_eq!(
        report.hits.first().map(|hit| hit.symbol.fqn.as_str()),
        Some("apply_tax")
    );
    assert_eq!(
        report.hits.first().map(|hit| hit.exact_identifier_match),
        Some(true)
    );
    Ok(())
}

fn symbol(
    repo: &RepoId,
    commit: &CommitSha,
    path: &str,
    fqn: &str,
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
            SymbolRange::new(1, 0, 2, 0),
        ),
    ))
}

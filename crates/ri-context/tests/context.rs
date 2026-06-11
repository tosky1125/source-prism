#![allow(missing_docs, reason = "Integration test names document behavior.")]

use ri_context::{RetrievalMode, build_context_pack};
use ri_core::{CommitSha, FilePath, Language, RepoId, SymbolKind};
use ri_symbols::{SymbolRange, SymbolRecord, SymbolSpec};

#[test]
fn context_pack_is_not_vector_only_and_includes_impact() -> Result<(), Box<dyn std::error::Error>> {
    let repo = RepoId::new("repo")?;
    let commit = CommitSha::new("commit")?;
    let target = symbol(&repo, &commit, "apply_tax")?;
    let sibling = symbol(&repo, &commit, "helper")?;

    let symbols = vec![target, sibling];
    let pack = build_context_pack(&symbols, "apply_tax", 5);

    assert!(!pack.vector_only);
    assert!(
        pack.retrieval_modes
            .contains(&RetrievalMode::ExactIdentifier)
    );
    assert_eq!(pack.hits.len(), 1);
    assert_eq!(pack.impacts.len(), 1);
    Ok(())
}

fn symbol(
    repo: &RepoId,
    commit: &CommitSha,
    fqn: &str,
) -> Result<SymbolRecord, ri_core::CoreError> {
    Ok(SymbolRecord::new(
        repo,
        commit,
        FilePath::new("src/lib.rs")?,
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

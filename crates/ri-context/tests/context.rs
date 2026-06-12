#![allow(missing_docs, reason = "Integration test names document behavior.")]

use ri_context::{RetrievalMode, build_context_pack, build_context_pack_with_calls};
use ri_core::{CommitSha, FilePath, Language, RepoId, SymbolKind};
use ri_impact::ImpactCallEdge;
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

#[test]
fn context_pack_impacts_include_call_graph_edges() -> Result<(), Box<dyn std::error::Error>> {
    let repo = RepoId::new("repo")?;
    let commit = CommitSha::new("commit")?;
    let target = symbol(&repo, &commit, "search")?;
    let callee = symbol(&repo, &commit, "build_context_pack")?;
    let calls = vec![ImpactCallEdge::new(
        target.versioned_symbol_id.clone(),
        callee.versioned_symbol_id.clone(),
    )];

    let pack = build_context_pack_with_calls(&[target, callee], calls.as_slice(), "search", 5);

    assert_eq!(
        pack.impacts
            .first()
            .map(|impact| impact.direct_callees.as_slice()),
        Some(&["build_context_pack".to_owned()][..])
    );
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

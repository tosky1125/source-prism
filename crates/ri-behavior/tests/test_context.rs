#![allow(missing_docs, reason = "Integration test names document behavior.")]

use ri_behavior::build_test_context;
use ri_core::{CommitSha, FilePath, Language, RepoId, SymbolKind};
use ri_symbols::{SymbolRange, SymbolRecord, SymbolSpec};

#[test]
fn test_context_links_named_test_without_executing_code() -> Result<(), Box<dyn std::error::Error>>
{
    let repo = RepoId::new("repo")?;
    let commit = CommitSha::new("commit")?;
    let target = symbol(
        &repo,
        &commit,
        SymbolKind::Function,
        "apply_tax",
        "src/invoice.rs",
    )?;
    let test = symbol(
        &repo,
        &commit,
        SymbolKind::TestCase,
        "apply_tax_adds_rate",
        "tests/invoice.rs",
    )?;

    let context = build_test_context(&[target, test], "apply_tax")?;

    assert_eq!(context.symbol, "apply_tax");
    assert!(!context.code_execution_allowed);
    assert!(!context.coverage_available);
    assert_eq!(context.related_tests.len(), 1);
    let related = context
        .related_tests
        .first()
        .ok_or_else(|| std::io::Error::other("expected one related test"))?;
    assert_eq!(related.fqn, "apply_tax_adds_rate");
    Ok(())
}

fn symbol(
    repo: &RepoId,
    commit: &CommitSha,
    kind: SymbolKind,
    fqn: &str,
    path: &str,
) -> Result<SymbolRecord, ri_core::CoreError> {
    Ok(SymbolRecord::new(
        repo,
        commit,
        FilePath::new(path)?,
        "hash",
        SymbolSpec::new(Language::Rust, kind, fqn, fqn, SymbolRange::new(1, 0, 2, 0)),
    ))
}

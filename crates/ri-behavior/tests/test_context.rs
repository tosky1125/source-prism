#![allow(missing_docs, reason = "Integration test names document behavior.")]

use ri_behavior::{
    CoverageEvidenceSegment, TestCoverageEdge, build_test_context,
    build_test_context_with_coverage, build_test_context_with_evidence,
};
use ri_core::{CommitSha, Confidence, FilePath, Language, RepoId, SymbolKind};
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

#[test]
fn test_context_uses_graph_test_covers_edges() -> Result<(), Box<dyn std::error::Error>> {
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
        "charges_are_correct",
        "tests/billing.rs",
    )?;
    let coverage = vec![TestCoverageEdge::new(
        test.versioned_symbol_id.clone(),
        target.versioned_symbol_id.clone(),
        Confidence::Medium,
        "graph edge: test_covers".to_owned(),
    )];

    let context =
        build_test_context_with_coverage(&[target, test], coverage.as_slice(), "apply_tax")?;

    assert!(context.coverage_available);
    assert_eq!(context.related_tests.len(), 1);
    let related = context
        .related_tests
        .first()
        .ok_or_else(|| std::io::Error::other("expected graph-related test"))?;
    assert_eq!(related.fqn, "charges_are_correct");
    assert_eq!(related.evidence, "graph edge: test_covers");
    Ok(())
}

#[test]
fn test_context_includes_overlapping_coverage_segments() -> Result<(), Box<dyn std::error::Error>> {
    let repo = RepoId::new("repo")?;
    let commit = CommitSha::new("commit")?;
    let target = symbol(
        &repo,
        &commit,
        SymbolKind::Function,
        "apply_tax",
        "src/invoice.rs",
    )?;
    let coverage = vec![CoverageEvidenceSegment::new(
        "src/invoice.rs",
        2,
        3,
        7,
        "lcov",
        "lcov.info",
    )];

    let context =
        build_test_context_with_evidence(&[target], &[], coverage.as_slice(), "apply_tax")?;

    assert!(context.coverage_available);
    let segment = context
        .coverage_segments
        .first()
        .ok_or_else(|| std::io::Error::other("missing coverage segment"))?;
    assert_eq!(segment.file_path, "src/invoice.rs");
    assert_eq!(segment.hit_count, 7);
    assert_eq!(segment.evidence, "coverage range overlaps target symbol");
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

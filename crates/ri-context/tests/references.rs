#![allow(missing_docs, reason = "Integration test names document behavior.")]

use ri_context::{ResolvedCallReference, find_symbol_references};
use ri_core::{CommitSha, FilePath, Language, RepoId, SymbolKind};
use ri_symbols::{SymbolRange, SymbolRecord, SymbolSpec};

#[test]
fn references_include_direct_call_sites_for_symbol() -> Result<(), Box<dyn std::error::Error>> {
    let caller = symbol("apply_tax_adds_rate", SymbolKind::TestCase, 8)?;
    let target = symbol("apply_tax", SymbolKind::Function, 2)?;
    let call = ResolvedCallReference::new(
        caller.versioned_symbol_id.clone(),
        target.versioned_symbol_id.clone(),
        FilePath::new("src/lib.rs")?,
        "apply_tax".to_owned(),
        SymbolRange::new(9, 16, 9, 25),
    );

    let report = find_symbol_references(&[caller, target], &[call], "apply_tax")?;

    assert_eq!(report.kind, "references");
    assert_eq!(report.symbol.fqn, "apply_tax");
    assert_eq!(report.incoming_count, 1);
    assert_eq!(report.outgoing_count, 0);
    let reference = report
        .references
        .first()
        .ok_or_else(|| std::io::Error::other("missing reference"))?;
    assert_eq!(reference.source_fqn, "apply_tax_adds_rate");
    assert_eq!(reference.target_fqn, "apply_tax");
    assert_eq!(reference.file_path, "src/lib.rs");
    Ok(())
}

fn symbol(
    fqn: &str,
    kind: SymbolKind,
    start_line: u32,
) -> Result<SymbolRecord, ri_core::CoreError> {
    let repo = RepoId::new("repo")?;
    let commit = CommitSha::new("commit")?;
    Ok(SymbolRecord::new(
        &repo,
        &commit,
        FilePath::new("src/lib.rs")?,
        "hash",
        SymbolSpec::new(
            Language::Rust,
            kind,
            fqn,
            fqn,
            SymbolRange::new(start_line, 0, start_line.saturating_add(1), 0),
        ),
    ))
}

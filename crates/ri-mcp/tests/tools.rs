#![allow(missing_docs, reason = "Integration test names document behavior.")]

use ri_context::ResolvedCallReference;
use ri_core::{CommitSha, FilePath, Language, RepoId, SymbolKind};
use ri_mcp::{
    ImpactToolRequest, McpToolCatalog, ReferenceToolRequest, RepositoryToolHandler,
    SearchContextToolRequest, SymbolToolRequest,
};
use ri_symbols::{SymbolRange, SymbolRecord, SymbolSpec};

#[test]
fn tool_catalog_exposes_repo_intelligence_tools() {
    let catalog = McpToolCatalog::new();
    let tools = catalog.tools();
    let names = tools
        .iter()
        .map(|tool| tool.name.as_str())
        .collect::<Vec<_>>();

    assert_eq!(
        names,
        [
            "repo.get_symbol",
            "repo.find_references",
            "repo.get_impact",
            "repo.search_context"
        ]
    );
    assert!(tools.iter().all(|tool| {
        tool.input_schema
            .get("type")
            .and_then(serde_json::Value::as_str)
            == Some("object")
    }));
}

#[test]
fn handler_answers_repo_tools_from_existing_evidence() -> Result<(), Box<dyn std::error::Error>> {
    let repo = RepoId::new("repo")?;
    let commit = CommitSha::new("commit")?;
    let target = symbol(
        &repo,
        &commit,
        SymbolKind::Function,
        "apply_tax",
        "src/invoice.rs",
        SymbolRange::new(1, 0, 3, 1),
    )?;
    let caller = symbol(
        &repo,
        &commit,
        SymbolKind::TestCase,
        "apply_tax_adds_rate",
        "tests/invoice.rs",
        SymbolRange::new(5, 0, 8, 1),
    )?;
    let calls = vec![ResolvedCallReference::new(
        caller.versioned_symbol_id.clone(),
        target.versioned_symbol_id.clone(),
        FilePath::new("tests/invoice.rs")?,
        "apply_tax".to_owned(),
        SymbolRange::new(6, 4, 6, 13),
    )];
    let handler = RepositoryToolHandler::new(vec![target, caller], calls);

    let symbol = handler.get_symbol(&SymbolToolRequest::new("apply_tax"))?;
    let references = handler.find_references(&ReferenceToolRequest::new("apply_tax"))?;
    let impact = handler.get_impact(&ImpactToolRequest::new("apply_tax"))?;
    let context = handler.search_context(&SearchContextToolRequest::new("apply_tax", 4))?;

    assert_eq!(symbol.fqn, "apply_tax");
    assert_eq!(references.references.len(), 1);
    assert_eq!(impact.direct_callers, ["apply_tax_adds_rate"]);
    assert!(!context.vector_only);
    let hit = context
        .hits
        .first()
        .ok_or_else(|| std::io::Error::other("missing context hit"))?;
    assert_eq!(hit.symbol.fqn, "apply_tax");
    Ok(())
}

fn symbol(
    repo: &RepoId,
    commit: &CommitSha,
    kind: SymbolKind,
    fqn: &str,
    path: &str,
    range: SymbolRange,
) -> Result<SymbolRecord, ri_core::CoreError> {
    Ok(SymbolRecord::new(
        repo,
        commit,
        FilePath::new(path)?,
        "hash",
        SymbolSpec::new(Language::Rust, kind, fqn, fqn, range),
    ))
}

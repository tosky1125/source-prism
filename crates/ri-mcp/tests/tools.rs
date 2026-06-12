#![allow(missing_docs, reason = "Integration test names document behavior.")]

use ri_context::ResolvedCallReference;
use ri_core::{CommitSha, FilePath, Language, RepoId, SymbolKind};
use ri_mcp::{
    ImpactToolRequest, McpToolCatalog, ReferenceToolRequest, RepositoryToolHandler,
    SearchContextToolRequest, SymbolToolRequest, TestContextToolRequest, handle_json_rpc_request,
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
            "repo.get_test_context",
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
    let test_context = handler.get_test_context(&TestContextToolRequest::new("apply_tax"))?;
    let context = handler.search_context(&SearchContextToolRequest::new("apply_tax", 4))?;

    assert_eq!(symbol.fqn, "apply_tax");
    assert_eq!(references.references.len(), 1);
    assert_eq!(impact.direct_callers, ["apply_tax_adds_rate"]);
    assert!(!test_context.code_execution_allowed);
    let related_test = test_context
        .related_tests
        .first()
        .ok_or_else(|| std::io::Error::other("missing related test"))?;
    assert_eq!(related_test.fqn, "apply_tax_adds_rate");
    assert_eq!(context.hit_count, 2);
    assert_eq!(context.impact_count, 2);
    assert!(!context.context_pack.vector_only);
    let hit = context
        .context_pack
        .hits
        .first()
        .ok_or_else(|| std::io::Error::other("missing context hit"))?;
    assert_eq!(hit.symbol.fqn, "apply_tax");
    Ok(())
}

#[test]
fn runtime_handles_json_rpc_tool_calls() -> Result<(), Box<dyn std::error::Error>> {
    let handler = fixture_handler()?;
    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 42,
        "method": "tools/call",
        "params": {
            "name": "repo.get_symbol",
            "arguments": {
                "symbol": "apply_tax"
            }
        }
    });

    let response = handle_json_rpc_request(&handler, &request);

    assert_eq!(
        response
            .pointer("/jsonrpc")
            .and_then(serde_json::Value::as_str),
        Some("2.0")
    );
    assert_eq!(
        response.pointer("/id").and_then(serde_json::Value::as_u64),
        Some(42)
    );
    assert_eq!(
        response
            .pointer("/result/structuredContent/fqn")
            .and_then(serde_json::Value::as_str),
        Some("apply_tax")
    );
    assert_eq!(
        response
            .pointer("/result/isError")
            .and_then(serde_json::Value::as_bool),
        Some(false)
    );
    Ok(())
}

#[test]
fn runtime_lists_tools() -> Result<(), Box<dyn std::error::Error>> {
    let handler = fixture_handler()?;
    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list"
    });

    let response = handle_json_rpc_request(&handler, &request);

    assert_eq!(
        response
            .pointer("/result/tools/0/name")
            .and_then(serde_json::Value::as_str),
        Some("repo.get_symbol")
    );
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

fn fixture_handler() -> Result<RepositoryToolHandler, Box<dyn std::error::Error>> {
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
    Ok(RepositoryToolHandler::new(vec![target, caller], calls))
}

#![allow(missing_docs, reason = "Integration test names document behavior.")]

use axum::{
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode},
};
use ri_api::{AppState, app};
use ri_core::{CommitSha, FilePath, Language, RepoId, SymbolKind};
use ri_symbols::{SymbolRange, SymbolRecord, SymbolSpec};
use serde_json::Value;
use tower::ServiceExt;

#[tokio::test]
async fn repo_tests_returns_test_inventory_for_repo() -> Result<(), Box<dyn std::error::Error>> {
    let app = app(AppState::for_test_symbols(vec![
        symbol(SymbolKind::Function, "apply_tax")?,
        symbol(SymbolKind::TestCase, "apply_tax_adds_rate")?,
    ])?);
    let request = Request::builder()
        .method(Method::GET)
        .uri("/v1/repos/local/tests")
        .body(Body::empty())?;

    let response = app.oneshot(request).await?;

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), 1_000_000).await?;
    let body = serde_json::from_slice::<Value>(&bytes)?;
    assert_eq!(body.pointer("/status").and_then(Value::as_str), Some("ok"));
    assert_eq!(body.pointer("/kind").and_then(Value::as_str), Some("tests"));
    assert_eq!(body.pointer("/test_count").and_then(Value::as_u64), Some(1));
    assert_eq!(
        body.pointer("/tests/0/fqn").and_then(Value::as_str),
        Some("apply_tax_adds_rate")
    );
    Ok(())
}

fn symbol(kind: SymbolKind, fqn: &str) -> Result<SymbolRecord, ri_core::CoreError> {
    let repo = RepoId::new("repo")?;
    let commit = CommitSha::new("commit")?;
    Ok(SymbolRecord::new(
        &repo,
        &commit,
        FilePath::new("tests/invoice.rs")?,
        "hash",
        SymbolSpec::new(Language::Rust, kind, fqn, fqn, SymbolRange::new(1, 0, 2, 0)),
    ))
}

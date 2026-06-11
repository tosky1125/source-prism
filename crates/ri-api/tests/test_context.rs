#![allow(missing_docs, reason = "Integration test names document behavior.")]

use axum::{
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode, header},
};
use ri_api::{AppState, app};
use ri_core::{CommitSha, FilePath, Language, RepoId, SymbolKind};
use ri_symbols::{SymbolRange, SymbolRecord, SymbolSpec};
use serde_json::Value;
use tower::ServiceExt;

#[tokio::test]
async fn test_context_returns_static_test_evidence() -> Result<(), Box<dyn std::error::Error>> {
    let app = app(AppState::for_test_symbols(vec![
        symbol(SymbolKind::Function, "apply_tax", "src/invoice.rs")?,
        symbol(
            SymbolKind::TestCase,
            "apply_tax_adds_rate",
            "tests/invoice.rs",
        )?,
    ])?);
    let request = Request::builder()
        .method(Method::POST)
        .uri("/v1/test-context")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(r#"{"symbol":"apply_tax"}"#))?;

    let response = app.oneshot(request).await?;

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), 1_000_000).await?;
    let body = serde_json::from_slice::<Value>(&bytes)?;
    assert_eq!(body.pointer("/status").and_then(Value::as_str), Some("ok"));
    assert_eq!(
        body.pointer("/kind").and_then(Value::as_str),
        Some("test_context")
    );
    assert_eq!(
        body.pointer("/test_context/code_execution_allowed")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        body.pointer("/test_context/related_tests/0/fqn")
            .and_then(Value::as_str),
        Some("apply_tax_adds_rate")
    );
    Ok(())
}

fn symbol(kind: SymbolKind, fqn: &str, path: &str) -> Result<SymbolRecord, ri_core::CoreError> {
    let repo = RepoId::new("repo")?;
    let commit = CommitSha::new("commit")?;
    Ok(SymbolRecord::new(
        &repo,
        &commit,
        FilePath::new(path)?,
        "hash",
        SymbolSpec::new(Language::Rust, kind, fqn, fqn, SymbolRange::new(1, 0, 2, 0)),
    ))
}

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
async fn repo_graph_returns_local_projection_without_database()
-> Result<(), Box<dyn std::error::Error>> {
    let app = app(AppState::for_test_symbols(vec![symbol(
        "src/invoice.rs",
        "apply_tax",
    )?])?);
    let request = Request::builder()
        .method(Method::GET)
        .uri("/v1/repos/local/graph")
        .body(Body::empty())?;

    let response = app.oneshot(request).await?;

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), 1_000_000).await?;
    let body = serde_json::from_slice::<Value>(&bytes)?;
    assert_eq!(body.pointer("/kind").and_then(Value::as_str), Some("graph"));
    assert_eq!(body.pointer("/node_count").and_then(Value::as_u64), Some(2));
    assert_eq!(body.pointer("/edge_count").and_then(Value::as_u64), Some(1));
    assert_eq!(
        body.pointer("/graph/nodes/0/node_type")
            .and_then(Value::as_str),
        Some("file")
    );
    assert_eq!(
        body.pointer("/graph/edges/0/edge_type")
            .and_then(Value::as_str),
        Some("contains")
    );
    Ok(())
}

fn symbol(path: &str, fqn: &str) -> Result<SymbolRecord, ri_core::CoreError> {
    let repo = RepoId::new("repo")?;
    let commit = CommitSha::new("commit")?;
    Ok(SymbolRecord::new(
        &repo,
        &commit,
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

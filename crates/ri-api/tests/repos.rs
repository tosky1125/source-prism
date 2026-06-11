#![allow(missing_docs, reason = "Integration test names document behavior.")]

use axum::{
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode, header},
};
use ri_api::{AppState, app};
use serde_json::Value;
use tower::ServiceExt;

#[tokio::test]
async fn create_repo_returns_registered_repo() -> Result<(), Box<dyn std::error::Error>> {
    let app = app(AppState::for_test_symbols(Vec::new())?);
    let request = Request::builder()
        .method(Method::POST)
        .uri("/v1/repos")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            r#"{"repo_id":"billing","name":"billing","default_branch":"main"}"#,
        ))?;

    let response = app.oneshot(request).await?;

    assert_eq!(response.status(), StatusCode::CREATED);
    let bytes = to_bytes(response.into_body(), 1_000_000).await?;
    let body = serde_json::from_slice::<Value>(&bytes)?;
    assert_eq!(body.pointer("/status").and_then(Value::as_str), Some("ok"));
    assert_eq!(body.pointer("/kind").and_then(Value::as_str), Some("repo"));
    assert_eq!(
        body.pointer("/repo/repo_id").and_then(Value::as_str),
        Some("billing")
    );
    assert_eq!(
        body.pointer("/repo/default_branch").and_then(Value::as_str),
        Some("main")
    );
    Ok(())
}

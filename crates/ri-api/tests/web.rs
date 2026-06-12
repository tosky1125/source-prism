#![allow(
    missing_docs,
    reason = "Web route integration test names document behavior."
)]

use axum::{
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode},
};
use ri_api::{AppState, app};
use tower::ServiceExt;

#[tokio::test]
async fn repo_web_shell_returns_structure_explorer() -> Result<(), Box<dyn std::error::Error>> {
    let app = app(AppState::for_test_symbols(Vec::new())?);
    let request = Request::builder()
        .method(Method::GET)
        .uri("/repo/local")
        .body(Body::empty())?;

    let response = app.oneshot(request).await?;

    assert_eq!(response.status(), StatusCode::OK);
    let body = html_body(response).await?;
    assert!(body.contains("Source Prism"));
    assert!(body.contains("data-repo-id=\"local\""));
    assert!(body.contains("/v1/repos/"));
    assert!(body.contains("Files"));
    assert!(body.contains("Symbols"));
    assert!(body.contains("References"));
    assert!(body.contains("Impact"));
    assert!(body.contains("Coverage"));
    assert!(body.contains("Docs"));
    assert!(body.contains("api(\"references\")"));
    assert!(body.contains("api(\"coverage\")"));
    assert!(body.contains("result.context_pack?.hits"));
    Ok(())
}

#[tokio::test]
async fn repo_web_shell_accepts_deep_repo_views() -> Result<(), Box<dyn std::error::Error>> {
    let app = app(AppState::for_test_symbols(Vec::new())?);
    let request = Request::builder()
        .method(Method::GET)
        .uri("/repo/local/references")
        .body(Body::empty())?;

    let response = app.oneshot(request).await?;

    assert_eq!(response.status(), StatusCode::OK);
    let body = html_body(response).await?;
    assert!(body.contains("data-initial-view=\"references\""));
    Ok(())
}

#[tokio::test]
async fn repo_web_shell_escapes_repo_id_in_markup() -> Result<(), Box<dyn std::error::Error>> {
    let app = app(AppState::for_test_symbols(Vec::new())?);
    let request = Request::builder()
        .method(Method::GET)
        .uri("/repo/%3Cscript%3E/files")
        .body(Body::empty())?;

    let response = app.oneshot(request).await?;

    assert_eq!(response.status(), StatusCode::OK);
    let body = html_body(response).await?;
    assert!(body.contains("data-repo-id=\"&lt;script&gt;\""));
    assert!(!body.contains("data-repo-id=\"<script>\""));
    Ok(())
}

async fn html_body(
    response: axum::response::Response,
) -> Result<String, Box<dyn std::error::Error>> {
    let bytes = to_bytes(response.into_body(), 1_000_000).await?;
    Ok(String::from_utf8(bytes.to_vec())?)
}

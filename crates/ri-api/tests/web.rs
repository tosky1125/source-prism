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
    assert!(body.contains("data-initial-view=\"overview\""));
    assert!(body.contains("<div id=\"root\"></div>"));
    assert!(body.contains("/assets/repo-explorer/assets/repo-explorer.js"));
    assert!(body.contains("/assets/repo-explorer/assets/repo-explorer.css"));
    assert!(
        body.len() < 2_000,
        "repo shell should stay small; TypeScript bundle belongs in assets"
    );
    assert!(!body.contains("react-flow"));
    Ok(())
}

#[tokio::test]
async fn repo_web_shell_accepts_deep_repo_views() -> Result<(), Box<dyn std::error::Error>> {
    let app = app(AppState::for_test_symbols(Vec::new())?);
    for view in [
        "files",
        "symbols",
        "references",
        "impact",
        "tests",
        "coverage",
        "docs",
        "runs",
        "sync",
        "changes",
        "search",
    ] {
        let request = Request::builder()
            .method(Method::GET)
            .uri(format!("/repo/local/{view}"))
            .body(Body::empty())?;

        let response = app.clone().oneshot(request).await?;

        assert_eq!(response.status(), StatusCode::OK);
        let body = html_body(response).await?;
        assert!(body.contains(format!("data-initial-view=\"{view}\"").as_str()));
    }
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

#[tokio::test]
async fn repo_web_assets_serve_split_react_bundle() -> Result<(), Box<dyn std::error::Error>> {
    let app = app(AppState::for_test_symbols(Vec::new())?);
    let js = asset_body(app.clone(), "/assets/repo-explorer/assets/repo-explorer.js").await?;
    let css = asset_body(app, "/assets/repo-explorer/assets/repo-explorer.css").await?;

    assert!(js.contains("Repo intelligence graph"));
    assert!(js.contains("react-flow"));
    assert!(css.contains(".react-flow"));
    Ok(())
}

async fn html_body(
    response: axum::response::Response,
) -> Result<String, Box<dyn std::error::Error>> {
    let bytes = to_bytes(response.into_body(), 1_000_000).await?;
    Ok(String::from_utf8(bytes.to_vec())?)
}

async fn asset_body(app: axum::Router, uri: &str) -> Result<String, Box<dyn std::error::Error>> {
    let request = Request::builder()
        .method(Method::GET)
        .uri(uri)
        .body(Body::empty())?;
    let response = app.oneshot(request).await?;

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), 1_000_000).await?;
    Ok(String::from_utf8(bytes.to_vec())?)
}

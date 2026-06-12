#![allow(missing_docs, reason = "Integration test names document behavior.")]

use axum::{
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode, header},
};
use ri_api::{AppState, app};
use serde_json::Value;
use support::{Fixture, symbol, test_pool};
use tower::ServiceExt;

pub mod support;

#[tokio::test]
async fn impact_returns_report_for_matching_symbol() -> Result<(), Box<dyn std::error::Error>> {
    let app = app(AppState::for_test_symbols(vec![
        symbol("src/invoice.rs", "InvoiceService::apply_tax")?,
        symbol("src/invoice.rs", "InvoiceService::helper")?,
    ])?);
    let request = Request::builder()
        .method(Method::POST)
        .uri("/v1/impact")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(r#"{"symbol":"InvoiceService::apply_tax"}"#))?;

    let response = app.oneshot(request).await?;

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), 1_000_000).await?;
    let body = serde_json::from_slice::<Value>(&bytes)?;
    assert_eq!(body.pointer("/status").and_then(Value::as_str), Some("ok"));
    assert_eq!(
        body.pointer("/kind").and_then(Value::as_str),
        Some("impact")
    );
    assert_eq!(
        body.pointer("/impact/symbol/fqn").and_then(Value::as_str),
        Some("InvoiceService::apply_tax")
    );
    assert_eq!(
        body.pointer("/impact/impact_score").and_then(Value::as_u64),
        Some(2)
    );
    Ok(())
}

#[tokio::test]
async fn impact_with_repo_id_requires_database() -> Result<(), Box<dyn std::error::Error>> {
    let app = app(AppState::for_test_symbols(Vec::new())?);
    let request = Request::builder()
        .method(Method::POST)
        .uri("/v1/impact")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(r#"{"repo_id":"repo","symbol":"run"}"#))?;

    let response = app.oneshot(request).await?;

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let bytes = to_bytes(response.into_body(), 1_000_000).await?;
    let body = serde_json::from_slice::<Value>(&bytes)?;
    assert_eq!(
        body.pointer("/error/code").and_then(Value::as_str),
        Some("database_not_configured")
    );
    Ok(())
}

#[tokio::test]
async fn repo_impact_uses_path_repo_id() -> Result<(), Box<dyn std::error::Error>> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };
    let fixture = Fixture::create(&pool).await?;
    fixture.seed_search_symbol(&pool, "repo_impact").await?;
    let app = app(AppState::for_test_database(pool.clone())?);
    let request = Request::builder()
        .method(Method::POST)
        .uri(format!("/v1/repos/{}/impact", fixture.repo_id))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(r#"{"symbol":"InvoiceService::apply_tax"}"#))?;

    let response = app.oneshot(request).await?;

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), 1_000_000).await?;
    let body = serde_json::from_slice::<Value>(&bytes)?;
    assert_eq!(
        body.pointer("/impact/symbol/fqn").and_then(Value::as_str),
        Some("InvoiceService::apply_tax")
    );
    fixture.cleanup(&pool).await?;
    Ok(())
}

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
async fn context_search_returns_context_pack_for_matching_symbol()
-> Result<(), Box<dyn std::error::Error>> {
    let app = app(AppState::for_test_symbols(vec![
        symbol("src/invoice.rs", "InvoiceService::apply_tax")?,
        symbol("src/invoice.rs", "InvoiceService::helper")?,
    ])?);
    let request = Request::builder()
        .method(Method::POST)
        .uri("/v1/context/search")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(r#"{"query":"apply_tax"}"#))?;

    let response = app.oneshot(request).await?;

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), 1_000_000).await?;
    let body = serde_json::from_slice::<Value>(&bytes)?;
    assert_eq!(body.pointer("/status").and_then(Value::as_str), Some("ok"));
    assert_eq!(
        body.pointer("/kind").and_then(Value::as_str),
        Some("context_search")
    );
    assert_eq!(
        body.pointer("/context_pack/vector_only")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        body.pointer("/context_pack/hits/0/symbol/fqn")
            .and_then(Value::as_str),
        Some("InvoiceService::apply_tax")
    );
    assert_eq!(body.pointer("/hit_count").and_then(Value::as_u64), Some(1));
    assert_eq!(
        body.pointer("/impact_count").and_then(Value::as_u64),
        Some(1)
    );
    assert_eq!(
        body.pointer("/context_pack/impacts")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(1)
    );
    Ok(())
}

#[tokio::test]
async fn context_search_with_repo_id_requires_database() -> Result<(), Box<dyn std::error::Error>> {
    let app = app(AppState::for_test_symbols(Vec::new())?);
    let request = Request::builder()
        .method(Method::POST)
        .uri("/v1/context/search")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(r#"{"repo_id":"repo","query":"apply_tax"}"#))?;

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
async fn context_search_with_repo_id_reports_search_chunk_evidence()
-> Result<(), Box<dyn std::error::Error>> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };
    let fixture = Fixture::create(&pool).await?;
    fixture.seed_search_symbol(&pool, "context_search").await?;
    let app = app(AppState::for_test_database(pool.clone())?);
    let request = Request::builder()
        .method(Method::POST)
        .uri("/v1/context/search")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(format!(
            r#"{{"repo_id":"{}","query":"apply_tax"}}"#,
            fixture.repo_id
        )))?;

    let response = app.oneshot(request).await?;

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), 1_000_000).await?;
    let body = serde_json::from_slice::<Value>(&bytes)?;
    assert_eq!(
        body.pointer("/search_chunk_count").and_then(Value::as_u64),
        Some(1)
    );
    assert_eq!(
        body.pointer("/context_pack/hits/0/symbol/fqn")
            .and_then(Value::as_str),
        Some("InvoiceService::apply_tax")
    );
    fixture.cleanup(&pool).await?;
    Ok(())
}

#[tokio::test]
async fn repo_context_search_uses_path_repo_id() -> Result<(), Box<dyn std::error::Error>> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };
    let fixture = Fixture::create(&pool).await?;
    fixture
        .seed_search_symbol(&pool, "repo_context_search")
        .await?;
    let app = app(AppState::for_test_database(pool.clone())?);
    let request = Request::builder()
        .method(Method::POST)
        .uri(format!("/v1/repos/{}/context/search", fixture.repo_id))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(r#"{"query":"apply_tax"}"#))?;

    let response = app.oneshot(request).await?;

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), 1_000_000).await?;
    let body = serde_json::from_slice::<Value>(&bytes)?;
    assert_eq!(
        body.pointer("/context_pack/hits/0/symbol/fqn")
            .and_then(Value::as_str),
        Some("InvoiceService::apply_tax")
    );
    assert_eq!(
        body.pointer("/search_chunk_count").and_then(Value::as_u64),
        Some(1)
    );
    fixture.cleanup(&pool).await?;
    Ok(())
}

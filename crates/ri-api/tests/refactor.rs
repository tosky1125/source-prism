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
async fn refactor_plan_returns_planner_only_for_matching_symbol()
-> Result<(), Box<dyn std::error::Error>> {
    let app = app(AppState::for_test_symbols(vec![symbol(
        "src/invoice.rs",
        "InvoiceService::apply_tax",
    )?])?);
    let request = Request::builder()
        .method(Method::POST)
        .uri("/v1/refactor/plan")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(r#"{"symbol":"InvoiceService::apply_tax"}"#))?;

    let response = app.oneshot(request).await?;

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), 1_000_000).await?;
    let body = serde_json::from_slice::<Value>(&bytes)?;
    assert_eq!(
        body.pointer("/kind").and_then(Value::as_str),
        Some("refactor_plan")
    );
    assert_eq!(
        body.pointer("/plan/execution_allowed")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        body.pointer("/plan/execution_policy")
            .and_then(Value::as_str),
        Some("planner_only_sandbox_required")
    );
    Ok(())
}

#[tokio::test]
async fn repo_refactor_plan_uses_path_repo_id() -> Result<(), Box<dyn std::error::Error>> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };
    let fixture = Fixture::create(&pool).await?;
    fixture.seed_search_symbol(&pool, "repo_refactor").await?;
    let app = app(AppState::for_test_database(pool.clone())?);
    let request = Request::builder()
        .method(Method::POST)
        .uri(format!("/v1/repos/{}/refactor/plan", fixture.repo_id))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(r#"{"symbol":"InvoiceService::apply_tax"}"#))?;

    let response = app.oneshot(request).await?;

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), 1_000_000).await?;
    let body = serde_json::from_slice::<Value>(&bytes)?;
    assert_eq!(
        body.pointer("/plan/symbol").and_then(Value::as_str),
        Some("InvoiceService::apply_tax")
    );
    assert!(
        body.pointer("/plan/required_gates")
            .and_then(Value::as_array)
            .is_some_and(|gates| !gates.is_empty())
    );
    fixture.cleanup(&pool).await?;
    Ok(())
}

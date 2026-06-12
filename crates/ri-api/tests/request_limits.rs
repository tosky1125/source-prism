#![allow(missing_docs, reason = "Integration test names document behavior.")]

use axum::{
    body::Body,
    http::{Method, Request, StatusCode, header},
};
use ri_api::{ApiRateLimit, AppState, app, app_with_rate_limit};
use std::time::Duration;
use tower::ServiceExt;

#[tokio::test]
async fn api_rejects_oversized_json_request_bodies() -> Result<(), Box<dyn std::error::Error>> {
    // Given: an API app and a JSON payload larger than the public request limit.
    let app = app(AppState::for_test_symbols(Vec::new())?);
    let oversized_diff = "x".repeat(300 * 1024);
    let payload = serde_json::json!({ "diff": oversized_diff }).to_string();

    // When: the oversized payload is posted to a JSON route.
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/v1/repos/local/changed-symbols")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(payload))?,
        )
        .await?;

    // Then: the request is rejected before route logic processes it.
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    Ok(())
}

#[tokio::test]
async fn api_returns_429_when_rate_limit_is_exhausted() -> Result<(), Box<dyn std::error::Error>> {
    // Given: an API app with a one-request fixed-window rate limit.
    let app = app_with_rate_limit(
        AppState::for_test_symbols(Vec::new())?,
        ApiRateLimit::new(1, Duration::from_secs(60))?,
    );

    // When: two requests hit the same process-level window.
    let first = app
        .clone()
        .oneshot(Request::builder().uri("/v1/health").body(Body::empty())?)
        .await?;
    let second = app
        .oneshot(Request::builder().uri("/v1/health").body(Body::empty())?)
        .await?;

    // Then: the first request is allowed and the second is rejected.
    assert_ne!(first.status(), StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(second.status(), StatusCode::TOO_MANY_REQUESTS);
    Ok(())
}

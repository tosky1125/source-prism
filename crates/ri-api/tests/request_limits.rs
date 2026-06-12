#![allow(missing_docs, reason = "Integration test names document behavior.")]

use axum::{
    body::Body,
    http::{Method, Request, StatusCode, header},
};
use ri_api::{AppState, app};
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

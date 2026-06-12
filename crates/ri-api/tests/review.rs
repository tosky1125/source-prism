#![allow(missing_docs, reason = "Integration test names document behavior.")]

use axum::{
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode, header},
};
use ri_api::{AppState, app};
use serde_json::Value;
use tower::ServiceExt;

#[tokio::test]
async fn review_verify_accepts_evidence_backed_findings() -> Result<(), Box<dyn std::error::Error>>
{
    let app = app(AppState::for_test_symbols(Vec::new())?);
    let request = Request::builder()
        .method(Method::POST)
        .uri("/v1/review/verify")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(valid_findings_body()))?;

    let response = app.oneshot(request).await?;

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), 1_000_000).await?;
    let body = serde_json::from_slice::<Value>(&bytes)?;
    assert_eq!(
        body.pointer("/kind").and_then(Value::as_str),
        Some("review_verification")
    );
    assert_eq!(
        body.pointer("/verified_count").and_then(Value::as_u64),
        Some(1)
    );
    assert_eq!(
        body.pointer("/findings/0/file_path")
            .and_then(Value::as_str),
        Some("src/invoice.rs")
    );
    Ok(())
}

#[tokio::test]
async fn review_verify_rejects_findings_without_evidence() -> Result<(), Box<dyn std::error::Error>>
{
    let app = app(AppState::for_test_symbols(Vec::new())?);
    let request = Request::builder()
        .method(Method::POST)
        .uri("/v1/review/verify")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(missing_evidence_body()))?;

    let response = app.oneshot(request).await?;

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let bytes = to_bytes(response.into_body(), 1_000_000).await?;
    let body = serde_json::from_slice::<Value>(&bytes)?;
    assert_eq!(
        body.pointer("/error/code").and_then(Value::as_str),
        Some("review_verification_failed")
    );
    assert!(
        body.pointer("/error/message")
            .and_then(Value::as_str)
            .is_some_and(|message| message.contains("evidence"))
    );
    Ok(())
}

#[tokio::test]
async fn github_review_dry_run_returns_annotations_and_sarif()
-> Result<(), Box<dyn std::error::Error>> {
    let app = app(AppState::for_test_symbols(Vec::new())?);
    let request = Request::builder()
        .method(Method::POST)
        .uri("/v1/review/github-dry-run")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(valid_findings_body()))?;

    let response = app.oneshot(request).await?;

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), 1_000_000).await?;
    let body = serde_json::from_slice::<Value>(&bytes)?;
    assert_eq!(
        body.pointer("/kind").and_then(Value::as_str),
        Some("github_review_dry_run")
    );
    assert_eq!(
        body.pointer("/annotations/0/path").and_then(Value::as_str),
        Some("src/invoice.rs")
    );
    assert_eq!(
        body.pointer("/sarif/version").and_then(Value::as_str),
        Some("2.1.0")
    );
    Ok(())
}

#[tokio::test]
async fn gitlab_review_dry_run_returns_discussions_and_codequality()
-> Result<(), Box<dyn std::error::Error>> {
    let app = app(AppState::for_test_symbols(Vec::new())?);
    let request = Request::builder()
        .method(Method::POST)
        .uri("/v1/review/gitlab-dry-run")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(valid_findings_body()))?;

    let response = app.oneshot(request).await?;

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), 1_000_000).await?;
    let body = serde_json::from_slice::<Value>(&bytes)?;
    assert_eq!(
        body.pointer("/kind").and_then(Value::as_str),
        Some("gitlab_review_dry_run")
    );
    assert_eq!(
        body.pointer("/discussions/0/position/new_path")
            .and_then(Value::as_str),
        Some("src/invoice.rs")
    );
    assert_eq!(
        body.pointer("/code_quality/0/severity")
            .and_then(Value::as_str),
        Some("major")
    );
    Ok(())
}

const fn valid_findings_body() -> &'static str {
    r#"{
      "findings": [
        {
          "title": "Tax rounding can skip fractional cents",
          "severity": "medium",
          "file_path": "src/invoice.rs",
          "start_line": 12,
          "end_line": 16,
          "evidence": [
            {
              "file_path": "src/invoice.rs",
              "start_line": 12,
              "end_line": 16,
              "summary": "rounding happens before line item aggregation"
            }
          ],
          "impact_path": [
            {
              "source": "InvoiceService::applyTax",
              "relation": "calls",
              "target": "Money::round"
            }
          ],
          "recommendation": "Round only after summing line item tax amounts."
        }
      ]
    }"#
}

const fn missing_evidence_body() -> &'static str {
    r#"{
      "findings": [
        {
          "title": "No evidence",
          "severity": "medium",
          "file_path": "src/invoice.rs",
          "start_line": 12,
          "end_line": 16,
          "impact_path": [
            {
              "source": "InvoiceService::applyTax",
              "relation": "calls",
              "target": "Money::round"
            }
          ],
          "recommendation": "Add evidence before publishing."
        }
      ]
    }"#
}

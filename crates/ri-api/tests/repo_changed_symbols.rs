#![allow(missing_docs, reason = "Integration test names document behavior.")]

use axum::{
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode, header},
};
use ri_api::{AppState, app};
use serde_json::Value;
use support::symbol;
use tower::ServiceExt;

pub mod support;

#[tokio::test]
async fn repo_changed_symbols_maps_diff_lines_to_innermost_symbols()
-> Result<(), Box<dyn std::error::Error>> {
    let app = app(AppState::for_test_symbols(vec![
        symbol("src/invoice.rs", "InvoiceService::apply_tax")?,
        symbol("src/other.rs", "OtherService::noop")?,
    ])?);
    let diff = "\
diff --git a/src/invoice.rs b/src/invoice.rs
--- a/src/invoice.rs
+++ b/src/invoice.rs
@@ -1,2 +1,2 @@
+fn apply_tax() {}
 context
";
    let request = Request::builder()
        .method(Method::POST)
        .uri("/v1/repos/local/changed-symbols")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::json!({ "diff": diff }).to_string()))?;

    let response = app.oneshot(request).await?;

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), 1_000_000).await?;
    let body = serde_json::from_slice::<Value>(&bytes)?;
    assert_eq!(body.pointer("/status").and_then(Value::as_str), Some("ok"));
    assert_eq!(
        body.pointer("/kind").and_then(Value::as_str),
        Some("changed_symbols")
    );
    assert_eq!(
        body.pointer("/repo_id").and_then(Value::as_str),
        Some("local")
    );
    assert_eq!(
        body.pointer("/changed_line_count").and_then(Value::as_u64),
        Some(1)
    );
    assert_eq!(
        body.pointer("/matched_symbol_count")
            .and_then(Value::as_u64),
        Some(1)
    );
    assert_eq!(
        body.pointer("/changed_symbols/0/symbol/fqn")
            .and_then(Value::as_str),
        Some("InvoiceService::apply_tax")
    );
    assert_eq!(
        body.pointer("/changed_file_count").and_then(Value::as_u64),
        Some(1)
    );
    assert_eq!(
        body.pointer("/changed_files/0/path")
            .and_then(Value::as_str),
        Some("src/invoice.rs")
    );
    assert_eq!(
        body.pointer("/changed_files/0/status")
            .and_then(Value::as_str),
        Some("modified")
    );
    Ok(())
}

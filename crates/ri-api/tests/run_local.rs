#![allow(missing_docs, reason = "Integration test names document behavior.")]

use axum::{
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode},
};
use ri_api::{AppState, app};
use serde_json::Value;
use tower::ServiceExt;

include!("support/local_temp_repo.rs");

#[tokio::test]
async fn get_run_returns_local_evidence_without_database() -> Result<(), Box<dyn std::error::Error>>
{
    let repo = LocalTempRepo::create("source-prism-api-run-local")?;
    repo.write_file(
        "src/lib.rs",
        r"
pub fn local_run_fixture() -> i32 {
    7
}

#[test]
fn local_run_fixture_is_indexed() {
    assert_eq!(local_run_fixture(), 7);
}
",
    )?;
    repo.commit()?;
    let app = app(AppState::for_test_repo_path(repo.path().to_path_buf())?);
    let listing_request = Request::builder()
        .method(Method::GET)
        .uri("/v1/repos/local/runs")
        .body(Body::empty())?;
    let listing_response = app.clone().oneshot(listing_request).await?;
    assert_eq!(listing_response.status(), StatusCode::OK);
    let listing_bytes = to_bytes(listing_response.into_body(), 1_000_000).await?;
    let runs = serde_json::from_slice::<Value>(&listing_bytes)?;
    let run_id = runs
        .pointer("/runs/0/run_id")
        .and_then(Value::as_str)
        .ok_or("missing local run id")?;

    let detail_request = Request::builder()
        .method(Method::GET)
        .uri(format!("/v1/runs/{run_id}"))
        .body(Body::empty())?;
    let detail_response = app.oneshot(detail_request).await?;

    assert_eq!(detail_response.status(), StatusCode::OK);
    let detail_bytes = to_bytes(detail_response.into_body(), 1_000_000).await?;
    let run = serde_json::from_slice::<Value>(&detail_bytes)?;
    assert_eq!(run.pointer("/kind").and_then(Value::as_str), Some("run"));
    assert_eq!(
        run.pointer("/run/run_id").and_then(Value::as_str),
        Some(run_id)
    );
    assert_eq!(
        run.pointer("/run/status").and_then(Value::as_str),
        Some("succeeded")
    );
    assert_eq!(
        run.pointer("/run/evidence/file_manifests")
            .and_then(Value::as_i64),
        Some(1)
    );
    assert_count_at_least(&run, "/run/evidence/symbols", 2)?;
    assert_count_at_least(&run, "/run/evidence/test_cases", 1)?;
    repo.cleanup()?;
    Ok(())
}

fn assert_count_at_least(
    body: &Value,
    pointer: &str,
    minimum: i64,
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(value) = body.pointer(pointer).and_then(Value::as_i64) else {
        return Err(format!("missing count at {pointer}").into());
    };
    if value < minimum {
        return Err(format!("count at {pointer} was {value}, expected at least {minimum}").into());
    }
    Ok(())
}

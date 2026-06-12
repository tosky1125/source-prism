use super::RunEvidence;
use crate::run_jobs::{RunSearchSyncJob, RunSearchSyncJobAttempt};
use crate::run_outbox::{RunSearchSyncOutboxItem, RunSearchSyncOutboxStateCounts};
use serde_json::Value;

#[test]
fn run_evidence_serializes_search_sync_job_details() -> Result<(), serde_json::Error> {
    let evidence = RunEvidence {
        file_manifests: 1,
        symbols: 2,
        graph_nodes: 3,
        graph_edges: 4,
        search_chunks: 5,
        search_sync_outbox_details: vec![RunSearchSyncOutboxItem {
            outbox_id: "outbox-1".to_owned(),
            entity_type: "symbol_chunk".to_owned(),
            entity_id: "chunk-1".to_owned(),
            operation: "upsert".to_owned(),
            target_index: "source-prism".to_owned(),
            state: "queued".to_owned(),
            attempt_count: 0,
            processed_at: None,
            last_error: None,
        }],
        search_sync_outbox_state_counts: RunSearchSyncOutboxStateCounts {
            queued: 1,
            leased: 0,
            succeeded: 0,
            failed: 0,
            dead_lettered: 0,
            cancelled: 0,
            total: 1,
        },
        search_sync_jobs: 1,
        search_sync_job_details: vec![RunSearchSyncJob {
            job_id: "job-1".to_owned(),
            state: "queued".to_owned(),
            attempt_count: 0,
            attempts: vec![RunSearchSyncJobAttempt {
                attempt_no: 1,
                worker_id: "worker-1".to_owned(),
                status: "started".to_owned(),
                error: None,
                started_at: "2026-06-12 00:00:00+00".to_owned(),
                finished_at: None,
            }],
        }],
        test_cases: 6,
        test_runs: 7,
        coverage_segments: 8,
        architecture_entities: 9,
    };

    let body = serde_json::to_value(evidence)?;

    assert_eq!(
        body.pointer("/search_sync_job_details/0/job_id")
            .and_then(Value::as_str),
        Some("job-1")
    );
    assert_eq!(
        body.pointer("/search_sync_job_details/0/state")
            .and_then(Value::as_str),
        Some("queued")
    );
    assert_eq!(
        body.pointer("/search_sync_job_details/0/attempts/0/status")
            .and_then(Value::as_str),
        Some("started")
    );
    assert_eq!(
        body.pointer("/search_sync_outbox_details/0/state")
            .and_then(Value::as_str),
        Some("queued")
    );
    assert_eq!(
        body.pointer("/search_sync_outbox_state_counts/queued")
            .and_then(Value::as_i64),
        Some(1)
    );
    Ok(())
}

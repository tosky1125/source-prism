use axum::{
    Json,
    extract::{Path, State},
};
use ri_indexer::{DEFAULT_SEARCH_INDEX, DriftReport, OpenSearchClient, PgSearchSyncStore};
use serde::Serialize;
use sqlx::{PgPool, Row as _};

use crate::{AppError, local_index::local_index_summary, state::AppState};

#[derive(Debug, Serialize)]
pub(crate) struct RepoSearchDriftResponse {
    status: &'static str,
    kind: &'static str,
    repo_id: String,
    latest_generation_id: Option<String>,
    expected_documents: i64,
    actual_documents: i64,
    has_drift: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    remediation: Option<SearchDriftRemediation>,
}

#[derive(Debug, Serialize)]
struct SearchDriftRemediation {
    summary: &'static str,
    rebuild_command: String,
    verify_command: String,
    steps: Vec<&'static str>,
}

pub(crate) async fn get(
    State(state): State<AppState>,
    Path(repo_id): Path<String>,
) -> Result<Json<RepoSearchDriftResponse>, AppError> {
    let Some(pool) = state.database.pool.as_ref() else {
        return Ok(Json(local_response(&state, repo_id)?));
    };
    ensure_repo_exists(pool, &repo_id).await?;
    let opensearch_url = state
        .opensearch_url
        .as_deref()
        .ok_or(AppError::OpenSearchNotConfigured)?;
    let Some(generation_id) = latest_generation_id(pool, &repo_id).await? else {
        return Ok(Json(RepoSearchDriftResponse {
            status: "ok",
            kind: "repo_search_drift",
            repo_id,
            latest_generation_id: None,
            expected_documents: 0,
            actual_documents: 0,
            has_drift: false,
            remediation: None,
        }));
    };
    let report = PgSearchSyncStore::new(pool.clone())
        .drift_report_for_repo_generation(
            &OpenSearchClient::new(opensearch_url),
            DEFAULT_SEARCH_INDEX,
            &repo_id,
            &generation_id,
        )
        .await?;
    Ok(Json(response_for_report(repo_id, &generation_id, &report)))
}

fn local_response(state: &AppState, repo_id: String) -> Result<RepoSearchDriftResponse, AppError> {
    let local = local_index_summary(state, &repo_id)?;
    Ok(RepoSearchDriftResponse {
        status: "ok",
        kind: "repo_search_drift",
        repo_id,
        latest_generation_id: Some(local.run_id),
        expected_documents: 0,
        actual_documents: 0,
        has_drift: false,
        remediation: None,
    })
}

fn response_for_report(
    repo_id: String,
    generation_id: &str,
    report: &DriftReport,
) -> RepoSearchDriftResponse {
    response_for_counts(
        repo_id,
        generation_id,
        report.expected_documents,
        report.actual_documents,
    )
}

fn response_for_counts(
    repo_id: String,
    generation_id: &str,
    expected_documents: i64,
    actual_documents: i64,
) -> RepoSearchDriftResponse {
    let has_drift = expected_documents != actual_documents;
    RepoSearchDriftResponse {
        status: "ok",
        kind: "repo_search_drift",
        repo_id,
        latest_generation_id: Some(generation_id.to_owned()),
        expected_documents,
        actual_documents,
        has_drift,
        remediation: has_drift.then(|| remediation_for_generation(generation_id)),
    }
}

fn remediation_for_generation(generation_id: &str) -> SearchDriftRemediation {
    SearchDriftRemediation {
        summary: "OpenSearch index is out of sync with Postgres canonical rows.",
        rebuild_command: format!(
            "ri-cli search rebuild --from-postgres --generation {generation_id}"
        ),
        verify_command: format!("ri-cli search drift-check --generation {generation_id}"),
        steps: vec![
            "Rebuild the OpenSearch index from Postgres.",
            "Run the search sync worker if queued jobs remain.",
            "Run drift-check again for the same generation.",
        ],
    }
}

async fn ensure_repo_exists(pool: &PgPool, repo_id: &str) -> Result<(), AppError> {
    let exists = sqlx::query_scalar::<_, bool>(
        r"
        SELECT EXISTS(
            SELECT 1 FROM repos WHERE repo_id = $1
        )
        ",
    )
    .bind(repo_id)
    .fetch_one(pool)
    .await?;
    if exists {
        Ok(())
    } else {
        Err(AppError::RepoNotFound {
            repo_id: repo_id.to_owned(),
        })
    }
}

async fn latest_generation_id(pool: &PgPool, repo_id: &str) -> Result<Option<String>, sqlx::Error> {
    sqlx::query(
        r"
        SELECT generation_id
        FROM index_generations
        WHERE repo_id = $1
        ORDER BY started_at DESC, generation_id DESC
        LIMIT 1
        ",
    )
    .bind(repo_id)
    .fetch_optional(pool)
    .await?
    .map(|row| row.try_get("generation_id"))
    .transpose()
}

#[cfg(test)]
mod tests {
    use super::response_for_counts;

    #[test]
    fn drift_response_includes_rebuild_guidance_when_counts_differ() {
        let response = response_for_counts("repo".to_owned(), "generation-1", 3, 2);

        let remediation = response
            .remediation
            .expect("drift response must include remediation");
        assert!(response.has_drift);
        assert_eq!(
            remediation.rebuild_command,
            "ri-cli search rebuild --from-postgres --generation generation-1"
        );
        assert_eq!(
            remediation.verify_command,
            "ri-cli search drift-check --generation generation-1"
        );
        assert_eq!(remediation.steps.len(), 3);
    }

    #[test]
    fn drift_response_omits_rebuild_guidance_when_counts_match() {
        let response = response_for_counts("repo".to_owned(), "generation-1", 2, 2);

        assert!(!response.has_drift);
        assert!(response.remediation.is_none());
    }
}

use axum::{Json, extract::State, http::StatusCode};
use serde::Serialize;

use crate::state::{AppState, DatabaseState};

const SERVICE_NAME: &str = "source-prism-api";

#[derive(Debug, Serialize)]
pub(crate) struct HealthResponse {
    service: &'static str,
    version: &'static str,
    status: OverallStatus,
    database: DependencyHealth,
    opensearch: DependencyHealth,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum OverallStatus {
    Ok,
    Degraded,
    Unhealthy,
}

#[derive(Debug, Serialize)]
pub(crate) struct DependencyHealth {
    status: DependencyStatus,
    configured: bool,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DependencyStatus {
    Ok,
    NotConfigured,
    Unhealthy,
}

pub(crate) async fn health(State(state): State<AppState>) -> (StatusCode, Json<HealthResponse>) {
    let database = database_health(&state.database).await;
    let opensearch = opensearch_health(&state.http_client, state.opensearch_url.as_deref()).await;
    let status = overall_status(&database, &opensearch);
    let status_code = match status {
        OverallStatus::Ok | OverallStatus::Degraded => StatusCode::OK,
        OverallStatus::Unhealthy => StatusCode::SERVICE_UNAVAILABLE,
    };
    (
        status_code,
        Json(HealthResponse {
            service: SERVICE_NAME,
            version: env!("CARGO_PKG_VERSION"),
            status,
            database,
            opensearch,
        }),
    )
}

async fn database_health(database: &DatabaseState) -> DependencyHealth {
    let Some(pool) = database.pool.as_ref() else {
        let status = if database.configured {
            DependencyStatus::Unhealthy
        } else {
            DependencyStatus::NotConfigured
        };
        return DependencyHealth {
            status,
            configured: database.configured,
        };
    };
    let status = if sqlx::query("SELECT 1").execute(pool).await.is_ok() {
        DependencyStatus::Ok
    } else {
        DependencyStatus::Unhealthy
    };
    DependencyHealth {
        status,
        configured: true,
    }
}

async fn opensearch_health(client: &reqwest::Client, url: Option<&str>) -> DependencyHealth {
    let Some(url) = url else {
        return DependencyHealth {
            status: DependencyStatus::NotConfigured,
            configured: false,
        };
    };
    let health_url = format!("{}/_cluster/health", url.trim_end_matches('/'));
    let status = match client.get(health_url).send().await {
        Ok(response) if response.status().is_success() => DependencyStatus::Ok,
        Ok(_) | Err(_) => DependencyStatus::Unhealthy,
    };
    DependencyHealth {
        status,
        configured: true,
    }
}

fn overall_status(database: &DependencyHealth, opensearch: &DependencyHealth) -> OverallStatus {
    if database.status == DependencyStatus::Unhealthy
        || opensearch.status == DependencyStatus::Unhealthy
    {
        OverallStatus::Unhealthy
    } else if database.status == DependencyStatus::NotConfigured
        || opensearch.status == DependencyStatus::NotConfigured
    {
        OverallStatus::Degraded
    } else {
        OverallStatus::Ok
    }
}

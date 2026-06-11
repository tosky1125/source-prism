#![allow(missing_docs, reason = "Milestone scaffold exposes no public API yet.")]
#![allow(
    clippy::missing_const_for_fn,
    reason = "Binary entry points cannot be const fn."
)]
#![allow(
    clippy::multiple_crate_versions,
    reason = "SQLx and Reqwest TLS dependencies pull duplicate platform crates outside this crate's control."
)]

use axum::{Json, Router, extract::State, http::StatusCode, routing::get};
use serde::Serialize;
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::{env, net::SocketAddr, time::Duration};

const DEFAULT_BIND_ADDR: &str = "127.0.0.1:3000";
const SERVICE_NAME: &str = "source-prism-api";

#[derive(Debug, Clone)]
struct AppState {
    database: DatabaseState,
    opensearch_url: Option<String>,
    http_client: reqwest::Client,
}

#[derive(Debug, Clone)]
struct DatabaseState {
    configured: bool,
    pool: Option<PgPool>,
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    service: &'static str,
    version: &'static str,
    status: OverallStatus,
    database: DependencyHealth,
    opensearch: DependencyHealth,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum OverallStatus {
    Ok,
    Degraded,
    Unhealthy,
}

#[derive(Debug, Serialize)]
struct DependencyHealth {
    status: DependencyStatus,
    configured: bool,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum DependencyStatus {
    Ok,
    NotConfigured,
    Unhealthy,
}

#[derive(Debug, thiserror::Error)]
enum ApiError {
    #[error("invalid API bind address: {value}")]
    InvalidBindAddress {
        value: String,
        source: std::net::AddrParseError,
    },
    #[error(transparent)]
    Http(#[from] reqwest::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
}

#[tokio::main]
async fn main() -> Result<(), ApiError> {
    let bind_addr = bind_addr()?;
    let state = AppState {
        database: database_pool(),
        opensearch_url: env::var("OPENSEARCH_URL").ok(),
        http_client: reqwest::Client::builder()
            .timeout(Duration::from_secs(2))
            .build()?,
    };
    let app = Router::new()
        .route("/v1/health", get(health))
        .with_state(state);
    let listener = tokio::net::TcpListener::bind(bind_addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn health(State(state): State<AppState>) -> (StatusCode, Json<HealthResponse>) {
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

fn database_pool() -> DatabaseState {
    let Ok(database_url) = env::var("DATABASE_URL") else {
        return DatabaseState {
            configured: false,
            pool: None,
        };
    };
    let pool = PgPoolOptions::new()
        .max_connections(3)
        .connect_lazy(database_url.as_str())
        .ok();
    DatabaseState {
        configured: true,
        pool,
    }
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

fn bind_addr() -> Result<SocketAddr, ApiError> {
    let value = env::var("API_BIND_ADDR").unwrap_or_else(|_| DEFAULT_BIND_ADDR.to_owned());
    value
        .parse()
        .map_err(|source| ApiError::InvalidBindAddress { value, source })
}

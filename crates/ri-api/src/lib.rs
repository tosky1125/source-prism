#![allow(
    missing_docs,
    reason = "Milestone HTTP API surface is self-describing."
)]
#![allow(
    clippy::redundant_pub_crate,
    reason = "Axum route handlers expose crate-visible DTOs from internal route modules."
)]
#![allow(
    clippy::multiple_crate_versions,
    reason = "SQLx and Reqwest TLS dependencies pull duplicate platform crates outside this crate's control."
)]

pub(crate) mod context_search;
pub(crate) mod error;
pub(crate) mod graph_call_edges;
pub(crate) mod graph_test_edges;
pub(crate) mod health;
pub(crate) mod impact;
pub(crate) mod local_index;
pub(crate) mod rate_limit;
pub(crate) mod refactor;
pub(crate) mod repo_architecture;
pub(crate) mod repo_changed_symbols;
pub(crate) mod repo_coverage;
pub(crate) mod repo_dead_letters;
pub(crate) mod repo_files;
pub(crate) mod repo_graph;
pub(crate) mod repo_index;
pub(crate) mod repo_index_jobs;
pub(crate) mod repo_references;
pub(crate) mod repo_runs;
pub(crate) mod repo_search_drift;
pub(crate) mod repo_search_sync;
pub(crate) mod repo_symbols;
pub(crate) mod repo_test_runs;
pub(crate) mod repo_tests;
pub(crate) mod repos;
pub(crate) mod review;
pub(crate) mod run_jobs;
pub(crate) mod run_outbox;
pub(crate) mod runs;
pub(crate) mod state;
pub(crate) mod test_context;
pub(crate) mod web;

use axum::{
    Router,
    extract::DefaultBodyLimit,
    middleware,
    routing::{get, post},
};
use std::{env, net::SocketAddr, time::Duration};

pub use error::{ApiError, AppError};
pub use rate_limit::ApiRateLimit;
pub use repo_files::{RepoFile, RepoFileFlags};
pub use state::AppState;

const DEFAULT_BIND_ADDR: &str = "127.0.0.1:3000";
const API_RATE_LIMIT_REQUESTS: &str = "API_RATE_LIMIT_REQUESTS";
const API_RATE_LIMIT_WINDOW_SECONDS: &str = "API_RATE_LIMIT_WINDOW_SECONDS";
const DEFAULT_API_RATE_LIMIT_REQUESTS: u32 = 600;
const DEFAULT_API_RATE_LIMIT_WINDOW_SECONDS: u64 = 60;
const API_MAX_REQUEST_BODY_BYTES: usize = 256 * 1024;

pub fn app(state: AppState) -> Router {
    app_with_rate_limit(state, ApiRateLimit::default())
}

pub fn app_with_rate_limit(state: AppState, rate_limit: ApiRateLimit) -> Router {
    let limiter = rate_limit::ProcessRateLimiter::new(rate_limit);
    Router::new()
        .route("/v1/health", get(health::health))
        .route("/v1/context/search", post(context_search::search))
        .route("/v1/impact", post(impact::analyze))
        .route("/v1/refactor/plan", post(refactor::plan))
        .route("/v1/review/github-dry-run", post(review::github_dry_run))
        .route("/v1/review/gitlab-dry-run", post(review::gitlab_dry_run))
        .route("/v1/review/verify", post(review::verify))
        .route("/v1/test-context", post(test_context::get))
        .route("/repo/{repo_id}", get(web::repo))
        .route("/repo/{repo_id}/{view}", get(web::repo_view))
        .route("/v1/repos", post(repos::create))
        .route("/v1/repos/{repo_id}", get(repos::get))
        .route(
            "/v1/repos/{repo_id}/architecture",
            get(repo_architecture::list),
        )
        .route(
            "/v1/repos/{repo_id}/changed-symbols",
            post(repo_changed_symbols::map),
        )
        .route("/v1/repos/{repo_id}/coverage", get(repo_coverage::list))
        .route(
            "/v1/repos/{repo_id}/context/search",
            post(context_search::search_for_repo),
        )
        .route("/v1/repos/{repo_id}/files", get(repo_files::list))
        .route(
            "/v1/repos/{repo_id}/dead-letters",
            get(repo_dead_letters::list),
        )
        .route("/v1/repos/{repo_id}/graph", get(repo_graph::get))
        .route("/v1/repos/{repo_id}/impact", post(impact::analyze_for_repo))
        .route("/v1/repos/{repo_id}/index", post(repo_index::index))
        .route("/v1/repos/{repo_id}/references", get(repo_references::list))
        .route(
            "/v1/repos/{repo_id}/refactor/plan",
            post(refactor::plan_for_repo),
        )
        .route("/v1/repos/{repo_id}/runs", get(repo_runs::list))
        .route(
            "/v1/repos/{repo_id}/search-drift",
            get(repo_search_drift::get),
        )
        .route(
            "/v1/repos/{repo_id}/search-sync",
            get(repo_search_sync::get),
        )
        .route("/v1/repos/{repo_id}/symbols", get(repo_symbols::list))
        .route(
            "/v1/repos/{repo_id}/test-context",
            get(test_context::get_for_repo),
        )
        .route("/v1/repos/{repo_id}/tests", get(repo_tests::list))
        .route("/v1/repos/{repo_id}/test-runs", get(repo_test_runs::list))
        .route("/v1/runs/{run_id}", get(runs::get))
        .layer(DefaultBodyLimit::max(API_MAX_REQUEST_BODY_BYTES))
        .layer(middleware::from_fn_with_state(limiter, rate_limit::enforce))
        .with_state(state)
}

pub fn state_from_env() -> Result<AppState, ApiError> {
    AppState::from_env()
}

pub fn bind_addr() -> Result<SocketAddr, ApiError> {
    let value = env::var("API_BIND_ADDR").unwrap_or_else(|_| DEFAULT_BIND_ADDR.to_owned());
    parse_bind_addr(&value)
}

pub fn rate_limit_from_env() -> Result<ApiRateLimit, ApiError> {
    let requests = optional_positive_u32(API_RATE_LIMIT_REQUESTS, DEFAULT_API_RATE_LIMIT_REQUESTS)?;
    let window_seconds = optional_positive_u64(
        API_RATE_LIMIT_WINDOW_SECONDS,
        DEFAULT_API_RATE_LIMIT_WINDOW_SECONDS,
    )?;
    ApiRateLimit::new(requests, Duration::from_secs(window_seconds))
}

pub fn parse_bind_addr(value: &str) -> Result<SocketAddr, ApiError> {
    value
        .parse()
        .map_err(|source| ApiError::InvalidBindAddress {
            value: value.to_owned(),
            source,
        })
        .and_then(|bind_addr: SocketAddr| {
            if bind_addr.ip().is_loopback() {
                Ok(bind_addr)
            } else {
                Err(ApiError::PublicBindAddress { bind_addr })
            }
        })
}

fn optional_positive_u32(key: &'static str, default: u32) -> Result<u32, ApiError> {
    match env::var(key) {
        Ok(value) => value
            .parse::<u32>()
            .ok()
            .filter(|parsed| *parsed > 0)
            .ok_or(ApiError::InvalidRateLimitConfig { key, value }),
        Err(env::VarError::NotPresent) => Ok(default),
        Err(env::VarError::NotUnicode(_)) => Err(ApiError::InvalidUnicodeEnv { key }),
    }
}

fn optional_positive_u64(key: &'static str, default: u64) -> Result<u64, ApiError> {
    match env::var(key) {
        Ok(value) => value
            .parse::<u64>()
            .ok()
            .filter(|parsed| *parsed > 0)
            .ok_or(ApiError::InvalidRateLimitConfig { key, value }),
        Err(env::VarError::NotPresent) => Ok(default),
        Err(env::VarError::NotUnicode(_)) => Err(ApiError::InvalidUnicodeEnv { key }),
    }
}

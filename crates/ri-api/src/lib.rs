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
pub(crate) mod health;
pub(crate) mod impact;
pub(crate) mod repo_files;
pub(crate) mod repo_graph;
pub(crate) mod repo_index;
pub(crate) mod repo_symbols;
pub(crate) mod repos;
pub(crate) mod runs;
pub(crate) mod state;
pub(crate) mod test_context;

use axum::{
    Router,
    routing::{get, post},
};
use std::{env, net::SocketAddr};

pub use error::{ApiError, AppError};
pub use repo_files::{RepoFile, RepoFileFlags};
pub use state::AppState;

const DEFAULT_BIND_ADDR: &str = "127.0.0.1:3000";

pub fn app(state: AppState) -> Router {
    Router::new()
        .route("/v1/health", get(health::health))
        .route("/v1/context/search", post(context_search::search))
        .route("/v1/impact", post(impact::analyze))
        .route("/v1/test-context", post(test_context::get))
        .route("/v1/repos", post(repos::create))
        .route("/v1/repos/{repo_id}/files", get(repo_files::list))
        .route("/v1/repos/{repo_id}/graph", get(repo_graph::get))
        .route("/v1/repos/{repo_id}/index", post(repo_index::index))
        .route("/v1/repos/{repo_id}/symbols", get(repo_symbols::list))
        .route("/v1/runs/{run_id}", get(runs::get))
        .with_state(state)
}

pub fn state_from_env() -> Result<AppState, ApiError> {
    AppState::from_env()
}

pub fn bind_addr() -> Result<SocketAddr, ApiError> {
    let value = env::var("API_BIND_ADDR").unwrap_or_else(|_| DEFAULT_BIND_ADDR.to_owned());
    value
        .parse()
        .map_err(|source| ApiError::InvalidBindAddress { value, source })
}

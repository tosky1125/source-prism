#![allow(missing_docs, reason = "Error JSON contract is self-describing.")]

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use ri_behavior::BehaviorError;
use ri_context::ContextError;
use ri_core::CoreError;
use ri_impact::ImpactError;
use ri_indexer::{
    ArchitectureStoreError, CoverageStoreError, FileOverlayStoreError, GenerationError,
    GraphStoreError, SearchSyncError, SymbolStoreError, TestCaseStoreError, TestRunStoreError,
};
use ri_review::ReviewError;
use ri_worker::JobError;
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ApiError {
    #[error("invalid API bind address: {value}")]
    InvalidBindAddress {
        value: String,
        source: std::net::AddrParseError,
    },
    #[error("public API bind address requires auth/tenancy first: {bind_addr}")]
    PublicBindAddress { bind_addr: std::net::SocketAddr },
    #[error("invalid positive integer for API rate limit config {key}: {value}")]
    InvalidRateLimitConfig { key: &'static str, value: String },
    #[error("invalid unicode for env: {key}")]
    InvalidUnicodeEnv { key: &'static str },
    #[error(transparent)]
    Http(#[from] reqwest::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum AppError {
    #[error("validation: {0}")]
    Validation(String),
    #[error("database is not configured")]
    DatabaseNotConfigured,
    #[error("opensearch is not configured")]
    OpenSearchNotConfigured,
    #[error("file is too large to index: {path} size_bytes={size_bytes}")]
    FileTooLarge { path: String, size_bytes: u64 },
    #[error("run not found: {run_id}")]
    RunNotFound { run_id: String },
    #[error("repo not found: {repo_id}")]
    RepoNotFound { repo_id: String },
    #[error(transparent)]
    Architecture(#[from] ri_architecture::ArchitectureError),
    #[error(transparent)]
    ArchitectureStore(#[from] ArchitectureStoreError),
    #[error(transparent)]
    Behavior(#[from] BehaviorError),
    #[error(transparent)]
    Context(#[from] ri_context::ContextError),
    #[error(transparent)]
    Core(#[from] CoreError),
    #[error(transparent)]
    Git(#[from] ri_git::GitError),
    #[error(transparent)]
    Impact(#[from] ImpactError),
    #[error(transparent)]
    Job(#[from] JobError),
    #[error(transparent)]
    Review(#[from] ReviewError),
    #[error(transparent)]
    Generation(#[from] GenerationError),
    #[error(transparent)]
    FileOverlayStore(#[from] FileOverlayStoreError),
    #[error(transparent)]
    GraphStore(#[from] GraphStoreError),
    #[error(transparent)]
    SearchSync(#[from] SearchSyncError),
    #[error(transparent)]
    SymbolStore(#[from] SymbolStoreError),
    #[error(transparent)]
    TestCaseStore(#[from] TestCaseStoreError),
    #[error(transparent)]
    TestRunStore(#[from] TestRunStoreError),
    #[error(transparent)]
    CoverageStore(#[from] CoverageStoreError),
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            Self::Validation(message) => {
                parts(StatusCode::UNPROCESSABLE_ENTITY, "validation", message)
            }
            Self::DatabaseNotConfigured => parts(
                StatusCode::SERVICE_UNAVAILABLE,
                "database_not_configured",
                "database is not configured",
            ),
            Self::OpenSearchNotConfigured => parts(
                StatusCode::SERVICE_UNAVAILABLE,
                "opensearch_not_configured",
                "opensearch is not configured",
            ),
            Self::FileTooLarge { path, size_bytes } => parts(
                StatusCode::PAYLOAD_TOO_LARGE,
                "file_too_large",
                format!("file is too large to index: {path} size_bytes={size_bytes}"),
            ),
            Self::RunNotFound { run_id } => parts(
                StatusCode::NOT_FOUND,
                "run_not_found",
                format!("run not found: {run_id}"),
            ),
            Self::RepoNotFound { repo_id } => parts(
                StatusCode::NOT_FOUND,
                "repo_not_found",
                format!("repo not found: {repo_id}"),
            ),
            Self::Behavior(BehaviorError::SymbolNotFound { symbol: query })
            | Self::Context(ContextError::SymbolNotFound { query })
            | Self::Impact(ImpactError::SymbolNotFound { query }) => parts(
                StatusCode::NOT_FOUND,
                "symbol_not_found",
                format!("symbol not found: {query}"),
            ),
            Self::Review(error) => parts(
                StatusCode::UNPROCESSABLE_ENTITY,
                "review_verification_failed",
                error.to_string(),
            ),
            Self::Architecture(_) => internal("architecture", "architecture extraction failed"),
            Self::ArchitectureStore(_) => {
                internal("architecture_store", "architecture store failed")
            }
            Self::Behavior(_) => internal("behavior", "behavior context failed"),
            Self::Context(_) => internal("context", "context search failed"),
            Self::Core(_) => internal("core", "core domain validation failed"),
            Self::Git(_) => internal("manifest", "file manifest failed"),
            Self::Impact(_) => internal("impact", "impact analysis failed"),
            Self::Job(_) => internal("jobs", "job query failed"),
            Self::Generation(_) => internal("index_generation", "index generation failed"),
            Self::FileOverlayStore(_) => internal("file_overlay", "file overlay failed"),
            Self::GraphStore(_) => internal("graph_store", "graph store failed"),
            Self::SearchSync(_) => internal("search_sync", "search sync failed"),
            Self::SymbolStore(_) => internal("symbol_store", "symbol store failed"),
            Self::TestCaseStore(_) => internal("test_case_store", "test case store failed"),
            Self::TestRunStore(_) => internal("test_run_store", "test run store failed"),
            Self::CoverageStore(_) => internal("coverage_store", "coverage store failed"),
            Self::Sqlx(_) => internal("database", "database query failed"),
        };
        (status, Json(ErrorResponse::new(code, message))).into_response()
    }
}

fn internal(code: &'static str, message: &'static str) -> (StatusCode, &'static str, String) {
    parts(StatusCode::INTERNAL_SERVER_ERROR, code, message)
}

fn parts(
    status: StatusCode,
    code: &'static str,
    message: impl Into<String>,
) -> (StatusCode, &'static str, String) {
    (status, code, message.into())
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: ErrorBody,
}

impl ErrorResponse {
    const fn new(code: &'static str, message: String) -> Self {
        Self {
            error: ErrorBody { code, message },
        }
    }
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    code: &'static str,
    message: String,
}

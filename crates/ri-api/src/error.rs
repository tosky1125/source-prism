#![allow(missing_docs, reason = "Error JSON contract is self-describing.")]

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use ri_behavior::BehaviorError;
use ri_context::ContextError;
use ri_impact::ImpactError;
use ri_indexer::{
    GenerationError, GraphStoreError, SearchSyncError, SymbolStoreError, TestCaseStoreError,
};
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ApiError {
    #[error("invalid API bind address: {value}")]
    InvalidBindAddress {
        value: String,
        source: std::net::AddrParseError,
    },
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
    #[error("file is too large to index: {path} size_bytes={size_bytes}")]
    FileTooLarge { path: String, size_bytes: u64 },
    #[error("run not found: {run_id}")]
    RunNotFound { run_id: String },
    #[error(transparent)]
    Behavior(#[from] BehaviorError),
    #[error(transparent)]
    Context(#[from] ri_context::ContextError),
    #[error(transparent)]
    Git(#[from] ri_git::GitError),
    #[error(transparent)]
    Impact(#[from] ImpactError),
    #[error(transparent)]
    Generation(#[from] GenerationError),
    #[error(transparent)]
    GraphStore(#[from] GraphStoreError),
    #[error(transparent)]
    SearchSync(#[from] SearchSyncError),
    #[error(transparent)]
    SymbolStore(#[from] SymbolStoreError),
    #[error(transparent)]
    TestCaseStore(#[from] TestCaseStoreError),
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            Self::Validation(message) => (StatusCode::UNPROCESSABLE_ENTITY, "validation", message),
            Self::DatabaseNotConfigured => (
                StatusCode::SERVICE_UNAVAILABLE,
                "database_not_configured",
                "database is not configured".to_owned(),
            ),
            Self::FileTooLarge { path, size_bytes } => (
                StatusCode::PAYLOAD_TOO_LARGE,
                "file_too_large",
                format!("file is too large to index: {path} size_bytes={size_bytes}"),
            ),
            Self::RunNotFound { run_id } => (
                StatusCode::NOT_FOUND,
                "run_not_found",
                format!("run not found: {run_id}"),
            ),
            Self::Behavior(BehaviorError::SymbolNotFound { symbol: query })
            | Self::Context(ContextError::SymbolNotFound { query })
            | Self::Impact(ImpactError::SymbolNotFound { query }) => (
                StatusCode::NOT_FOUND,
                "symbol_not_found",
                format!("symbol not found: {query}"),
            ),
            Self::Behavior(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "behavior",
                "behavior context failed".to_owned(),
            ),
            Self::Context(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "context",
                "context search failed".to_owned(),
            ),
            Self::Git(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "manifest",
                "file manifest failed".to_owned(),
            ),
            Self::Impact(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "impact",
                "impact analysis failed".to_owned(),
            ),
            Self::Generation(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "index_generation",
                "index generation failed".to_owned(),
            ),
            Self::GraphStore(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "graph_store",
                "graph store failed".to_owned(),
            ),
            Self::SearchSync(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "search_sync",
                "search sync failed".to_owned(),
            ),
            Self::SymbolStore(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "symbol_store",
                "symbol store failed".to_owned(),
            ),
            Self::TestCaseStore(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "test_case_store",
                "test case store failed".to_owned(),
            ),
            Self::Sqlx(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "database",
                "database query failed".to_owned(),
            ),
        };
        (status, Json(ErrorResponse::new(code, message))).into_response()
    }
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

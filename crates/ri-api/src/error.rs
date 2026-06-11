#![allow(missing_docs, reason = "Error JSON contract is self-describing.")]

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
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
    #[error(transparent)]
    Context(#[from] ri_context::ContextError),
    #[error(transparent)]
    Git(#[from] ri_git::GitError),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            Self::Validation(message) => (StatusCode::UNPROCESSABLE_ENTITY, "validation", message),
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

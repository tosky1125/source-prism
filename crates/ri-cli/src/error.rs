#![allow(
    clippy::redundant_pub_crate,
    reason = "Binary crate helper modules share crate-visible types across sibling modules."
)]

use std::{io, process::ExitCode};

use thiserror::Error;

#[derive(Debug, Error)]
pub(crate) enum CliError {
    #[error(
        "usage: ri-cli config check --env-file <path> | db migrate | repo manifest --repo <path> | index --repo <path> --sha <sha> | symbols --repo <path> | changed-symbols --diff <diff> | impact --symbol <symbol> | search sync --once | search drift-check [--expect-mismatch fixture] | search rebuild --from-postgres"
    )]
    Usage,
    #[error("missing required env: {key}")]
    MissingEnv { key: &'static str },
    #[error("{0}")]
    Config(#[from] ri_config::ConfigError),
    #[error(transparent)]
    Core(#[from] ri_core::CoreError),
    #[error(transparent)]
    Git(#[from] ri_git::GitError),
    #[error("search drift detected: expected={expected} actual={actual}")]
    Drift { expected: i64, actual: i64 },
    #[error("file is too large to index: {path} size_bytes={size_bytes}")]
    FileTooLarge { path: String, size_bytes: u64 },
    #[error(transparent)]
    Generation(#[from] ri_indexer::GenerationError),
    #[error(transparent)]
    Parser(#[from] ri_parser::ParserError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Migrate(#[from] sqlx::migrate::MigrateError),
    #[error(transparent)]
    OpenSearch(#[from] ri_indexer::OpenSearchError),
    #[error(transparent)]
    SearchSync(#[from] ri_indexer::SearchSyncError),
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
    #[error(transparent)]
    Io(#[from] io::Error),
}

impl CliError {
    pub(crate) const fn exit_code(&self) -> ExitCode {
        match self {
            Self::Usage
            | Self::MissingEnv { .. }
            | Self::Config(_)
            | Self::Core(_)
            | Self::Git(_)
            | Self::Drift { .. }
            | Self::FileTooLarge { .. }
            | Self::Generation(_)
            | Self::Parser(_)
            | Self::Json(_)
            | Self::Migrate(_)
            | Self::OpenSearch(_)
            | Self::SearchSync(_)
            | Self::Sqlx(_)
            | Self::Io(_) => ExitCode::FAILURE,
        }
    }
}

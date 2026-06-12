#![allow(
    clippy::redundant_pub_crate,
    reason = "Binary crate helper modules share crate-visible types across sibling modules."
)]

use std::{io, process::ExitCode};

use thiserror::Error;

#[derive(Debug, Error)]
pub(crate) enum CliError {
    #[error(
        "usage: ri-cli config check --env-file <path> | db migrate | repo manifest --repo <path> | index --repo <path> --sha <sha> | symbols --repo <path> | changed-symbols --diff <diff> | references --symbol <symbol> | architecture --repo <path> | impact --symbol <symbol> | search-context <query> | test-context --symbol <symbol> | tests import-junit --repo <path> --sha <sha> --junit <file> | tests import-pytest-json --repo <path> --sha <sha> --pytest-json <file> | tests import-lcov --repo <path> --sha <sha> --lcov <file> | tests import-cobertura --repo <path> --sha <sha> --cobertura <file> | tests import-jacoco --repo <path> --sha <sha> --jacoco <file> | search sync --once | search drift-check [--expect-mismatch fixture] | search rebuild --from-postgres"
    )]
    Usage,
    #[error("missing required env: {key}")]
    MissingEnv { key: &'static str },
    #[error(transparent)]
    Behavior(#[from] ri_behavior::BehaviorError),
    #[error(transparent)]
    Architecture(#[from] ri_architecture::ArchitectureError),
    #[error("{0}")]
    Config(#[from] ri_config::ConfigError),
    #[error(transparent)]
    Context(#[from] ri_context::ContextError),
    #[error(transparent)]
    Core(#[from] ri_core::CoreError),
    #[error(transparent)]
    Git(#[from] ri_git::GitError),
    #[error(transparent)]
    Impact(#[from] ri_impact::ImpactError),
    #[error("search drift detected: expected={expected} actual={actual}")]
    Drift { expected: i64, actual: i64 },
    #[error("file is too large to index: {path} size_bytes={size_bytes}")]
    FileTooLarge { path: String, size_bytes: u64 },
    #[error(transparent)]
    Generation(#[from] ri_indexer::GenerationError),
    #[error(transparent)]
    Graph(#[from] ri_indexer::GraphStoreError),
    #[error(transparent)]
    ArchitectureStore(#[from] ri_indexer::ArchitectureStoreError),
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
    SymbolStore(#[from] ri_indexer::SymbolStoreError),
    #[error(transparent)]
    TestCaseStore(#[from] ri_indexer::TestCaseStoreError),
    #[error(transparent)]
    TestRunStore(#[from] ri_indexer::TestRunStoreError),
    #[error(transparent)]
    CoverageStore(#[from] ri_indexer::CoverageStoreError),
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
            | Self::Behavior(_)
            | Self::Architecture(_)
            | Self::Config(_)
            | Self::Context(_)
            | Self::Core(_)
            | Self::Git(_)
            | Self::Impact(_)
            | Self::Drift { .. }
            | Self::FileTooLarge { .. }
            | Self::Generation(_)
            | Self::Graph(_)
            | Self::ArchitectureStore(_)
            | Self::Parser(_)
            | Self::Json(_)
            | Self::Migrate(_)
            | Self::OpenSearch(_)
            | Self::SearchSync(_)
            | Self::SymbolStore(_)
            | Self::TestCaseStore(_)
            | Self::TestRunStore(_)
            | Self::CoverageStore(_)
            | Self::Sqlx(_)
            | Self::Io(_) => ExitCode::FAILURE,
        }
    }
}

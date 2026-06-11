#![allow(
    missing_docs,
    reason = "Parser boundary is intentionally small and self-describing."
)]

use ri_core::{CommitSha, FilePath, Language, RepoId};
use ri_symbols::SymbolRecord;
use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ParserError {
    #[error("unsupported language: {language:?}")]
    UnsupportedLanguage { language: Language },
    #[error("parse failed for {path}: {message}")]
    ParseFailed { path: String, message: String },
    #[error(transparent)]
    Core(#[from] ri_core::CoreError),
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct SourceFile<'source> {
    pub repo: RepoId,
    pub commit: CommitSha,
    pub path: FilePath,
    pub language: Language,
    pub content_hash: &'source str,
    pub source: &'source str,
}

impl<'source> SourceFile<'source> {
    pub const fn new(
        repo: RepoId,
        commit: CommitSha,
        path: FilePath,
        language: Language,
        content_hash: &'source str,
        source: &'source str,
    ) -> Self {
        Self {
            repo,
            commit,
            path,
            language,
            content_hash,
            source,
        }
    }
}

pub trait SymbolExtractor {
    fn extract_symbols(&self, file: &SourceFile<'_>) -> Result<Vec<SymbolRecord>, ParserError>;
}

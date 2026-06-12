use axum::{
    Json,
    extract::{Path, State},
};
use ri_core::Language;
use ri_git::FileManifest;
use ri_indexer::{FileManifestRecord, PgGenerationStore};
use serde::Serialize;

use crate::{AppError, state::AppState};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RepoFileFlags {
    is_generated: bool,
    is_vendor: bool,
    is_test: bool,
}

impl RepoFileFlags {
    pub const fn new(is_generated: bool, is_vendor: bool, is_test: bool) -> Self {
        Self {
            is_generated,
            is_vendor,
            is_test,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RepoFile {
    path: String,
    language: Language,
    size_bytes: u64,
    content_sha256: String,
    is_generated: bool,
    is_vendor: bool,
    is_test: bool,
}

impl RepoFile {
    pub fn new(
        path: impl Into<String>,
        language: Language,
        size_bytes: u64,
        content_sha256: impl Into<String>,
        flags: RepoFileFlags,
    ) -> Self {
        Self {
            path: path.into(),
            language,
            size_bytes,
            content_sha256: content_sha256.into(),
            is_generated: flags.is_generated,
            is_vendor: flags.is_vendor,
            is_test: flags.is_test,
        }
    }

    pub fn from_manifest(file: &FileManifest) -> Self {
        Self::new(
            file.path(),
            file.language(),
            file.size_bytes(),
            file.content_sha256(),
            RepoFileFlags::new(file.is_generated(), file.is_vendor(), file.is_test()),
        )
    }

    pub fn from_indexed(record: &FileManifestRecord) -> Result<Self, AppError> {
        let size_bytes = u64::try_from(record.size_bytes).map_err(|_| AppError::FileTooLarge {
            path: record.file_path.clone(),
            size_bytes: u64::MAX,
        })?;
        Ok(Self::new(
            &record.file_path,
            language_from_id(&record.language),
            size_bytes,
            &record.content_sha256,
            RepoFileFlags::new(record.is_generated, record.is_vendor, record.is_test),
        ))
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct RepoFilesResponse {
    status: &'static str,
    kind: &'static str,
    repo_id: String,
    file_count: usize,
    files: Vec<RepoFile>,
}

pub(crate) async fn list(
    State(state): State<AppState>,
    Path(repo_id): Path<String>,
) -> Result<Json<RepoFilesResponse>, AppError> {
    let files = repo_files(&state, &repo_id).await?;
    Ok(Json(RepoFilesResponse {
        status: "ok",
        kind: "files",
        repo_id,
        file_count: files.len(),
        files,
    }))
}

async fn repo_files(state: &AppState, repo_id: &str) -> Result<Vec<RepoFile>, AppError> {
    let Some(pool) = state.database.pool.as_ref() else {
        return Ok(state.repo_files()?.into_owned());
    };
    let records = PgGenerationStore::new(pool.clone())
        .active_file_manifests_for_repo(repo_id)
        .await?;
    records.iter().map(RepoFile::from_indexed).collect()
}

const fn language_from_id(language: &str) -> Language {
    match language.as_bytes() {
        b"typescript" => Language::TypeScript,
        b"javascript" => Language::JavaScript,
        b"php" => Language::Php,
        b"python" => Language::Python,
        b"java" => Language::Java,
        b"go" => Language::Go,
        b"rust" => Language::Rust,
        _ => Language::Unknown,
    }
}

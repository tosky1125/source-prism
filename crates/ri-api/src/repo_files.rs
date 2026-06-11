use axum::{
    Json,
    extract::{Path, State},
};
use ri_core::Language;
use ri_git::FileManifest;
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
    let files = state.repo_files()?.into_owned();
    Ok(Json(RepoFilesResponse {
        status: "ok",
        kind: "files",
        repo_id,
        file_count: files.len(),
        files,
    }))
}

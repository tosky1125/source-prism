use axum::{
    Json,
    extract::{Path, State},
};
use ri_core::Language;
use ri_git::LocalManifest;
use ri_indexer::{FileOverlayInput, FileOverlayStatus, PgFileOverlayStore, PgSymbolStore};
use ri_symbols::{
    ChangedFile, ChangedFileStatus, ChangedSymbol, changed_symbols_for_diff, parse_changed_files,
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::Path as FsPath};

use crate::{AppError, state::AppState};

#[derive(Debug, Deserialize)]
pub(crate) struct ChangedSymbolsRequest {
    diff: String,
    #[serde(default)]
    persist_overlay: bool,
    head_sha: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct ChangedSymbolsResponse {
    status: &'static str,
    kind: &'static str,
    repo_id: String,
    changed_file_count: usize,
    changed_line_count: usize,
    matched_symbol_count: usize,
    overlay_index: Option<OverlayIndexResponse>,
    changed_files: Vec<ChangedFile>,
    changed_symbols: Vec<ChangedSymbol>,
}

#[derive(Debug, Serialize)]
pub(crate) struct OverlayIndexResponse {
    base_generation_id: String,
    base_commit_sha: String,
    head_sha: String,
    indexed_file_count: u64,
}

pub(crate) async fn map(
    State(state): State<AppState>,
    Path(repo_id): Path<String>,
    Json(request): Json<ChangedSymbolsRequest>,
) -> Result<Json<ChangedSymbolsResponse>, AppError> {
    let symbols = if let Some(pool) = state.database.pool.as_ref() {
        PgSymbolStore::new(pool.clone())
            .active_symbols_for_repo(&repo_id)
            .await?
    } else {
        state.context_symbols()?.into_owned()
    };
    let changed_files = parse_changed_files(request.diff.as_str());
    let (changed_lines, changed_symbols) =
        changed_symbols_for_diff(symbols.as_slice(), request.diff.as_str());
    let overlay_index = if request.persist_overlay {
        let pool = state
            .database
            .pool
            .as_ref()
            .ok_or(AppError::DatabaseNotConfigured)?;
        let store = PgFileOverlayStore::new(pool.clone());
        let base = store.latest_base_generation(&repo_id).await?;
        let head_sha = request
            .head_sha
            .unwrap_or_else(|| "working-tree".to_owned());
        let inputs = overlay_inputs(state.context_repo_path(), changed_files.as_slice())?;
        let indexed = store
            .replace_overlay(&repo_id, &base, head_sha.as_str(), inputs.as_slice())
            .await?;
        Some(OverlayIndexResponse {
            base_generation_id: base.generation_id,
            base_commit_sha: base.commit_sha,
            head_sha,
            indexed_file_count: indexed,
        })
    } else {
        None
    };
    Ok(Json(ChangedSymbolsResponse {
        status: "ok",
        kind: "changed_symbols",
        repo_id,
        changed_file_count: changed_files.len(),
        changed_line_count: changed_lines.len(),
        matched_symbol_count: changed_symbols.len(),
        overlay_index,
        changed_files,
        changed_symbols,
    }))
}

fn overlay_inputs(
    head_repo: &FsPath,
    changed_files: &[ChangedFile],
) -> Result<Vec<FileOverlayInput>, AppError> {
    let manifest = LocalManifest::extract(head_repo)?;
    let files_by_path = manifest
        .files()
        .iter()
        .map(|file| (file.path().to_owned(), file))
        .collect::<BTreeMap<_, _>>();
    let mut inputs = Vec::with_capacity(changed_files.len());
    for changed in changed_files {
        let mut input = FileOverlayInput::new(
            changed.path.as_str(),
            changed.previous_path.clone(),
            overlay_status(changed.status),
        );
        if !matches!(changed.status, ChangedFileStatus::Deleted) {
            if let Some(file) = files_by_path.get(changed.path.as_str()) {
                input.content_sha256 = Some(file.content_sha256().to_owned());
                input.size_bytes =
                    Some(
                        i64::try_from(file.size_bytes()).map_err(|_| AppError::FileTooLarge {
                            path: file.path().to_owned(),
                            size_bytes: file.size_bytes(),
                        })?,
                    );
                language_id(file.language()).clone_into(&mut input.language);
            }
        }
        inputs.push(input);
    }
    Ok(inputs)
}

const fn overlay_status(status: ChangedFileStatus) -> FileOverlayStatus {
    match status {
        ChangedFileStatus::Added => FileOverlayStatus::Added,
        ChangedFileStatus::Deleted => FileOverlayStatus::Deleted,
        ChangedFileStatus::Renamed => FileOverlayStatus::Renamed,
        ChangedFileStatus::ModeOnly => FileOverlayStatus::ModeOnly,
        _ => FileOverlayStatus::Modified,
    }
}

const fn language_id(language: Language) -> &'static str {
    match language {
        Language::TypeScript => "typescript",
        Language::JavaScript => "javascript",
        Language::Php => "php",
        Language::Python => "python",
        Language::Java => "java",
        Language::Go => "go",
        Language::Rust => "rust",
        _ => "unknown",
    }
}

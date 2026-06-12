#![allow(
    clippy::redundant_pub_crate,
    reason = "Binary crate helper modules share crate-visible command handlers."
)]

use std::{
    collections::BTreeMap,
    env, fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

use ri_core::Language;
use ri_git::LocalManifest;
use ri_indexer::{FileOverlayInput, FileOverlayStatus, PgFileOverlayStore, PgSymbolStore};
use ri_symbols::{
    ChangedFile, ChangedFileStatus, SymbolRecord, changed_symbols_for_diff, parse_changed_files,
};
use serde_json::json;
use sqlx::{PgPool, postgres::PgPoolOptions};

use crate::{
    error::CliError,
    symbols::{extract_repo_symbols, symbol_json},
};

pub(crate) async fn changed_symbols_command(
    mut args: impl Iterator<Item = String>,
) -> Result<(), CliError> {
    let request = ChangedSymbolsArgs::parse(&mut args)?;
    let diff = fs::read_to_string(&request.diff_path)?;
    let (repo_id, symbols) = match &request.source {
        ChangedSymbolsSource::Worktree(repo) => (None, extract_repo_symbols(repo)?),
        ChangedSymbolsSource::PersistedRepo(repo_id) => {
            let symbols = persisted_symbols(repo_id).await?;
            (Some(repo_id.as_str()), symbols)
        }
    };
    let changed_files = parse_changed_files(&diff);
    let (changed_lines, changed_symbols) = changed_symbols_for_diff(&symbols, &diff);
    let overlay_index = if request.persist_overlay {
        let repo_id = repo_id.ok_or(CliError::Usage)?;
        Some(
            persist_overlay_index(
                repo_id,
                &request
                    .head_repo_path
                    .clone()
                    .unwrap_or_else(|| PathBuf::from(".")),
                request.head_sha.as_deref().unwrap_or("working-tree"),
                changed_files.as_slice(),
            )
            .await?,
        )
    } else {
        None
    };
    let changed_symbols = changed_symbols
        .iter()
        .map(|changed| {
            json!({
                "file_path": changed.file_path,
                "line": changed.line,
                "symbol": symbol_json(&changed.symbol),
            })
        })
        .collect::<Vec<_>>();

    print_json(&json!({
        "status": "ok",
        "kind": "changed_symbols",
        "repo_id": repo_id,
        "changed_file_count": changed_files.len(),
        "changed_line_count": changed_lines.len(),
        "matched_symbol_count": changed_symbols.len(),
        "changed_files": changed_files,
        "changed_symbols": changed_symbols,
        "overlay_index": overlay_index,
    }))
}

#[derive(Debug)]
struct ChangedSymbolsArgs {
    source: ChangedSymbolsSource,
    diff_path: PathBuf,
    head_repo_path: Option<PathBuf>,
    head_sha: Option<String>,
    persist_overlay: bool,
}

#[derive(Debug)]
enum ChangedSymbolsSource {
    Worktree(PathBuf),
    PersistedRepo(String),
}

impl ChangedSymbolsArgs {
    fn parse(args: &mut impl Iterator<Item = String>) -> Result<Self, CliError> {
        let mut source = None::<ChangedSymbolsSource>;
        let mut diff_path = None::<PathBuf>;
        let mut head_repo_path = None::<PathBuf>;
        let mut head_sha = None::<String>;
        let mut persist_overlay = false;

        while let Some(flag) = args.next() {
            match flag.as_str() {
                "--repo" => {
                    let path = args.next().ok_or(CliError::Usage)?;
                    set_source(
                        &mut source,
                        ChangedSymbolsSource::Worktree(PathBuf::from(path)),
                    )?;
                }
                "--repo-id" => {
                    let repo_id = args.next().ok_or(CliError::Usage)?;
                    set_source(&mut source, ChangedSymbolsSource::PersistedRepo(repo_id))?;
                }
                "--diff" => {
                    let path = args.next().ok_or(CliError::Usage)?;
                    diff_path = Some(PathBuf::from(path));
                }
                "--head-repo" => {
                    let path = args.next().ok_or(CliError::Usage)?;
                    head_repo_path = Some(PathBuf::from(path));
                }
                "--head-sha" => {
                    head_sha = Some(args.next().ok_or(CliError::Usage)?);
                }
                "--persist-overlay" => {
                    persist_overlay = true;
                }
                _ => return Err(CliError::Usage),
            }
        }

        if persist_overlay && !matches!(source, Some(ChangedSymbolsSource::PersistedRepo(_))) {
            return Err(CliError::Usage);
        }

        Ok(Self {
            source: source.unwrap_or_else(|| ChangedSymbolsSource::Worktree(PathBuf::from("."))),
            diff_path: diff_path.ok_or(CliError::Usage)?,
            head_repo_path,
            head_sha,
            persist_overlay,
        })
    }
}

fn set_source(
    current: &mut Option<ChangedSymbolsSource>,
    next: ChangedSymbolsSource,
) -> Result<(), CliError> {
    if current.is_some() {
        return Err(CliError::Usage);
    }
    *current = Some(next);
    Ok(())
}

async fn persisted_symbols(repo_id: &str) -> Result<Vec<SymbolRecord>, CliError> {
    let pool = database_pool().await?;
    Ok(PgSymbolStore::new(pool)
        .active_symbols_for_repo(repo_id)
        .await?)
}

async fn persist_overlay_index(
    repo_id: &str,
    head_repo: &Path,
    head_sha: &str,
    changed_files: &[ChangedFile],
) -> Result<serde_json::Value, CliError> {
    let pool = database_pool().await?;
    let store = PgFileOverlayStore::new(pool);
    let base = store.latest_base_generation(repo_id).await?;
    let inputs = overlay_inputs(head_repo, changed_files)?;
    let indexed = store
        .replace_overlay(repo_id, &base, head_sha, inputs.as_slice())
        .await?;
    Ok(json!({
        "base_generation_id": base.generation_id,
        "base_commit_sha": base.commit_sha,
        "head_sha": head_sha,
        "indexed_file_count": indexed,
    }))
}

async fn database_pool() -> Result<PgPool, CliError> {
    let database_url = env::var("DATABASE_URL").map_err(|_| CliError::MissingEnv {
        key: "DATABASE_URL",
    })?;
    PgPoolOptions::new()
        .max_connections(5)
        .connect(database_url.as_str())
        .await
        .map_err(CliError::from)
}

fn overlay_inputs(
    head_repo: &Path,
    changed_files: &[ChangedFile],
) -> Result<Vec<FileOverlayInput>, CliError> {
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
                        i64::try_from(file.size_bytes()).map_err(|_| CliError::FileTooLarge {
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

fn print_json(value: &serde_json::Value) -> Result<(), CliError> {
    let stdout = io::stdout();
    let mut lock = stdout.lock();
    serde_json::to_writer_pretty(&mut lock, value)?;
    writeln!(lock)?;
    Ok(())
}

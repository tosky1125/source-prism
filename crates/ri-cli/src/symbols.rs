#![allow(
    clippy::redundant_pub_crate,
    reason = "Binary crate helper modules share crate-visible command handlers."
)]

use std::{
    env, fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

use ri_core::{CommitSha, FilePath, Language, RepoId};
use ri_git::{LocalManifest, discover_worktree, resolve_commit_sha};
use ri_indexer::PgSymbolStore;
use ri_parser::{SourceFile, SymbolExtractor};
use ri_symbols::SymbolRecord;
use ri_tree_sitter::TreeSitterExtractor;
use serde_json::json;
use sqlx::postgres::PgPoolOptions;

use crate::error::CliError;

pub(crate) async fn symbols_command(
    mut args: impl Iterator<Item = String>,
) -> Result<(), CliError> {
    match SymbolArgs::parse(&mut args)? {
        SymbolArgs::Worktree(repo_path) => {
            let symbols = extract_repo_symbols(&repo_path)?;
            print_symbols(None, &symbols)
        }
        SymbolArgs::PersistedRepo(repo_id) => {
            let symbols = persisted_symbols(&repo_id).await?;
            print_symbols(Some(repo_id.as_str()), &symbols)
        }
    }
}

#[derive(Debug)]
enum SymbolArgs {
    Worktree(PathBuf),
    PersistedRepo(String),
}

impl SymbolArgs {
    fn parse(args: &mut impl Iterator<Item = String>) -> Result<Self, CliError> {
        let Some(flag) = args.next() else {
            return Err(CliError::Usage);
        };
        let Some(value) = args.next() else {
            return Err(CliError::Usage);
        };
        if args.next().is_some() {
            return Err(CliError::Usage);
        }

        match flag.as_str() {
            "--repo" => Ok(Self::Worktree(PathBuf::from(value))),
            "--repo-id" => Ok(Self::PersistedRepo(value)),
            _ => Err(CliError::Usage),
        }
    }
}

pub(crate) fn extract_repo_symbols(repo_path: &Path) -> Result<Vec<SymbolRecord>, CliError> {
    let worktree = discover_worktree(repo_path)?;
    let repo = RepoId::new(format!("local:{}", worktree.canonicalize()?.display()))?;
    let commit = CommitSha::new(resolve_commit_sha(repo_path, "HEAD")?)?;
    let manifest = LocalManifest::extract(repo_path)?;
    let extractor = TreeSitterExtractor::new();
    let mut symbols = Vec::new();

    for file in manifest.files() {
        if file.is_vendor() || file.is_generated() || !is_supported_language(file.language()) {
            continue;
        }
        let path = worktree.join(file.path());
        let source = fs::read_to_string(path)?;
        let source_file = SourceFile::new(
            repo.clone(),
            commit.clone(),
            FilePath::new(file.path())?,
            file.language(),
            file.content_sha256(),
            source.as_str(),
        );
        symbols.extend(extractor.extract_symbols(&source_file)?);
    }
    symbols.sort_by(|left, right| {
        left.file_path
            .cmp(&right.file_path)
            .then(left.fqn.cmp(&right.fqn))
    });
    Ok(symbols)
}

const fn is_supported_language(language: Language) -> bool {
    matches!(
        language,
        Language::Rust
            | Language::TypeScript
            | Language::JavaScript
            | Language::Python
            | Language::Go
    )
}

async fn persisted_symbols(repo_id: &str) -> Result<Vec<SymbolRecord>, CliError> {
    let database_url = env::var("DATABASE_URL").map_err(|_| CliError::MissingEnv {
        key: "DATABASE_URL",
    })?;
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(database_url.as_str())
        .await?;
    Ok(PgSymbolStore::new(pool)
        .active_symbols_for_repo(repo_id)
        .await?)
}

fn print_symbols(repo_id: Option<&str>, symbols: &[SymbolRecord]) -> Result<(), CliError> {
    print_json(&json!({
        "status": "ok",
        "kind": "symbols",
        "repo_id": repo_id,
        "symbol_count": symbols.len(),
        "symbols": symbols.iter().map(symbol_json).collect::<Vec<_>>(),
    }))
}

pub(crate) fn symbol_json(symbol: &SymbolRecord) -> serde_json::Value {
    json!({
        "stable_symbol_id": symbol.stable_symbol_id,
        "versioned_symbol_id": symbol.versioned_symbol_id,
        "file_path": symbol.file_path,
        "language": symbol.language,
        "kind": symbol.kind,
        "name": symbol.name,
        "fqn": symbol.fqn,
        "range": {
            "start_line": symbol.range.start_line,
            "start_column": symbol.range.start_column,
            "end_line": symbol.range.end_line,
            "end_column": symbol.range.end_column,
        },
    })
}

fn print_json(value: &serde_json::Value) -> Result<(), CliError> {
    let stdout = io::stdout();
    let mut lock = stdout.lock();
    serde_json::to_writer_pretty(&mut lock, value)?;
    writeln!(lock)?;
    Ok(())
}

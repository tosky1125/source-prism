#![allow(
    clippy::redundant_pub_crate,
    reason = "Binary crate helper modules share crate-visible command handlers."
)]

use std::{
    collections::BTreeMap,
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

use ri_core::{CommitSha, FilePath, Language, RepoId};
use ri_git::{LocalManifest, discover_worktree, resolve_commit_sha};
use ri_parser::{SourceFile, SymbolExtractor};
use ri_symbols::{SymbolRecord, innermost_symbol_for_line};
use ri_tree_sitter::TreeSitterExtractor;
use serde_json::json;

use crate::error::CliError;

pub(crate) fn symbols_command(mut args: impl Iterator<Item = String>) -> Result<(), CliError> {
    let repo_path = parse_repo_args(&mut args)?;
    let symbols = extract_repo_symbols(&repo_path)?;
    print_json(&json!({
        "status": "ok",
        "kind": "symbols",
        "symbol_count": symbols.len(),
        "symbols": symbols.iter().map(symbol_json).collect::<Vec<_>>(),
    }))
}

pub(crate) fn changed_symbols_command(
    mut args: impl Iterator<Item = String>,
) -> Result<(), CliError> {
    let Some(flag) = args.next() else {
        return Err(CliError::Usage);
    };
    if flag != "--diff" {
        return Err(CliError::Usage);
    }
    let Some(diff_path) = args.next() else {
        return Err(CliError::Usage);
    };
    if args.next().is_some() {
        return Err(CliError::Usage);
    }

    let diff = fs::read_to_string(diff_path)?;
    let changed_lines = parse_changed_lines(&diff);
    let repo_path = PathBuf::from(".");
    let symbols = extract_repo_symbols(&repo_path)?;
    let by_file = symbols_by_file(&symbols);
    let changed_symbols = changed_lines
        .iter()
        .filter_map(|line| {
            let file_symbols = by_file.get(line.file_path.as_str())?;
            let symbol = innermost_symbol_for_line(file_symbols, line.line)?;
            Some(json!({
                "file_path": line.file_path,
                "line": line.line,
                "symbol": symbol_json(symbol),
            }))
        })
        .collect::<Vec<_>>();

    print_json(&json!({
        "status": "ok",
        "kind": "changed_symbols",
        "changed_line_count": changed_lines.len(),
        "matched_symbol_count": changed_symbols.len(),
        "changed_symbols": changed_symbols,
    }))
}

fn parse_repo_args(args: &mut impl Iterator<Item = String>) -> Result<PathBuf, CliError> {
    let Some(flag) = args.next() else {
        return Err(CliError::Usage);
    };
    if flag != "--repo" {
        return Err(CliError::Usage);
    }
    let Some(path) = args.next() else {
        return Err(CliError::Usage);
    };
    if args.next().is_some() {
        return Err(CliError::Usage);
    }
    Ok(PathBuf::from(path))
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

fn symbols_by_file(symbols: &[SymbolRecord]) -> BTreeMap<String, Vec<SymbolRecord>> {
    let mut by_file = BTreeMap::<String, Vec<SymbolRecord>>::new();
    for symbol in symbols {
        by_file
            .entry(symbol.file_path.to_string())
            .or_default()
            .push(symbol.clone());
    }
    by_file
}

fn parse_changed_lines(diff: &str) -> Vec<ChangedLine> {
    let mut file_path = None::<String>;
    let mut new_line = None::<u32>;
    let mut changed = Vec::new();

    for line in diff.lines() {
        if let Some(path) = line.strip_prefix("+++ ") {
            file_path = parse_diff_path(path);
            continue;
        }
        if let Some(header) = line.strip_prefix("@@") {
            new_line = parse_hunk_new_start(header);
            continue;
        }
        let Some(current_line) = new_line else {
            continue;
        };
        if line.starts_with('+') {
            if let Some(path) = &file_path {
                changed.push(ChangedLine {
                    file_path: path.clone(),
                    line: current_line,
                });
            }
            new_line = current_line.checked_add(1);
        } else if !line.starts_with('-') && !line.starts_with('\\') {
            new_line = current_line.checked_add(1);
        }
    }
    changed
}

fn parse_diff_path(path: &str) -> Option<String> {
    if path == "/dev/null" {
        return None;
    }
    Some(path.strip_prefix("b/").unwrap_or(path).to_owned())
}

fn parse_hunk_new_start(header: &str) -> Option<u32> {
    header
        .split_whitespace()
        .find_map(|part| part.strip_prefix('+'))
        .and_then(|part| part.split(',').next())
        .and_then(|line| line.parse::<u32>().ok())
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

fn symbol_json(symbol: &SymbolRecord) -> serde_json::Value {
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

#[derive(Debug)]
struct ChangedLine {
    file_path: String,
    line: u32,
}

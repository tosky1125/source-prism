#![allow(
    clippy::redundant_pub_crate,
    reason = "Binary crate helper modules share crate-visible command handlers."
)]

use std::{
    collections::BTreeMap,
    fs,
    io::{self, Write},
    path::PathBuf,
};

use ri_symbols::{SymbolRecord, innermost_symbol_for_line};
use serde_json::json;

use crate::{
    error::CliError,
    symbols::{extract_repo_symbols, symbol_json},
};

pub(crate) fn changed_symbols_command(
    mut args: impl Iterator<Item = String>,
) -> Result<(), CliError> {
    let request = ChangedSymbolsArgs::parse(&mut args)?;
    let diff = fs::read_to_string(request.diff_path)?;
    let changed_lines = parse_changed_lines(&diff);
    let symbols = extract_repo_symbols(&request.repo)?;
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

#[derive(Debug)]
struct ChangedSymbolsArgs {
    repo: PathBuf,
    diff_path: PathBuf,
}

impl ChangedSymbolsArgs {
    fn parse(args: &mut impl Iterator<Item = String>) -> Result<Self, CliError> {
        let mut repo = PathBuf::from(".");
        let mut diff_path = None::<PathBuf>;

        while let Some(flag) = args.next() {
            match flag.as_str() {
                "--repo" => {
                    let Some(path) = args.next() else {
                        return Err(CliError::Usage);
                    };
                    repo = PathBuf::from(path);
                }
                "--diff" => {
                    let Some(path) = args.next() else {
                        return Err(CliError::Usage);
                    };
                    diff_path = Some(PathBuf::from(path));
                }
                _ => return Err(CliError::Usage),
            }
        }

        Ok(Self {
            repo,
            diff_path: diff_path.ok_or(CliError::Usage)?,
        })
    }
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

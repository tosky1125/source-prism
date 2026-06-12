#![allow(
    missing_docs,
    reason = "Symbol contract names are self-describing at this milestone."
)]

use ri_core::{CommitSha, FilePath, Language, RepoId, SymbolId, SymbolKind};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SymbolRange {
    pub start_line: u32,
    pub start_column: u32,
    pub end_line: u32,
    pub end_column: u32,
}

impl SymbolRange {
    pub const fn new(start_line: u32, start_column: u32, end_line: u32, end_column: u32) -> Self {
        Self {
            start_line,
            start_column,
            end_line,
            end_column,
        }
    }

    pub const fn contains_line(&self, line: u32) -> bool {
        self.start_line <= line && line <= self.end_line
    }

    pub const fn line_span(&self) -> u32 {
        self.end_line.saturating_sub(self.start_line)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SymbolRecord {
    pub stable_symbol_id: SymbolId,
    pub versioned_symbol_id: SymbolId,
    pub file_path: FilePath,
    pub language: Language,
    pub kind: SymbolKind,
    pub name: String,
    pub fqn: String,
    pub range: SymbolRange,
}

impl SymbolRecord {
    pub fn new(
        repo: &RepoId,
        commit: &CommitSha,
        file_path: FilePath,
        content_hash: &str,
        spec: SymbolSpec,
    ) -> Self {
        let stable_symbol_id = SymbolId::stable(repo, &file_path, &spec.fqn);
        let versioned_symbol_id =
            SymbolId::versioned(repo, commit, &file_path, &spec.fqn, content_hash);
        Self {
            stable_symbol_id,
            versioned_symbol_id,
            file_path,
            language: spec.language,
            kind: spec.kind,
            name: spec.name,
            fqn: spec.fqn,
            range: spec.range,
        }
    }

    pub fn from_ids(
        stable_symbol_id: SymbolId,
        versioned_symbol_id: SymbolId,
        file_path: FilePath,
        spec: SymbolSpec,
    ) -> Self {
        Self {
            stable_symbol_id,
            versioned_symbol_id,
            file_path,
            language: spec.language,
            kind: spec.kind,
            name: spec.name,
            fqn: spec.fqn,
            range: spec.range,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct SymbolSpec {
    pub language: Language,
    pub kind: SymbolKind,
    pub name: String,
    pub fqn: String,
    pub range: SymbolRange,
}

impl SymbolSpec {
    pub fn new(
        language: Language,
        kind: SymbolKind,
        name: impl Into<String>,
        fqn: impl Into<String>,
        range: SymbolRange,
    ) -> Self {
        Self {
            language,
            kind,
            name: name.into(),
            fqn: fqn.into(),
            range,
        }
    }
}

pub fn innermost_symbol_for_line(symbols: &[SymbolRecord], line: u32) -> Option<&SymbolRecord> {
    symbols
        .iter()
        .filter(|symbol| symbol.range.contains_line(line))
        .min_by_key(|symbol| {
            (
                symbol.range.line_span(),
                symbol
                    .range
                    .end_column
                    .saturating_sub(symbol.range.start_column),
            )
        })
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ChangedLine {
    pub file_path: String,
    pub line: u32,
}

impl ChangedLine {
    pub fn new(file_path: impl Into<String>, line: u32) -> Self {
        Self {
            file_path: file_path.into(),
            line,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ChangedSymbol {
    pub file_path: String,
    pub line: u32,
    pub symbol: SymbolRecord,
}

impl ChangedSymbol {
    pub fn new(file_path: impl Into<String>, line: u32, symbol: SymbolRecord) -> Self {
        Self {
            file_path: file_path.into(),
            line,
            symbol,
        }
    }
}

pub fn changed_symbols_for_diff(
    symbols: &[SymbolRecord],
    diff: &str,
) -> (Vec<ChangedLine>, Vec<ChangedSymbol>) {
    let changed_lines = parse_changed_lines(diff);
    let by_file = symbols_by_file(symbols);
    let changed_symbols = changed_lines
        .iter()
        .filter_map(|line| {
            let file_symbols = by_file.get(line.file_path.as_str())?;
            let symbol = innermost_symbol_for_line(file_symbols, line.line)?;
            Some(ChangedSymbol::new(
                line.file_path.clone(),
                line.line,
                symbol.clone(),
            ))
        })
        .collect::<Vec<_>>();
    (changed_lines, changed_symbols)
}

pub fn parse_changed_lines(diff: &str) -> Vec<ChangedLine> {
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
                changed.push(ChangedLine::new(path.clone(), current_line));
            }
            new_line = current_line.checked_add(1);
        } else if !line.starts_with('-') && !line.starts_with('\\') {
            new_line = current_line.checked_add(1);
        }
    }
    changed
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

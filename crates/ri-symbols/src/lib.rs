#![allow(
    missing_docs,
    reason = "Symbol contract names are self-describing at this milestone."
)]

use ri_core::{CommitSha, FilePath, Language, RepoId, SymbolId, SymbolKind};
use serde::{Deserialize, Serialize};

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

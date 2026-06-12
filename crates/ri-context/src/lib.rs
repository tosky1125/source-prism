#![allow(
    missing_docs,
    reason = "Context pack contracts are self-describing at this milestone."
)]
#![allow(
    clippy::multiple_crate_versions,
    reason = "Tree-sitter and SQLx-adjacent workspace dependencies pull duplicate transitive crates outside this crate's control."
)]

use ri_core::{CommitSha, FilePath, Language, RepoId, SymbolId};
use ri_git::{LocalManifest, discover_worktree, resolve_commit_sha};
use ri_impact::{ImpactReport, analyze_symbol_impact};
use ri_parser::{CallExtractor, CallReference, SourceFile, SymbolExtractor};
use ri_search::{SearchHit, search_symbols};
use ri_symbols::{SymbolRange, SymbolRecord, innermost_symbol_for_line};
use ri_tree_sitter::TreeSitterExtractor;
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};
use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ContextError {
    #[error(transparent)]
    Core(#[from] ri_core::CoreError),
    #[error(transparent)]
    Git(#[from] ri_git::GitError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Parser(#[from] ri_parser::ParserError),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ContextPack {
    pub query: String,
    pub retrieval_modes: Vec<RetrievalMode>,
    pub vector_used: bool,
    pub vector_only: bool,
    pub hits: Vec<SearchHit>,
    pub impacts: Vec<ImpactReport>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum RetrievalMode {
    ExactIdentifier,
    Lexical,
    SymbolGraphProximity,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct RepoIndexEvidence {
    pub symbols: Vec<SymbolRecord>,
    pub calls: Vec<ResolvedCallReference>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct ResolvedCallReference {
    pub source_symbol_id: SymbolId,
    pub target_symbol_id: SymbolId,
    pub file_path: FilePath,
    pub target_name: String,
    pub range: SymbolRange,
}

pub fn build_context_pack(symbols: &[SymbolRecord], query: &str, limit: usize) -> ContextPack {
    let search = search_symbols(symbols, query, limit);
    let impacts = search
        .hits
        .iter()
        .filter_map(|hit| analyze_symbol_impact(symbols.to_vec(), &hit.symbol.fqn).ok())
        .collect::<Vec<_>>();
    ContextPack {
        query: query.to_owned(),
        retrieval_modes: vec![
            RetrievalMode::ExactIdentifier,
            RetrievalMode::Lexical,
            RetrievalMode::SymbolGraphProximity,
        ],
        vector_used: search.vector_used,
        vector_only: search.vector_only,
        hits: search.hits,
        impacts,
    }
}

pub fn extract_repo_symbols(repo_path: &Path) -> Result<Vec<SymbolRecord>, ContextError> {
    let worktree = discover_worktree(repo_path)?;
    let repo = RepoId::new(format!("local:{}", worktree.canonicalize()?.display()))?;
    let commit = CommitSha::new(resolve_commit_sha(repo_path, "HEAD")?)?;
    extract_repo_symbols_for(repo_path, &repo, &commit)
}

pub fn extract_repo_symbols_for(
    repo_path: &Path,
    repo: &RepoId,
    commit: &CommitSha,
) -> Result<Vec<SymbolRecord>, ContextError> {
    Ok(extract_repo_index_for(repo_path, repo, commit)?.symbols)
}

pub fn extract_repo_index_for(
    repo_path: &Path,
    repo: &RepoId,
    commit: &CommitSha,
) -> Result<RepoIndexEvidence, ContextError> {
    let worktree = discover_worktree(repo_path)?;
    let manifest = LocalManifest::extract(repo_path)?;
    let extractor = TreeSitterExtractor::new();
    let mut symbols = Vec::new();
    let mut calls = Vec::new();

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
        calls.extend(extractor.extract_calls(&source_file)?);
    }
    symbols.sort_by(|left, right| {
        left.file_path
            .cmp(&right.file_path)
            .then(left.fqn.cmp(&right.fqn))
    });
    Ok(RepoIndexEvidence {
        calls: resolve_calls(&symbols, &calls),
        symbols,
    })
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

fn resolve_calls(symbols: &[SymbolRecord], calls: &[CallReference]) -> Vec<ResolvedCallReference> {
    calls
        .iter()
        .filter_map(|call| resolve_call(symbols, call))
        .collect()
}

fn resolve_call(symbols: &[SymbolRecord], call: &CallReference) -> Option<ResolvedCallReference> {
    let file_symbols = symbols
        .iter()
        .filter(|symbol| symbol.file_path == call.file_path)
        .cloned()
        .collect::<Vec<_>>();
    let source = innermost_symbol_for_line(&file_symbols, call.range.start_line)?;
    let target = symbols
        .iter()
        .filter(|symbol| symbol.name == call.target_name)
        .filter(|symbol| symbol.versioned_symbol_id != source.versioned_symbol_id)
        .min_by_key(|symbol| {
            (
                symbol.file_path != call.file_path,
                symbol.file_path.as_str().to_owned(),
                symbol.fqn.clone(),
            )
        })?;
    Some(ResolvedCallReference {
        source_symbol_id: source.versioned_symbol_id.clone(),
        target_symbol_id: target.versioned_symbol_id.clone(),
        file_path: call.file_path.clone(),
        target_name: call.target_name.clone(),
        range: call.range.clone(),
    })
}

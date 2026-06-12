use ri_architecture::extract_architecture_entities_for;
use ri_core::{CommitSha, RepoId, SymbolKind};
use ri_git::{LocalManifest, resolve_commit_sha};
use std::collections::BTreeSet;

use crate::{AppError, state::AppState};

#[derive(Debug)]
pub(crate) struct LocalIndexSummary {
    pub(crate) run_id: String,
    pub(crate) commit_sha: String,
    pub(crate) started_at: String,
    pub(crate) finished_at: Option<String>,
    pub(crate) file_manifests: i64,
    pub(crate) symbols: i64,
    pub(crate) graph_nodes: i64,
    pub(crate) graph_edges: i64,
    pub(crate) search_chunks: i64,
    pub(crate) test_cases: i64,
    pub(crate) architecture_entities: i64,
}

pub(crate) fn local_index_summary(
    state: &AppState,
    repo_id: &str,
) -> Result<LocalIndexSummary, AppError> {
    let evidence = state.context_index_evidence()?;
    let symbols = evidence.symbols.as_slice();
    let manifest = LocalManifest::extract(state.context_repo_path())?;
    let commit_sha = resolve_commit_sha(state.context_repo_path(), "HEAD")?;
    let symbol_files = symbols
        .iter()
        .map(|symbol| symbol.file_path.to_string())
        .collect::<BTreeSet<_>>();
    let repo = RepoId::new(repo_id)?;
    let commit = CommitSha::new(&commit_sha)?;
    let architecture_entities =
        extract_architecture_entities_for(state.context_repo_path(), &repo, &commit, &manifest)?
            .len();

    Ok(LocalIndexSummary {
        run_id: format!("local:{repo_id}:{commit_sha}"),
        commit_sha,
        started_at: "local-worktree".to_owned(),
        finished_at: Some("local-worktree".to_owned()),
        file_manifests: usize_to_i64(manifest.files().len()),
        symbols: usize_to_i64(symbols.len()),
        graph_nodes: usize_to_i64(symbol_files.len().saturating_add(symbols.len())),
        graph_edges: usize_to_i64(symbols.len().saturating_add(evidence.calls.len())),
        search_chunks: usize_to_i64(symbols.len()),
        test_cases: usize_to_i64(
            symbols
                .iter()
                .filter(|symbol| symbol.kind == SymbolKind::TestCase)
                .count(),
        ),
        architecture_entities: usize_to_i64(architecture_entities),
    })
}

fn usize_to_i64(value: usize) -> i64 {
    i64::try_from(value).map_or(i64::MAX, |converted| converted)
}

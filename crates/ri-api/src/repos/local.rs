use crate::{
    AppError,
    local_index::local_index_summary,
    repos::{RepoEvidenceSummary, RepoLatestRun, RepoSummary},
    state::AppState,
};

pub(super) fn local_repo(repo_id: &str) -> RepoSummary {
    RepoSummary {
        repo_id: repo_id.to_owned(),
        name: repo_id.to_owned(),
        origin_url: None,
        default_branch: None,
    }
}

pub(super) fn local_latest_run(state: &AppState, repo_id: &str) -> Result<RepoLatestRun, AppError> {
    let local = local_index_summary(state, repo_id)?;
    Ok(RepoLatestRun {
        run_id: local.run_id,
        commit_sha: local.commit_sha,
        index_kind: "local_worktree".to_owned(),
        status: "succeeded".to_owned(),
        started_at: local.started_at,
        finished_at: local.finished_at,
        evidence: RepoEvidenceSummary {
            file_manifests: local.file_manifests,
            symbols: local.symbols,
            graph_nodes: local.graph_nodes,
            graph_edges: local.graph_edges,
            search_chunks: local.search_chunks,
            test_cases: local.test_cases,
            architecture_entities: local.architecture_entities,
        },
    })
}

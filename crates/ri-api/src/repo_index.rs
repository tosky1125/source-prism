use axum::{
    Json,
    extract::{Path, State},
};
use ri_architecture::extract_architecture_entities_for;
use ri_context::{ResolvedCallReference, extract_repo_index_for};
use ri_core::{CommitSha, Language, RepoId};
use ri_git::{LocalManifest, resolve_commit_sha};
use ri_indexer::{
    CallEdgeInput, DEFAULT_SEARCH_INDEX, FileManifestInput, PgArchitectureStore, PgGenerationStore,
    PgGraphStore, PgSearchSyncStore, PgSymbolStore, PgTestCaseStore,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::{AppError, state::AppState};

const DEFAULT_SHA: &str = "HEAD";
const INDEX_KIND: &str = "file_manifest";
const EXTRACTOR_VERSION: &str = "ri-api-index-v1";

#[derive(Debug, Deserialize)]
pub(crate) struct IndexRepoRequest {
    sha: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct IndexRepoResponse {
    status: &'static str,
    kind: &'static str,
    repo_id: String,
    commit_sha: String,
    run_id: String,
    generation_id: String,
    inserted_file_manifests: u64,
    indexed_symbols: u64,
    indexed_graph_nodes: u64,
    indexed_graph_edges: u64,
    indexed_import_edges: u64,
    indexed_call_edges: u64,
    indexed_test_cover_edges: u64,
    indexed_search_chunks: u64,
    indexed_test_cases: u64,
    indexed_architecture_entities: u64,
}

pub(crate) async fn index(
    State(state): State<AppState>,
    Path(repo_id): Path<String>,
    Json(request): Json<IndexRepoRequest>,
) -> Result<Json<IndexRepoResponse>, AppError> {
    let pool = state
        .database
        .pool
        .as_ref()
        .ok_or(AppError::DatabaseNotConfigured)?;
    let repo_path = state.context_repo_path();
    let sha = request.sha.as_deref().unwrap_or(DEFAULT_SHA).trim();
    if sha.is_empty() {
        return Err(AppError::Validation("sha must not be empty".to_owned()));
    }
    let commit_sha = resolve_commit_sha(repo_path, sha)?;
    let repo = RepoId::new(&repo_id).map_err(|error| AppError::Validation(error.to_string()))?;
    let commit =
        CommitSha::new(&commit_sha).map_err(|error| AppError::Validation(error.to_string()))?;
    upsert_repo_commit(pool, &repo_id, &commit_sha).await?;

    let manifest = LocalManifest::extract(repo_path)?;
    let inputs = manifest_inputs(&manifest)?;
    let store = PgGenerationStore::new(pool.clone());
    let generation = store
        .begin_generation(&repo_id, &commit_sha, INDEX_KIND, Some(EXTRACTOR_VERSION))
        .await?;
    let inserted = store
        .replace_file_manifest_generation(&generation.generation_id, &inputs)
        .await?;
    let evidence = extract_repo_index_for(repo_path, &repo, &commit)?;
    let symbols = evidence.symbols;
    let indexed_symbols = PgSymbolStore::new(pool.clone())
        .replace_symbol_generation(&generation.generation_id, &symbols)
        .await?;
    let indexed_test_cases = PgTestCaseStore::new(pool.clone())
        .replace_test_cases_for_generation(&generation.generation_id, &symbols)
        .await?;
    let architecture = extract_architecture_entities_for(repo_path, &repo, &commit, &manifest)?;
    let indexed_architecture_entities = PgArchitectureStore::new(pool.clone())
        .replace_architecture_entities_for_generation(&generation.generation_id, &architecture)
        .await?;
    let graph_store = PgGraphStore::new(pool.clone());
    let graph = graph_store
        .replace_contains_graph(&generation.generation_id, &symbols)
        .await?;
    let indexed_test_cover_edges = graph_store
        .replace_test_covers_graph(&generation.generation_id)
        .await?;
    let indexed_import_edges = graph_store
        .replace_import_graph(&generation.generation_id)
        .await?;
    let indexed_call_edges = graph_store
        .replace_call_graph(&generation.generation_id, &call_inputs(&evidence.calls))
        .await?;
    let indexed_search_chunks = PgSearchSyncStore::new(pool.clone())
        .enqueue_symbol_chunks(
            &repo_id,
            &generation.generation_id,
            &symbols,
            DEFAULT_SEARCH_INDEX,
        )
        .await?;
    let generation_id = generation.generation_id.to_string();
    Ok(Json(IndexRepoResponse {
        status: "succeeded",
        kind: "index",
        repo_id,
        commit_sha,
        run_id: generation_id.clone(),
        generation_id,
        inserted_file_manifests: inserted,
        indexed_symbols,
        indexed_graph_nodes: graph.nodes,
        indexed_graph_edges: graph
            .edges
            .saturating_add(indexed_test_cover_edges)
            .saturating_add(indexed_import_edges)
            .saturating_add(indexed_call_edges),
        indexed_import_edges,
        indexed_call_edges,
        indexed_test_cover_edges,
        indexed_search_chunks,
        indexed_test_cases,
        indexed_architecture_entities,
    }))
}

fn call_inputs(calls: &[ResolvedCallReference]) -> Vec<CallEdgeInput> {
    calls
        .iter()
        .map(|call| {
            CallEdgeInput::new(
                call.source_symbol_id.to_string(),
                call.target_symbol_id.to_string(),
                call.file_path.to_string(),
                call.range.clone(),
                call.target_name.clone(),
            )
        })
        .collect()
}

async fn upsert_repo_commit(
    pool: &PgPool,
    repo_id: &str,
    commit_sha: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r"
        INSERT INTO repos (repo_id, name)
        VALUES ($1, $1)
        ON CONFLICT (repo_id) DO UPDATE SET updated_at = now()
        ",
    )
    .bind(repo_id)
    .execute(pool)
    .await?;
    sqlx::query(
        r"
        INSERT INTO commits (repo_id, commit_sha)
        VALUES ($1, $2)
        ON CONFLICT (repo_id, commit_sha) DO NOTHING
        ",
    )
    .bind(repo_id)
    .bind(commit_sha)
    .execute(pool)
    .await?;
    Ok(())
}

fn manifest_inputs(manifest: &LocalManifest) -> Result<Vec<FileManifestInput>, AppError> {
    let mut inputs = Vec::with_capacity(manifest.files().len());
    for file in manifest.files() {
        let size_bytes = i64::try_from(file.size_bytes()).map_err(|_| AppError::FileTooLarge {
            path: file.path().to_owned(),
            size_bytes: file.size_bytes(),
        })?;
        let mut input = FileManifestInput::new(file.path(), file.content_sha256(), size_bytes);
        language_id(file.language()).clone_into(&mut input.language);
        input.is_generated = file.is_generated();
        input.is_vendor = file.is_vendor();
        input.is_test = file.is_test();
        inputs.push(input);
    }
    Ok(inputs)
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

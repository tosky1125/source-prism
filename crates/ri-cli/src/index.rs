#![allow(
    clippy::redundant_pub_crate,
    reason = "Binary crate helper modules share crate-visible command handlers."
)]

use std::{
    env,
    io::{self, Write},
    path::PathBuf,
};

use ri_context::extract_repo_symbols_for;
use ri_core::{CommitSha, Language, RepoId};
use ri_git::{LocalManifest, discover_worktree, resolve_commit_sha};
use ri_indexer::{
    DEFAULT_SEARCH_INDEX, FileManifestInput, PgGenerationStore, PgGraphStore, PgSearchSyncStore,
    PgSymbolStore, PgTestCaseStore,
};
use serde_json::json;
use sqlx::{PgPool, postgres::PgPoolOptions};

use crate::error::CliError;

pub(crate) async fn command(mut args: impl Iterator<Item = String>) -> Result<(), CliError> {
    let Some(repo_flag) = args.next() else {
        return Err(CliError::Usage);
    };
    if repo_flag != "--repo" {
        return Err(CliError::Usage);
    }
    let Some(repo_arg) = args.next() else {
        return Err(CliError::Usage);
    };
    let Some(sha_flag) = args.next() else {
        return Err(CliError::Usage);
    };
    if sha_flag != "--sha" {
        return Err(CliError::Usage);
    }
    let Some(sha_arg) = args.next() else {
        return Err(CliError::Usage);
    };
    if args.next().is_some() {
        return Err(CliError::Usage);
    }

    let pool = database_pool().await?;
    let repo_path = PathBuf::from(repo_arg);
    let worktree = discover_worktree(&repo_path)?;
    let repo_id = repo_id_for_worktree(&worktree)?;
    let commit_sha = resolve_commit_sha(&repo_path, &sha_arg)?;
    let repo = RepoId::new(&repo_id)?;
    let commit = CommitSha::new(&commit_sha)?;
    upsert_repo_commit(&pool, &repo_id, &worktree, &commit_sha).await?;

    let manifest = LocalManifest::extract(&repo_path)?;
    let inputs = manifest_inputs(&manifest)?;
    let store = PgGenerationStore::new(pool.clone());
    let generation = store
        .begin_generation(
            &repo_id,
            &commit_sha,
            "file_manifest",
            Some("ri-cli-index-v1"),
        )
        .await?;
    let result = store
        .replace_file_manifest_generation(&generation.generation_id, &inputs)
        .await;
    let inserted = match result {
        Ok(inserted) => inserted,
        Err(error) => {
            let _fail_result = store
                .fail_generation(&generation.generation_id, &error.to_string())
                .await;
            return Err(error.into());
        }
    };
    let symbols = extract_repo_symbols_for(&repo_path, &repo, &commit)?;
    let indexed_symbols = PgSymbolStore::new(pool.clone())
        .replace_symbol_generation(&generation.generation_id, &symbols)
        .await?;
    let indexed_test_cases = PgTestCaseStore::new(pool.clone())
        .replace_test_cases_for_generation(&generation.generation_id, &symbols)
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
    let indexed_search_chunks = PgSearchSyncStore::new(pool)
        .enqueue_symbol_chunks(
            &repo_id,
            &generation.generation_id,
            &symbols,
            DEFAULT_SEARCH_INDEX,
        )
        .await?;

    print_index_result(&IndexResult {
        repo_id,
        commit_sha,
        generation_id: generation.generation_id.to_string(),
        inserted_file_manifests: inserted,
        indexed_symbols,
        indexed_graph_nodes: graph.nodes,
        indexed_graph_edges: graph
            .edges
            .saturating_add(indexed_test_cover_edges)
            .saturating_add(indexed_import_edges),
        indexed_import_edges,
        indexed_test_cover_edges,
        indexed_search_chunks,
        indexed_test_cases,
    })
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

async fn upsert_repo_commit(
    pool: &PgPool,
    repo_id: &str,
    worktree: &std::path::Path,
    commit_sha: &str,
) -> Result<(), CliError> {
    let repo_name = worktree
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("local-repo");
    sqlx::query(
        r"
        INSERT INTO repos (repo_id, name)
        VALUES ($1, $2)
        ON CONFLICT (repo_id) DO UPDATE SET updated_at = now()
        ",
    )
    .bind(repo_id)
    .bind(repo_name)
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

fn manifest_inputs(manifest: &LocalManifest) -> Result<Vec<FileManifestInput>, CliError> {
    let mut inputs = Vec::with_capacity(manifest.files().len());
    for file in manifest.files() {
        let size_bytes = i64::try_from(file.size_bytes()).map_err(|_| CliError::FileTooLarge {
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

fn repo_id_for_worktree(worktree: &std::path::Path) -> Result<String, CliError> {
    let canonical = worktree.canonicalize()?;
    Ok(format!("local:{}", canonical.display()))
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

struct IndexResult {
    repo_id: String,
    commit_sha: String,
    generation_id: String,
    inserted_file_manifests: u64,
    indexed_symbols: u64,
    indexed_graph_nodes: u64,
    indexed_graph_edges: u64,
    indexed_import_edges: u64,
    indexed_test_cover_edges: u64,
    indexed_search_chunks: u64,
    indexed_test_cases: u64,
}

fn print_index_result(result: &IndexResult) -> Result<(), CliError> {
    let stdout = io::stdout();
    let mut lock = stdout.lock();
    serde_json::to_writer_pretty(
        &mut lock,
        &json!({
            "status": "ok",
            "kind": "index",
            "repo_id": result.repo_id,
            "commit_sha": result.commit_sha,
            "generation_id": result.generation_id,
            "inserted_file_manifests": result.inserted_file_manifests,
            "indexed_symbols": result.indexed_symbols,
            "indexed_graph_nodes": result.indexed_graph_nodes,
            "indexed_graph_edges": result.indexed_graph_edges,
            "indexed_import_edges": result.indexed_import_edges,
            "indexed_test_cover_edges": result.indexed_test_cover_edges,
            "indexed_search_chunks": result.indexed_search_chunks,
            "indexed_test_cases": result.indexed_test_cases,
        }),
    )?;
    writeln!(lock)?;
    Ok(())
}

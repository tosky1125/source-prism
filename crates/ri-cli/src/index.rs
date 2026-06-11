#![allow(
    clippy::redundant_pub_crate,
    reason = "Binary crate helper modules share crate-visible command handlers."
)]

use std::{
    env,
    io::{self, Write},
    path::PathBuf,
};

use ri_core::Language;
use ri_git::{LocalManifest, discover_worktree, resolve_commit_sha};
use ri_indexer::{FileManifestInput, PgGenerationStore};
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
    upsert_repo_commit(&pool, &repo_id, &worktree, &commit_sha).await?;

    let manifest = LocalManifest::extract(&repo_path)?;
    let inputs = manifest_inputs(&manifest)?;
    let store = PgGenerationStore::new(pool);
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

    print_index_result(
        &repo_id,
        &commit_sha,
        &generation.generation_id.to_string(),
        inserted,
    )
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

fn print_index_result(
    repo_id: &str,
    commit_sha: &str,
    generation_id: &str,
    inserted_file_manifests: u64,
) -> Result<(), CliError> {
    let stdout = io::stdout();
    let mut lock = stdout.lock();
    serde_json::to_writer_pretty(
        &mut lock,
        &json!({
            "status": "ok",
            "kind": "index",
            "repo_id": repo_id,
            "commit_sha": commit_sha,
            "generation_id": generation_id,
            "inserted_file_manifests": inserted_file_manifests,
        }),
    )?;
    writeln!(lock)?;
    Ok(())
}

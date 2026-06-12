#![allow(missing_docs, reason = "Binary crate exposes no public API.")]
#![allow(
    clippy::multiple_crate_versions,
    reason = "SQLx and Reqwest TLS dependencies pull duplicate platform crates outside this crate's control."
)]

use std::{
    env,
    io::{self, Write},
    path::{Path, PathBuf},
    process::ExitCode,
};

use ri_config::{RuntimeConfig, load_env_file};
use ri_git::LocalManifest;
use serde_json::json;
use sqlx::postgres::PgPoolOptions;

pub(crate) mod architecture;
pub(crate) mod embeddings;
pub(crate) mod error;
pub(crate) mod impact;
pub(crate) mod index;
pub(crate) mod index_args;
pub(crate) mod mcp;
pub(crate) mod refactor;
pub(crate) mod references;
pub(crate) mod review;
pub(crate) mod runs;
pub(crate) mod search;
pub(crate) mod search_context;
pub(crate) mod symbols;
pub(crate) mod test_context;
pub(crate) mod test_go;
pub(crate) mod test_junit;
pub(crate) mod test_playwright;
pub(crate) mod test_pytest;
pub(crate) mod tests;

use error::CliError;

#[tokio::main]
async fn main() -> ExitCode {
    match run(env::args()).await {
        Ok(code) => code,
        Err(error) => {
            let mut stderr = io::stderr().lock();
            let _ = writeln!(stderr, "{error}");
            error.exit_code()
        }
    }
}

async fn run(args: impl IntoIterator<Item = String>) -> Result<ExitCode, CliError> {
    let mut args = args.into_iter();
    let _program = args.next();
    let Some(command) = args.next() else {
        return Err(CliError::Usage);
    };

    match command.as_str() {
        "config" => {
            expect_subcommand(&mut args, "check")?;
            check_config(args)?;
            Ok(ExitCode::SUCCESS)
        }
        "db" => {
            expect_subcommand(&mut args, "migrate")?;
            db_migrate(args).await?;
            Ok(ExitCode::SUCCESS)
        }
        "repo" => {
            expect_subcommand(&mut args, "manifest")?;
            repo_manifest(args)?;
            Ok(ExitCode::SUCCESS)
        }
        "index" => index::command(args).await.map(|()| ExitCode::SUCCESS),
        "symbols" => symbols::symbols_command(args).map(|()| ExitCode::SUCCESS),
        "changed-symbols" => symbols::changed_symbols_command(args).map(|()| ExitCode::SUCCESS),
        "references" => references::references_command(args).map(|()| ExitCode::SUCCESS),
        "refactor" => {
            expect_subcommand(&mut args, "plan")?;
            refactor::plan_command(args).map(|()| ExitCode::SUCCESS)
        }
        "review" => review::command(args).map(|()| ExitCode::SUCCESS),
        "run" => runs::run_command(args).await.map(|()| ExitCode::SUCCESS),
        "runs" => runs::command(args).await.map(|()| ExitCode::SUCCESS),
        "embeddings" => embeddings::command(args).await.map(|()| ExitCode::SUCCESS),
        "mcp" => mcp::command(args).map(|()| ExitCode::SUCCESS),
        "architecture" => architecture::architecture_command(args).map(|()| ExitCode::SUCCESS),
        "impact" => impact::impact_command(args).map(|()| ExitCode::SUCCESS),
        "search-context" => {
            search_context::search_context_command(args).map(|()| ExitCode::SUCCESS)
        }
        "test-context" => test_context::test_context_command(args).map(|()| ExitCode::SUCCESS),
        "tests" => tests::command(args).await.map(|()| ExitCode::SUCCESS),
        "search" => search::command(args).await.map(|()| ExitCode::SUCCESS),
        _ => Err(CliError::Usage),
    }
}

fn expect_subcommand(
    args: &mut impl Iterator<Item = String>,
    expected: &str,
) -> Result<(), CliError> {
    if args.next().as_deref() == Some(expected) {
        Ok(())
    } else {
        Err(CliError::Usage)
    }
}

fn check_config(mut args: impl Iterator<Item = String>) -> Result<(), CliError> {
    let Some(flag) = args.next() else {
        return Err(CliError::Usage);
    };
    if flag != "--env-file" {
        return Err(CliError::Usage);
    }
    let Some(path) = args.next() else {
        return Err(CliError::Usage);
    };
    if args.next().is_some() {
        return Err(CliError::Usage);
    }

    let env = load_env_file(&PathBuf::from(path))?;
    let _config = RuntimeConfig::from_env_map(&env)?;
    Ok(())
}

async fn db_migrate(mut args: impl Iterator<Item = String>) -> Result<(), CliError> {
    if args.next().is_some() {
        return Err(CliError::Usage);
    }
    let database_url = env::var("DATABASE_URL").map_err(|_| CliError::MissingEnv {
        key: "DATABASE_URL",
    })?;
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(database_url.as_str())
        .await?;
    let migrator = sqlx::migrate::Migrator::new(Path::new("migrations")).await?;
    migrator.run(&pool).await?;
    writeln!(io::stdout().lock(), "db migrate ok")?;
    Ok(())
}

fn repo_manifest(mut args: impl Iterator<Item = String>) -> Result<(), CliError> {
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
    let manifest = LocalManifest::extract(&path)?;
    let files = manifest
        .files()
        .iter()
        .map(|file| {
            json!({
                "path": file.path(),
                "language": file.language(),
                "size_bytes": file.size_bytes(),
                "content_sha256": file.content_sha256(),
                "is_generated": file.is_generated(),
                "is_vendor": file.is_vendor(),
                "is_test": file.is_test(),
            })
        })
        .collect::<Vec<_>>();
    print_json(&json!({
        "status": "ok",
        "kind": "manifest",
        "repo": path,
        "file_count": files.len(),
        "files": files,
    }))
}

fn print_json(value: &serde_json::Value) -> Result<(), CliError> {
    let stdout = io::stdout();
    let mut lock = stdout.lock();
    serde_json::to_writer_pretty(&mut lock, value)?;
    writeln!(lock)?;
    Ok(())
}

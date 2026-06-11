#![allow(missing_docs, reason = "Binary crate exposes no public API.")]

use std::{
    env,
    io::{self, Write},
    path::PathBuf,
    process::ExitCode,
};

use ri_config::{RuntimeConfig, load_env_file};
use thiserror::Error;

fn main() -> ExitCode {
    match run(env::args()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            let mut stderr = io::stderr().lock();
            let _ = writeln!(stderr, "{error}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: impl IntoIterator<Item = String>) -> Result<(), CliError> {
    let mut args = args.into_iter();
    let _program = args.next();
    let Some(command) = args.next() else {
        return Err(CliError::Usage);
    };
    let Some(subcommand) = args.next() else {
        return Err(CliError::Usage);
    };

    match (command.as_str(), subcommand.as_str()) {
        ("config", "check") => check_config(args),
        _ => Err(CliError::Usage),
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

#[derive(Debug, Error)]
enum CliError {
    #[error("usage: ri-cli config check --env-file <path>")]
    Usage,
    #[error("{0}")]
    Config(#[from] ri_config::ConfigError),
}

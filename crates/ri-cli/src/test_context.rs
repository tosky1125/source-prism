#![allow(
    clippy::redundant_pub_crate,
    reason = "Binary crate helper modules share crate-visible command handlers."
)]

use std::{
    io::{self, Write},
    path::PathBuf,
};

use ri_behavior::build_test_context;
use serde_json::json;

use crate::{error::CliError, symbols::extract_repo_symbols};

pub(crate) fn test_context_command(mut args: impl Iterator<Item = String>) -> Result<(), CliError> {
    let request = TestContextArgs::parse(&mut args)?;
    let symbols = extract_repo_symbols(&request.repo)?;
    let test_context = build_test_context(&symbols, &request.symbol)?;
    print_json(&json!({
        "status": "ok",
        "kind": "test_context",
        "test_context": test_context,
    }))
}

#[derive(Debug)]
struct TestContextArgs {
    repo: PathBuf,
    symbol: String,
}

impl TestContextArgs {
    fn parse(args: &mut impl Iterator<Item = String>) -> Result<Self, CliError> {
        let mut repo = PathBuf::from(".");
        let mut symbol = None::<String>;

        while let Some(flag) = args.next() {
            match flag.as_str() {
                "--repo" => {
                    let Some(path) = args.next() else {
                        return Err(CliError::Usage);
                    };
                    repo = PathBuf::from(path);
                }
                "--symbol" => {
                    symbol = args.next();
                }
                _ => return Err(CliError::Usage),
            }
        }

        Ok(Self {
            repo,
            symbol: symbol.ok_or(CliError::Usage)?,
        })
    }
}

fn print_json(value: &serde_json::Value) -> Result<(), CliError> {
    let stdout = io::stdout();
    let mut lock = stdout.lock();
    serde_json::to_writer_pretty(&mut lock, value)?;
    writeln!(lock)?;
    Ok(())
}

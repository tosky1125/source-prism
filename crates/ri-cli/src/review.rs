#![allow(
    clippy::redundant_pub_crate,
    reason = "Binary crate helper modules share crate-visible command handlers."
)]

use std::{
    fs,
    io::{self, Write},
    path::PathBuf,
};

use ri_github::build_review_dry_run;
use ri_review::{ProposedFinding, verify_findings};
use serde_json::json;

use crate::error::CliError;

pub(crate) fn command(mut args: impl Iterator<Item = String>) -> Result<(), CliError> {
    let Some(subcommand) = args.next() else {
        return Err(CliError::Usage);
    };
    match subcommand.as_str() {
        "verify" => verify_command(args),
        "github-dry-run" => github_dry_run_command(args),
        _ => Err(CliError::Usage),
    }
}

fn verify_command(args: impl Iterator<Item = String>) -> Result<(), CliError> {
    let findings = read_findings(args)?;
    let verified = verify_findings(findings.as_slice())?;
    print_json(&json!({
        "status": "ok",
        "kind": "review_verification",
        "verified_count": verified.len(),
        "findings": verified,
    }))
}

fn github_dry_run_command(args: impl Iterator<Item = String>) -> Result<(), CliError> {
    let findings = read_findings(args)?;
    let verified = verify_findings(findings.as_slice())?;
    let dry_run = build_review_dry_run(verified.as_slice());
    print_json(&json!({
        "status": "ok",
        "kind": "github_review_dry_run",
        "verified_count": verified.len(),
        "annotations": dry_run.annotations,
        "sarif": dry_run.sarif,
    }))
}

fn read_findings(mut args: impl Iterator<Item = String>) -> Result<Vec<ProposedFinding>, CliError> {
    let Some(flag) = args.next() else {
        return Err(CliError::Usage);
    };
    if flag != "--input" {
        return Err(CliError::Usage);
    }
    let Some(input) = args.next() else {
        return Err(CliError::Usage);
    };
    if args.next().is_some() {
        return Err(CliError::Usage);
    }

    let body = fs::read_to_string(PathBuf::from(input))?;
    Ok(serde_json::from_str::<Vec<ProposedFinding>>(body.as_str())?)
}

fn print_json(value: &serde_json::Value) -> Result<(), CliError> {
    let stdout = io::stdout();
    let mut lock = stdout.lock();
    serde_json::to_writer_pretty(&mut lock, value)?;
    writeln!(lock)?;
    Ok(())
}

#![allow(
    clippy::redundant_pub_crate,
    reason = "Binary crate helper modules share crate-visible command handlers."
)]

use std::{
    fs,
    io::{self, Write},
    path::PathBuf,
};

use ri_review::{ProposedFinding, verify_findings};
use serde_json::json;

use crate::error::CliError;

pub(crate) fn verify_command(mut args: impl Iterator<Item = String>) -> Result<(), CliError> {
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
    let findings = serde_json::from_str::<Vec<ProposedFinding>>(body.as_str())?;
    let verified = verify_findings(findings.as_slice())?;
    print_json(&json!({
        "status": "ok",
        "kind": "review_verification",
        "verified_count": verified.len(),
        "findings": verified,
    }))
}

fn print_json(value: &serde_json::Value) -> Result<(), CliError> {
    let stdout = io::stdout();
    let mut lock = stdout.lock();
    serde_json::to_writer_pretty(&mut lock, value)?;
    writeln!(lock)?;
    Ok(())
}

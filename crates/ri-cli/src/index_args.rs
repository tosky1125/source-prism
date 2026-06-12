#![allow(
    clippy::redundant_pub_crate,
    reason = "Binary crate helper modules share crate-visible command handlers."
)]

use std::path::PathBuf;

use crate::error::CliError;

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct IndexArgs {
    pub(crate) repo_path: PathBuf,
    pub(crate) sha: String,
}

impl IndexArgs {
    pub(crate) fn parse(mut args: impl Iterator<Item = String>) -> Result<Self, CliError> {
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
        let Some(sha) = args.next() else {
            return Err(CliError::Usage);
        };
        if args.next().is_some() {
            return Err(CliError::Usage);
        }
        Ok(Self {
            repo_path: PathBuf::from(repo_arg),
            sha,
        })
    }
}

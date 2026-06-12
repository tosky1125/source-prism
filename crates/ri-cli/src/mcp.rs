#![allow(
    clippy::redundant_pub_crate,
    reason = "Binary crate helper modules share crate-visible command handlers."
)]

use std::{
    fs,
    io::{self, Write},
    path::PathBuf,
};

use ri_mcp::{
    ImpactToolRequest, McpToolCatalog, ReferenceToolRequest, RepositoryToolHandler,
    SearchContextToolRequest, SymbolToolRequest, TestContextToolRequest, handle_json_rpc_request,
};
use serde_json::json;

use crate::error::CliError;

const DEFAULT_LIMIT: usize = 8;

pub(crate) fn command(mut args: impl Iterator<Item = String>) -> Result<(), CliError> {
    let Some(subcommand) = args.next() else {
        return Err(CliError::Usage);
    };
    match subcommand.as_str() {
        "tools" => tools_command(args),
        "call" => call_command(args),
        "serve" => serve_command(args),
        _ => Err(CliError::Usage),
    }
}

fn tools_command(mut args: impl Iterator<Item = String>) -> Result<(), CliError> {
    if args.next().is_some() {
        return Err(CliError::Usage);
    }
    print_json(&json!({
        "kind": "mcp_tool_catalog",
        "tools": McpToolCatalog::new().tools()
    }))
}

fn call_command(args: impl Iterator<Item = String>) -> Result<(), CliError> {
    let request = McpCallArgs::parse(args)?;
    let evidence = ri_context::extract_repo_index(&request.repo)?;
    let handler = RepositoryToolHandler::new(evidence.symbols, evidence.calls);
    let result = match request.tool.as_str() {
        "repo.get_symbol" => serde_json::to_value(
            handler.get_symbol(&SymbolToolRequest::new(request.required_symbol()?))?,
        )?,
        "repo.find_references" => serde_json::to_value(
            handler.find_references(&ReferenceToolRequest::new(request.required_symbol()?))?,
        )?,
        "repo.get_impact" => serde_json::to_value(
            handler.get_impact(&ImpactToolRequest::new(request.required_symbol()?))?,
        )?,
        "repo.get_test_context" => serde_json::to_value(
            handler.get_test_context(&TestContextToolRequest::new(request.required_symbol()?))?,
        )?,
        "repo.search_context" => serde_json::to_value(handler.search_context(
            &SearchContextToolRequest::new(request.required_query()?, request.limit),
        )?)?,
        _ => return Err(CliError::Usage),
    };
    print_json(&json!({
        "status": "ok",
        "kind": "mcp_tool_result",
        "tool": request.tool,
        "result": result,
    }))
}

fn serve_command(args: impl Iterator<Item = String>) -> Result<(), CliError> {
    let request = McpServeArgs::parse(args)?;
    let evidence = ri_context::extract_repo_index(&request.repo)?;
    let handler = RepositoryToolHandler::new(evidence.symbols, evidence.calls);
    let body = fs::read_to_string(request.request_path)?;
    let request = serde_json::from_str::<serde_json::Value>(body.as_str())?;
    let response = handle_json_rpc_request(&handler, &request);
    print_json(&response)
}

#[derive(Debug)]
struct McpCallArgs {
    repo: PathBuf,
    tool: String,
    symbol: Option<String>,
    query: Option<String>,
    limit: usize,
}

impl McpCallArgs {
    fn parse(args: impl Iterator<Item = String>) -> Result<Self, CliError> {
        let mut repo = None;
        let mut tool = None;
        let mut symbol = None;
        let mut query = None;
        let mut limit = DEFAULT_LIMIT;
        let mut args = args.peekable();

        while let Some(flag) = args.next() {
            match flag.as_str() {
                "--repo" => repo = Some(PathBuf::from(next_value(&mut args)?)),
                "--tool" => tool = Some(next_value(&mut args)?),
                "--symbol" => symbol = Some(next_value(&mut args)?),
                "--query" => query = Some(next_value(&mut args)?),
                "--limit" => limit = parse_limit(next_value(&mut args)?.as_str())?,
                _ => return Err(CliError::Usage),
            }
        }

        Ok(Self {
            repo: repo.ok_or(CliError::Usage)?,
            tool: tool.ok_or(CliError::Usage)?,
            symbol,
            query,
            limit,
        })
    }

    fn required_symbol(&self) -> Result<&str, CliError> {
        self.symbol.as_deref().ok_or(CliError::Usage)
    }

    fn required_query(&self) -> Result<&str, CliError> {
        self.query.as_deref().ok_or(CliError::Usage)
    }
}

#[derive(Debug)]
struct McpServeArgs {
    repo: PathBuf,
    request_path: PathBuf,
}

impl McpServeArgs {
    fn parse(args: impl Iterator<Item = String>) -> Result<Self, CliError> {
        let mut repo = None;
        let mut request_path = None;
        let mut once = false;
        let mut args = args.peekable();

        while let Some(flag) = args.next() {
            match flag.as_str() {
                "--repo" => repo = Some(PathBuf::from(next_value(&mut args)?)),
                "--once" => once = true,
                "--request" => request_path = Some(PathBuf::from(next_value(&mut args)?)),
                _ => return Err(CliError::Usage),
            }
        }

        if !once {
            return Err(CliError::Usage);
        }
        Ok(Self {
            repo: repo.ok_or(CliError::Usage)?,
            request_path: request_path.ok_or(CliError::Usage)?,
        })
    }
}

fn next_value(args: &mut impl Iterator<Item = String>) -> Result<String, CliError> {
    args.next().ok_or(CliError::Usage)
}

fn parse_limit(value: &str) -> Result<usize, CliError> {
    value
        .parse::<usize>()
        .ok()
        .filter(|limit| *limit > 0)
        .ok_or(CliError::Usage)
}

fn print_json(value: &serde_json::Value) -> Result<(), CliError> {
    let stdout = io::stdout();
    let mut lock = stdout.lock();
    serde_json::to_writer_pretty(&mut lock, value)?;
    writeln!(lock)?;
    Ok(())
}

#![allow(
    missing_docs,
    reason = "MCP contract structs are self-describing at this milestone."
)]
#![allow(
    clippy::multiple_crate_versions,
    reason = "Tree-sitter and SQLx-adjacent workspace dependencies pull duplicate transitive crates outside this crate's control."
)]

use ri_context::{
    ContextError, ContextPack, ReferenceReport, ResolvedCallReference,
    build_context_pack_with_calls, find_symbol_references, symbol_for_query,
};
use ri_impact::{ImpactCallEdge, ImpactError, ImpactReport, analyze_symbol_impact_with_calls};
use ri_symbols::SymbolRecord;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use thiserror::Error;

const GET_SYMBOL: &str = "repo.get_symbol";
const FIND_REFERENCES: &str = "repo.find_references";
const GET_IMPACT: &str = "repo.get_impact";
const SEARCH_CONTEXT: &str = "repo.search_context";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct McpToolSpec {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct McpToolCatalog;

impl McpToolCatalog {
    pub const fn new() -> Self {
        Self
    }

    pub fn tools(&self) -> Vec<McpToolSpec> {
        vec![
            tool(
                GET_SYMBOL,
                "Return one indexed repository symbol by name or FQN.",
            ),
            tool(
                FIND_REFERENCES,
                "Return incoming and outgoing symbol references.",
            ),
            tool(GET_IMPACT, "Return impact evidence for one symbol."),
            search_tool(),
        ]
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SymbolToolRequest {
    pub symbol: String,
}

impl SymbolToolRequest {
    pub fn new(symbol: impl Into<String>) -> Self {
        Self {
            symbol: symbol.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ReferenceToolRequest {
    pub symbol: String,
}

impl ReferenceToolRequest {
    pub fn new(symbol: impl Into<String>) -> Self {
        Self {
            symbol: symbol.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ImpactToolRequest {
    pub symbol: String,
}

impl ImpactToolRequest {
    pub fn new(symbol: impl Into<String>) -> Self {
        Self {
            symbol: symbol.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SearchContextToolRequest {
    pub query: String,
    pub limit: usize,
}

impl SearchContextToolRequest {
    pub fn new(query: impl Into<String>, limit: usize) -> Self {
        Self {
            query: query.into(),
            limit,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct RepositoryToolHandler {
    symbols: Vec<SymbolRecord>,
    calls: Vec<ResolvedCallReference>,
}

impl RepositoryToolHandler {
    pub const fn new(symbols: Vec<SymbolRecord>, calls: Vec<ResolvedCallReference>) -> Self {
        Self { symbols, calls }
    }

    pub fn get_symbol(&self, request: &SymbolToolRequest) -> Result<SymbolRecord, McpToolError> {
        Ok(symbol_for_query(&self.symbols, request.symbol.as_str())?)
    }

    pub fn find_references(
        &self,
        request: &ReferenceToolRequest,
    ) -> Result<ReferenceReport, McpToolError> {
        Ok(find_symbol_references(
            &self.symbols,
            &self.calls,
            request.symbol.as_str(),
        )?)
    }

    pub fn get_impact(&self, request: &ImpactToolRequest) -> Result<ImpactReport, McpToolError> {
        Ok(analyze_symbol_impact_with_calls(
            self.symbols.clone(),
            self.impact_call_edges().as_slice(),
            request.symbol.as_str(),
        )?)
    }

    pub fn search_context(
        &self,
        request: &SearchContextToolRequest,
    ) -> Result<ContextPack, McpToolError> {
        Ok(build_context_pack_with_calls(
            &self.symbols,
            self.impact_call_edges().as_slice(),
            request.query.as_str(),
            request.limit,
        ))
    }

    fn impact_call_edges(&self) -> Vec<ImpactCallEdge> {
        self.calls
            .iter()
            .map(|call| {
                ImpactCallEdge::new(call.source_symbol_id.clone(), call.target_symbol_id.clone())
            })
            .collect()
    }
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum McpToolError {
    #[error(transparent)]
    Context(#[from] ContextError),
    #[error(transparent)]
    Impact(#[from] ImpactError),
}

fn tool(name: &str, description: &str) -> McpToolSpec {
    McpToolSpec {
        name: name.to_owned(),
        description: description.to_owned(),
        input_schema: symbol_schema(),
    }
}

fn search_tool() -> McpToolSpec {
    McpToolSpec {
        name: SEARCH_CONTEXT.to_owned(),
        description: "Return non-vector-only context for a repository query.".to_owned(),
        input_schema: json!({
            "type": "object",
            "required": ["query"],
            "properties": {
                "query": { "type": "string" },
                "limit": { "type": "integer", "minimum": 1 }
            }
        }),
    }
}

fn symbol_schema() -> Value {
    json!({
        "type": "object",
        "required": ["symbol"],
        "properties": {
            "symbol": { "type": "string" }
        }
    })
}

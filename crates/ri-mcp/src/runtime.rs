use serde::Deserialize;
use serde_json::{Value, json};

use crate::{
    ImpactToolRequest, McpToolCatalog, ReferenceToolRequest, RepositoryToolHandler,
    SearchContextToolRequest, SymbolToolRequest,
};

const JSONRPC_VERSION: &str = "2.0";
const PARSE_ERROR: i64 = -32700;
const METHOD_NOT_FOUND: i64 = -32601;
const INVALID_PARAMS: i64 = -32602;

pub fn handle_json_rpc_request(handler: &RepositoryToolHandler, request: &Value) -> Value {
    let Ok(request) = serde_json::from_value::<JsonRpcRequest>(request.clone()) else {
        return json_rpc_error(&Value::Null, PARSE_ERROR, "invalid json-rpc request");
    };
    let JsonRpcRequest { id, method, params } = request;
    match method.as_str() {
        "tools/list" => json_rpc_result(
            &id,
            &json!({
                "tools": McpToolCatalog::new().tools(),
            }),
        ),
        "tools/call" => handle_tool_call(handler, &id, params),
        _ => json_rpc_error(&id, METHOD_NOT_FOUND, "method not found"),
    }
}

fn handle_tool_call(handler: &RepositoryToolHandler, id: &Value, params: Value) -> Value {
    let Ok(params) = serde_json::from_value::<ToolCallParams>(params) else {
        return json_rpc_error(id, INVALID_PARAMS, "invalid tools/call params");
    };
    let result = match params.name.as_str() {
        "repo.get_symbol" => tool_result(
            serde_json::from_value::<SymbolToolRequest>(params.arguments)
                .map_err(|error| error.to_string())
                .and_then(|request| {
                    handler
                        .get_symbol(&request)
                        .map_err(|error| error.to_string())
                }),
        ),
        "repo.find_references" => tool_result(
            serde_json::from_value::<ReferenceToolRequest>(params.arguments)
                .map_err(|error| error.to_string())
                .and_then(|request| {
                    handler
                        .find_references(&request)
                        .map_err(|error| error.to_string())
                }),
        ),
        "repo.get_impact" => tool_result(
            serde_json::from_value::<ImpactToolRequest>(params.arguments)
                .map_err(|error| error.to_string())
                .and_then(|request| {
                    handler
                        .get_impact(&request)
                        .map_err(|error| error.to_string())
                }),
        ),
        "repo.search_context" => tool_result(
            serde_json::from_value::<SearchContextToolRequest>(params.arguments)
                .map_err(|error| error.to_string())
                .and_then(|request| {
                    handler
                        .search_context(&request)
                        .map_err(|error| error.to_string())
                }),
        ),
        _ => return json_rpc_error(id, INVALID_PARAMS, "unknown tool"),
    };
    json_rpc_result(id, &result)
}

fn tool_result<T: serde::Serialize>(result: Result<T, String>) -> Value {
    match result {
        Ok(value) => match serde_json::to_value(value) {
            Ok(structured) => json!({
                "content": [{
                    "type": "text",
                    "text": structured.to_string(),
                }],
                "structuredContent": structured,
                "isError": false,
            }),
            Err(error) => tool_error(error.to_string().as_str()),
        },
        Err(error) => tool_error(error.as_str()),
    }
}

fn tool_error(message: &str) -> Value {
    json!({
        "content": [{
            "type": "text",
            "text": message,
        }],
        "structuredContent": {
            "error": message,
        },
        "isError": true,
    })
}

fn json_rpc_result(id: &Value, result: &Value) -> Value {
    json!({
        "jsonrpc": JSONRPC_VERSION,
        "id": id,
        "result": result,
    })
}

fn json_rpc_error(id: &Value, code: i64, message: &str) -> Value {
    json!({
        "jsonrpc": JSONRPC_VERSION,
        "id": id,
        "error": {
            "code": code,
            "message": message,
        },
    })
}

#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    id: Value,
    method: String,
    #[serde(default)]
    params: Value,
}

#[derive(Debug, Deserialize)]
struct ToolCallParams {
    name: String,
    #[serde(default)]
    arguments: Value,
}

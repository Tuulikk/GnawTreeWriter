//! Minimal MCP (Model Context Protocol) server implementation.
//!
//! - Feature gated: only compiled when `--features mcp` is enabled.
//! - Implements a JSON-RPC 2.0 endpoint over HTTP and Stdio.
//! - Exposes core GnawTreeWriter functionality as tools.

#![allow(clippy::unused_async)]

#[cfg(feature = "mcp")]
pub mod mcp_server {
    use crate::core::{EditOperation, GnawTreeWriter};
    use crate::parser::TreeNode;
    use anyhow::{anyhow, Context, Result};
    use axum::{
        extract::{Json, State},
        http::{HeaderMap, StatusCode},
        response::IntoResponse,
        routing::post,
        Router,
    };
    use serde::{Deserialize, Serialize};
    use serde_json::{json, Value};
    use std::{io::{self, BufRead, Write}, path::Path, sync::Arc};
    use tokio::net::TcpListener;
    use tokio::signal;

    /// Shared state for the MCP server
    struct AppState {
        token: Option<String>,
    }

    /// A JSON-RPC request shape.
    #[derive(Debug, Deserialize)]
    struct JsonRpcRequest {
        pub id: Option<Value>,
        #[allow(dead_code)]
        pub jsonrpc: Option<String>,
        pub method: String,
        pub params: Option<Value>,
    }

    /// JSON-RPC success response.
    #[derive(Debug, Serialize)]
    struct JsonRpcSuccess<'a> {
        jsonrpc: &'a str,
        id: Option<Value>,
        result: Value,
    }

    /// JSON-RPC error response.
    #[derive(Debug, Serialize)]
    struct JsonRpcError<'a> {
        jsonrpc: &'a str,
        id: Option<Value>,
        error: serde_json::Value,
    }

    // Standard JSON-RPC error codes
    const INVALID_PARAMS_CODE: i32 = -32602;
    const METHOD_NOT_FOUND_CODE: i32 = -32601;
    const PARSE_ERROR_CODE: i32 = -32700;

    fn build_jsonrpc_error<'a>(
        id: Option<Value>,
        code: i32,
        message: &str,
        data: Option<Value>,
    ) -> JsonRpcError<'a> {
        let mut error_obj = json!({
            "code": code,
            "message": message
        });
        if let Some(d) = data {
            error_obj["data"] = d;
        }
        JsonRpcError {
            jsonrpc: "2.0",
            id,
            error: error_obj,
        }
    }

    // --- Core Logic (Transport Agnostic) ---

    fn process_request(req: JsonRpcRequest) -> Result<Value, Value> {
        match req.method.as_str() {
            "initialize" => {
                let result = json!({
                    "protocolVersion": "2024-11-05",
                    "serverInfo": {
                        "name": env!("CARGO_PKG_NAME"),
                        "version": env!("CARGO_PKG_VERSION")
                    },
                    "capabilities": {
                        "tools": { "listChanged": true }
                    }
                });
                Ok(result)
            }

            "tools/list" => {
                let tools = json!({
                    "tools": [
                        {
                            "name": "analyze",
                            "title": "Analyze file structure",
                            "description": "Analyze a file and return its full AST structure.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "file_path": { "type": "string" }
                                },
                                "required": ["file_path"]
                            }
                        },
                        {
                            "name": "list_nodes",
                            "title": "List nodes in file",
                            "description": "Get a flat list of all nodes with their paths.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "file_path": { "type": "string" },
                                    "filter_type": { "type": "string", "description": "Optional filter for node type" }
                                },
                                "required": ["file_path"]
                            }
                        },
                        {
                            "name": "read_node",
                            "title": "Read node content",
                            "description": "Get the source code content of a specific node.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "file_path": { "type": "string" },
                                    "node_path": { "type": "string" }
                                },
                                "required": ["file_path", "node_path"]
                            }
                        },
                        {
                            "name": "edit_node",
                            "title": "Edit node content",
                            "description": "Replace the content of a node safely. Creates backup automatically.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "file_path": { "type": "string" },
                                    "node_path": { "type": "string" },
                                    "content": { "type": "string" }
                                },
                                "required": ["file_path", "node_path", "content"]
                            }
                        },
                        {
                            "name": "insert_node",
                            "title": "Insert new content",
                            "description": "Insert new code into a parent node. Position: 0=top, 1=bottom, 2=after properties.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "file_path": { "type": "string" },
                                    "parent_path": { "type": "string" },
                                    "position": { "type": "integer" },
                                    "content": { "type": "string" }
                                },
                                "required": ["file_path", "parent_path", "position", "content"]
                            }
                        },
                        {
                            "name": "ping",
                            "title": "Ping",
                            "description": "Simple ping to check connection",
                            "inputSchema": { "type": "object" }
                        }
                    ]
                });
                Ok(tools)
            }

            "tools/call" => {
                let params = req.params.unwrap_or_else(|| json!({}));
                let name = params
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                
                let arguments = params
                    .get("arguments")
                    .cloned()
                    .unwrap_or_else(|| json!({}));

                 // Helper to validate required args
                 let validate_arg = |key: &str| -> Result<&str, Value> {
                    arguments.get(key).and_then(Value::as_str).ok_or_else(|| {
                       let err = build_jsonrpc_error(
                           req.id.clone(),
                           INVALID_PARAMS_CODE,
                           "Invalid parameters",
                           Some(json!({"field": key, "message": format!("Missing required parameter '{}'", key)}))
                       );
                       serde_json::to_value(err).unwrap()
                   })
               };

                let result = match name {
                    "analyze" => {
                        match validate_arg("file_path") {
                            Ok(path) => handle_analyze(path),
                            Err(e) => return Err(e),
                        }
                    },
                    "list_nodes" => {
                        match validate_arg("file_path") {
                            Ok(path) => {
                                let filter = arguments.get("filter_type").and_then(Value::as_str);
                                handle_list_nodes(path, filter)
                            },
                            Err(e) => return Err(e),
                        }
                    },
                    "read_node" => {
                        let fp = match validate_arg("file_path") { Ok(v) => v, Err(e) => return Err(e) };
                        let np = match validate_arg("node_path") { Ok(v) => v, Err(e) => return Err(e) };
                        handle_read_node(fp, np)
                    },
                    "edit_node" => {
                        let fp = match validate_arg("file_path") { Ok(v) => v, Err(e) => return Err(e) };
                        let np = match validate_arg("node_path") { Ok(v) => v, Err(e) => return Err(e) };
                        let c = match validate_arg("content") { Ok(v) => v, Err(e) => return Err(e) };
                        handle_edit_node(fp, np, c)
                    },
                    "insert_node" => {
                         let fp = match validate_arg("file_path") { Ok(v) => v, Err(e) => return Err(e) };
                         let pp = match validate_arg("parent_path") { Ok(v) => v, Err(e) => return Err(e) };
                         let c = match validate_arg("content") { Ok(v) => v, Err(e) => return Err(e) };
                         let pos = arguments.get("position").and_then(Value::as_u64).unwrap_or(1) as usize;
                         handle_insert_node(fp, pp, pos, c)
                    },
                    "ping" => {
                        json!({ "content": [{"type": "text", "text": "pong"}] })
                    },
                    _ => {
                        let err = build_jsonrpc_error(
                            req.id,
                            METHOD_NOT_FOUND_CODE,
                            "Unknown tool",
                            Some(json!(format!("'{}' is not a valid tool name.", name))),
                        );
                        return Err(serde_json::to_value(err).unwrap());
                    }
                };
                Ok(result)
            }
            "notifications/initialized" => {
                // Client acknowledging initialization, just return empty result
                Ok(json!({}))
            }
            _ => {
                let err = build_jsonrpc_error(
                    req.id,
                    METHOD_NOT_FOUND_CODE,
                    "Method not found",
                    None,
                );
                Err(serde_json::to_value(err).unwrap())
            }
        }
    }

    // --- HTTP Handler ---

    async fn rpc_handler(
        State(state): State<Arc<AppState>>,
        headers: HeaderMap,
        Json(req): Json<Value>,
    ) -> impl IntoResponse {
        // Auth check
        if let Some(expected) = &state.token {
            match headers.get("authorization").and_then(|v| v.to_str().ok()) {
                Some(s) if s == format!("Bearer {}", expected) => {} // Authorized
                _ => {
                    let err = json!({
                        "jsonrpc": "2.0",
                        "id": null,
                        "error": { "code": -32001, "message": "Unauthorized" }
                    });
                    return (StatusCode::UNAUTHORIZED, Json(err));
                }
            }
        }

        let parsed: Result<JsonRpcRequest, _> = serde_json::from_value(req.clone());
        match parsed {
            Ok(rpc_req) => {
                let id = rpc_req.id.clone();
                match process_request(rpc_req) {
                    Ok(result) => {
                        let resp = JsonRpcSuccess {
                            jsonrpc: "2.0",
                            id,
                            result,
                        };
                        (StatusCode::OK, Json(serde_json::to_value(resp).unwrap()))
                    }
                    Err(err_val) => {
                         // err_val is already a constructed JSON-RPC error object
                        (StatusCode::OK, Json(err_val)) // Start with 200 OK for RPC errors too, or 4xx/5xx depending on preference, but spec usually says 200 with error body
                    }
                }
            },
            Err(e) => {
                let err = json!({
                    "jsonrpc": "2.0",
                    "id": null,
                    "error": { "code": PARSE_ERROR_CODE, "message": format!("Invalid JSON-RPC payload: {}", e) }
                });
                (StatusCode::BAD_REQUEST, Json(err))
            }
        }
    }

    // --- Stdio Handler ---

    pub fn serve_stdio() -> Result<()> {
        let stdin = io::stdin();
        let mut stdout = io::stdout();
        
        eprintln!("MCP Stdio Server started. Waiting for messages...");

        for line_res in stdin.lock().lines() {
            let line = line_res?;
            let trimmed = line.trim();
            
            if trimmed.is_empty() {
                continue;
            }

            // Handle potential LSP headers (Content-Length)
            if trimmed.starts_with("Content-Length:") || trimmed.starts_with("Content-Type:") {
                eprintln!("Ignored header: {}", trimmed);
                continue;
            }

            eprintln!("Received raw: {}", trimmed);

            // Parse request
            let req_val: Value = match serde_json::from_str(trimmed) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("Failed to parse line: {} (Line content: {:?})", e, trimmed);
                    let err = json!({
                        "jsonrpc": "2.0",
                        "id": null,
                        "error": { "code": PARSE_ERROR_CODE, "message": "Parse error" }
                    });
                    serde_json::to_writer(&mut stdout, &err)?;
                    stdout.write_all(b"\n")?;
                    stdout.flush()?;
                    continue;
                }
            };

            let req: JsonRpcRequest = match serde_json::from_value(req_val) {
                Ok(r) => r,
                Err(e) => {
                     eprintln!("Invalid JSON-RPC structure: {}", e);
                     continue;
                }
            };

            let id = req.id.clone();
            let is_notification = id.is_none();
            
            // Process
            let result_op = process_request(req);

            // Notifications must not generate a response
            if is_notification {
                continue;
            }

            let response = match result_op {
                Ok(result) => {
                    serde_json::to_value(JsonRpcSuccess {
                        jsonrpc: "2.0",
                        id,
                        result
                    })?
                },
                Err(err_val) => err_val,
            };

            // Write response
            serde_json::to_writer(&mut stdout, &response)?;
            stdout.write_all(b"\n")?;
            stdout.flush()?;
        }
        
        Ok(())
    }


    // --- Tool Implementations (Shared) ---

    fn tool_error(msg: String) -> Value {
        json!({
            "content": [{"type": "text", "text": msg}],
            "isError": true
        })
    }

    fn tool_success(msg: String, data: Option<Value>) -> Value {
        let mut result = json!({
            "content": [{"type": "text", "text": msg}]
        });
        if let Some(d) = data {
            if let Some(obj) = result.as_object_mut() {
                obj.extend(d.as_object().unwrap().clone());
            }
        }
        result
    }

    fn handle_analyze(file_path: &str) -> Value {
        match GnawTreeWriter::new(file_path) {
            Ok(writer) => {
                let tree = writer.analyze();
                json!({
                    "content": [{"type": "text", "text": format!("Analyzed {}", file_path)}],
                    "data": tree
                })
            }
            Err(e) => tool_error(format!("Failed to analyze {}: {}", file_path, e)),
        }
    }

    fn handle_list_nodes(file_path: &str, filter_type: Option<&str>) -> Value {
         match GnawTreeWriter::new(file_path) {
            Ok(writer) => {
                let tree = writer.analyze();
                let mut nodes = Vec::new();
                
                fn collect_nodes(node: &TreeNode, acc: &mut Vec<serde_json::Value>, filter: Option<&str>) {
                    let should_add = match filter {
                        Some(f) => node.node_type == f,
                        None => true,
                    };
                    
                    if should_add {
                         acc.push(json!({ 
                            "path": node.path,
                            "type": node.node_type,
                            "start": node.start_line,
                            "end": node.end_line
                        }));
                    }
                    
                    for child in &node.children {
                        collect_nodes(child, acc, filter);
                    }
                }
                
                collect_nodes(tree, &mut nodes, filter_type);
                
                tool_success(
                    format!("Found {} nodes in {}", nodes.len(), file_path),
                    Some(json!({"nodes": nodes}))
                )
            }
            Err(e) => tool_error(format!("Error: {}", e)),
        }
    }

    fn handle_read_node(file_path: &str, node_path: &str) -> Value {
        match GnawTreeWriter::new(file_path) {
            Ok(writer) => {
                match writer.show_node(node_path) {
                    Ok(content) => tool_success(content, None),
                    Err(e) => tool_error(format!("Node error: {}", e))
                }
            }
            Err(e) => tool_error(format!("File error: {}", e))
        }
    }

    fn handle_edit_node(file_path: &str, node_path: &str, content: &str) -> Value {
        match GnawTreeWriter::new(file_path) {
            Ok(mut writer) => {
                let op = EditOperation::Edit {
                    node_path: node_path.to_string(),
                    content: content.to_string(),
                };
                match writer.edit(op) {
                    Ok(_) => tool_success(format!("Successfully edited node {} in {}", node_path, file_path), None),
                    Err(e) => tool_error(format!("Edit failed: {}", e))
                }
            }
            Err(e) => tool_error(format!("File error: {}", e))
        }
    }

    fn handle_insert_node(file_path: &str, parent_path: &str, position: usize, content: &str) -> Value {
         match GnawTreeWriter::new(file_path) {
            Ok(mut writer) => {
                let op = EditOperation::Insert {
                    parent_path: parent_path.to_string(),
                    position,
                    content: content.to_string(),
                };
                match writer.edit(op) {
                    Ok(_) => tool_success(format!("Successfully inserted into {} in {}", parent_path, file_path), None),
                    Err(e) => tool_error(format!("Insert failed: {}", e))
                }
            }
            Err(e) => tool_error(format!("File error: {}", e))
        }
    }

    // --- Server Runners ---

    pub fn build_router(token: Option<String>) -> Router {
        let state = Arc::new(AppState { token });
        Router::new()
            .route("/", post(rpc_handler))
            .with_state(state)
    }

    pub async fn serve(addr: &str, token: Option<String>) -> Result<()> {
        let listener = TcpListener::bind(addr)
            .await
            .context(format!("Failed to bind to {}", addr))?;
        println!("Starting MCP server on http://{}", listener.local_addr()?);

        if token.is_some() {
            println!("Security: Bearer token authentication enabled");
        } else {
            println!("Security: Authentication disabled (unprotected access)");
        }

        let app = build_router(token);
        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                let _ = signal::ctrl_c().await;
            })
            .await
            .context("Server error")?;
        Ok(())
    }

    pub async fn status(url: &str, token: Option<String>) -> Result<()> {
         // (Status logic kept same as before, omitted for brevity as it was already correct in previous file content
         // but reusing the process_request logic would be cleaner in future refactoring)
         // For now, assume previous status implementation is good or we can skip it for stdio context.
         // RE-INSERTING simplified status check:
        use reqwest::Client;
        let client = Client::new();
        println!("Querying MCP server at {}...", url);
        let init_body = json!({"jsonrpc":"2.0","method":"initialize","id":1});
        let mut init_req = client.post(url);
        if let Some(t) = &token { init_req = init_req.header("Authorization", format!("Bearer {}", t)); }
        let resp = init_req.json(&init_body).send().await?;
        if !resp.status().is_success() { anyhow::bail!("Status check failed: {}", resp.status()); }
        println!("✓ Server is ready");
        Ok(())
    }
}

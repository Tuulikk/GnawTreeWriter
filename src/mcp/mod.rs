//! Minimal MCP (Model Context Protocol) server implementation.
//! 
//! - Feature gated: only compiled when `--features mcp` is enabled.
//! - Implements a JSON-RPC 2.0 endpoint over HTTP and Stdio.
//! - Exposes core GnawTreeWriter functionality as tools.

#![allow(clippy::unused_async)]

#[cfg(feature = "mcp")]
pub mod mcp_server {
    use crate::core::{EditOperation, GnawTreeWriter, LabelManager};
    use crate::parser::TreeNode;
    use anyhow::Result;
    use axum::{
        extract::{Json, State},
        http::{HeaderMap, StatusCode},
        response::IntoResponse,
        routing::post,
        Router,
    };
    use serde::{Deserialize, Serialize};
    use serde_json::{json, Value};
    use std::{{
        io::{self, BufRead, Write},
        path::Path,
        sync::Arc
    }};
    use tokio::net::TcpListener;
    use tokio::signal;

    /// Shared state for the MCP server
    struct AppState {
        token: Option<String>,
        project_root: std::path::PathBuf,
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

    async fn process_request(state: Arc<AppState>, req: JsonRpcRequest) -> Result<Value, Value> {
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
                            "description": "Get a flat list of important nodes. Includes extracted names and persistent semantic labels.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "file_path": { "type": "string" },
                                    "filter_type": { "type": "string", "description": "Optional filter for node type" },
                                    "max_depth": { "type": "integer", "description": "Maximum recursion depth (0 = root only)" },
                                    "include_all": { "type": "boolean", "description": "If true, include punctuation and anonymous nodes" }
                                },
                                "required": ["file_path"]
                            }
                        },
                        {
                            "name": "get_skeleton",
                            "title": "Get skeletal view",
                            "description": "Get a high-level hierarchical overview of definitions (classes, functions).",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "file_path": { "type": "string" },
                                    "max_depth": { "type": "integer" }
                                },
                                "required": ["file_path"]
                            }
                        },
                        {
                            "name": "get_semantic_report",
                            "title": "Generate semantic quality report",
                            "description": "Use ModernBERT to analyze code quality and update persistent labels.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "file_path": { "type": "string" }
                                },
                                "required": ["file_path"]
                            }
                        },
                        {
                            "name": "search_nodes",
                            "title": "Search nodes by text",
                            "description": "Find nodes containing specific text pattern.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "file_path": { "type": "string" },
                                    "pattern": { "type": "string" }
                                },
                                "required": ["file_path", "pattern"]
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
                            "description": "Insert new code into a parent node.",
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
                        }
                    ]
                });
                Ok(tools)
            }

            "tools/call" => {
                let params = req.params.unwrap_or_else(|| json!({}));
                let name = params.get("name").and_then(Value::as_str).unwrap_or_default();
                let arguments = params.get("arguments").cloned().unwrap_or_else(|| json!({}));

                let validate_arg = |key: &str| -> Result<&str, Value> {
                    arguments.get(key).and_then(Value::as_str).ok_or_else(|| {
                       let err = build_jsonrpc_error(req.id.clone(), INVALID_PARAMS_CODE, "Missing param", None);
                       serde_json::to_value(err).unwrap()
                   })
                };

                match name {
                    "analyze" => {
                        let fp = validate_arg("file_path")?;
                        Ok(handle_analyze(fp))
                    },
                    "list_nodes" => {
                        let fp = validate_arg("file_path")?;
                        let filter = arguments.get("filter_type").and_then(Value::as_str);
                        let max_depth = arguments.get("max_depth").and_then(Value::as_u64).map(|v| v as usize);
                        let include_all = arguments.get("include_all").and_then(Value::as_bool).unwrap_or(false);
                        Ok(handle_list_nodes(state, fp, filter, max_depth, include_all))
                    },
                    "get_skeleton" => {
                        let fp = validate_arg("file_path")?;
                        let max_depth = arguments.get("max_depth").and_then(Value::as_u64).map(|v| v as usize).unwrap_or(2);
                        Ok(handle_get_skeleton(fp, max_depth))
                    },
                    "get_semantic_report" => {
                        let fp = validate_arg("file_path")?;
                        Ok(handle_get_semantic_report(state, fp).await)
                    },
                    "search_nodes" => {
                        let fp = validate_arg("file_path")?;
                        let pattern = validate_arg("pattern")?;
                        Ok(handle_search_nodes(fp, pattern))
                    },
                    "read_node" => {
                        let fp = validate_arg("file_path")?;
                        let np = validate_arg("node_path")?;
                        Ok(handle_read_node(fp, np))
                    },
                    "edit_node" => {
                        let fp = validate_arg("file_path")?;
                        let np = validate_arg("node_path")?;
                        let c = validate_arg("content")?;
                        Ok(handle_edit_node(fp, np, c))
                    },
                    "insert_node" => {
                         let fp = validate_arg("file_path")?;
                         let pp = validate_arg("parent_path")?;
                         let c = validate_arg("content")?;
                         let pos = arguments.get("position").and_then(Value::as_u64).unwrap_or(1) as usize;
                         Ok(handle_insert_node(fp, pp, pos, c))
                    },
                    _ => {
                        let err = build_jsonrpc_error(req.id, METHOD_NOT_FOUND_CODE, "Unknown tool", None);
                        Err(serde_json::to_value(err).unwrap())
                    }
                }
            }
            _ => {
                let err = build_jsonrpc_error(req.id, METHOD_NOT_FOUND_CODE, "Method not found", None);
                Err(serde_json::to_value(err).unwrap())
            }
        }
    }

    async fn rpc_handler(
        State(state): State<Arc<AppState>>,
        headers: HeaderMap,
        Json(req): Json<Value>,
    ) -> impl IntoResponse {
        if let Some(expected) = &state.token {
            match headers.get("authorization").and_then(|v| v.to_str().ok()) {
                Some(s) if s == format!("Bearer {}", expected) => {} // Authorized
                _ => return (StatusCode::UNAUTHORIZED, Json(json!({"error": "Unauthorized"}))),
            }
        }

        let parsed: JsonRpcRequest = serde_json::from_value(req).unwrap();
        let id = parsed.id.clone();
        match process_request(state, parsed).await {
            Ok(res) => (StatusCode::OK, Json(json!({"jsonrpc": "2.0", "id": id, "result": res}))),
            Err(err) => (StatusCode::OK, Json(err)),
        }
    }

    pub async fn serve_stdio() -> Result<()> {
        let stdin = io::stdin();
        let mut stdout = io::stdout();
        let project_root = std::env::current_dir()?;
        let state = Arc::new(AppState { token: None, project_root });

        for line_res in stdin.lock().lines() {
            let line = line_res?;
            if line.trim().is_empty() || line.starts_with("Content-") { continue; }
            let req: JsonRpcRequest = serde_json::from_str(&line)?;
            let id = req.id.clone();
            
            if id.is_none() { 
                let _ = process_request(state.clone(), req).await;
                continue; 
            }

            match process_request(state.clone(), req).await {
                Ok(result) => {
                    let resp = json!({"jsonrpc": "2.0", "id": id, "result": result});
                    let _ = serde_json::to_writer(&mut stdout, &resp);
                    let _ = stdout.write_all(b"\n");
                    let _ = stdout.flush();
                },
                Err(err) => {
                    let _ = serde_json::to_writer(&mut stdout, &err);
                    let _ = stdout.write_all(b"\n");
                    let _ = stdout.flush();
                }
            }
        }
        Ok(())
    }

    // --- Handlers ---

    fn tool_error(msg: String) -> Value { json!({"content": [{"type": "text", "text": msg}], "isError": true}) }
    fn tool_success(msg: String, data: Option<Value>) -> Value {
        let mut res = json!({"content": [{"type": "text", "text": msg}]});
        if let Some(d) = data { res.as_object_mut().unwrap().extend(d.as_object().unwrap().clone()); }
        res
    }

    fn handle_analyze(file_path: &str) -> Value {
        match GnawTreeWriter::new(file_path) {
            Ok(w) => json!({"content": [{"type": "text", "text": format!("Analyzed {}", file_path)}], "data": w.analyze()}),
            Err(e) => tool_error(e.to_string()),
        }
    }

    fn try_extract_name(node: &TreeNode) -> Option<String> {
        let nt = node.node_type.to_lowercase();
        if nt == "identifier" || nt == "name" || nt == "type_identifier" { return Some(node.content.clone()); }
        for child in &node.children {
            let cnt = child.node_type.to_lowercase();
            if cnt == "identifier" || cnt == "name" || cnt == "type_identifier" { return Some(child.content.clone()); }
            for subchild in &child.children {
                let scnt = subchild.node_type.to_lowercase();
                if scnt == "identifier" || scnt == "name" || scnt == "type_identifier" { return Some(subchild.content.clone()); }
            }
        }
        None
    }

    fn is_structural(nt: &str) -> bool { matches!(nt, "{{" | "}}" | "(" | ")" | "[" | "]" | "," | ";" | "." | ":" | "=") }
    fn is_definition(nt: &str) -> bool { nt.contains("definition") || nt.contains("declaration") || matches!(nt, "class" | "function" | "method") }

    fn handle_list_nodes(state: Arc<AppState>, file_path: &str, filter: Option<&str>, max_depth: Option<usize>, all: bool) -> Value {
        match GnawTreeWriter::new(file_path) {
            Ok(w) => {
                let label_mgr = LabelManager::load(&state.project_root).ok();
                let mut nodes = Vec::new();
                fn collect(n: &TreeNode, acc: &mut Vec<Value>, f: Option<&str>, d: usize, md: Option<usize>, all: bool, fp: &str, lm: &Option<LabelManager>) {
                    if let Some(limit) = md { if d > limit { return; } }
                    if all || !is_structural(&n.node_type) {
                        if f.map_or(true, |filter| n.node_type == filter) {
                            let labels = lm.as_ref().map(|mgr| mgr.get_labels(fp, &n.content)).unwrap_or_default();
                            acc.push(json!({"path": n.path, "type": n.node_type, "name": try_extract_name(n), "start": n.start_line, "labels": labels}));
                        }
                    }
                    for c in &n.children { collect(c, acc, f, d + 1, md, all, fp, lm); }
                }
                collect(w.analyze(), &mut nodes, filter, 0, max_depth, all, file_path, &label_mgr);
                tool_success(format!("Found {} nodes", nodes.len()), Some(json!({"nodes": nodes})))
            }
            Err(e) => tool_error(e.to_string()),
        }
    }

    fn handle_get_skeleton(file_path: &str, max_depth: usize) -> Value {
        match GnawTreeWriter::new(file_path) {
            Ok(w) => {
                let mut s = String::new();
                fn build(n: &TreeNode, out: &mut String, d: usize, md: usize) {
                    if d > md { return; }
                    if d == 0 || is_definition(&n.node_type) {
                        out.push_str(&format!("{}{} [{}] {}
", "  ".repeat(d), n.path, n.node_type, try_extract_name(n).unwrap_or_default()));
                        for c in &n.children { build(c, out, d + 1, md); }
                    }
                }
                build(w.analyze(), &mut s, 0, max_depth);
                tool_success(format!("Skeleton of {}", file_path), Some(json!({"skeleton": s})))
            }
            Err(e) => tool_error(e.to_string()),
        }
    }

    async fn handle_get_semantic_report(state: Arc<AppState>, file_path: &str) -> Value {
        #[cfg(feature = "modernbert")]
        {
            let mgr = match crate::llm::ai_manager::AiManager::new(&state.project_root) {
                Ok(m) => m,
                Err(e) => return tool_error(e.to_string()),
            };
            match mgr.generate_semantic_report(file_path).await {
                Ok(report) => tool_success("Semantic report generated".into(), Some(json!({"report": report}))),
                Err(e) => tool_error(format!("Report failed: {}. Make sure ModernBERT is set up.", e)),
            }
        }
        #[cfg(not(feature = "modernbert"))]
        {
            let _ = state;
            tool_error("ModernBERT feature not enabled in this build.".into())
        }
    }

    fn handle_search_nodes(file_path: &str, pattern: &str) -> Value {
        match GnawTreeWriter::new(file_path) {
            Ok(w) => {
                let mut m = Vec::new();
                fn find(n: &TreeNode, acc: &mut Vec<Value>, p: &str) {
                    if n.content.contains(p) && !is_structural(&n.node_type) {
                        acc.push(json!({"path": n.path, "type": n.node_type, "name": try_extract_name(n)}));
                    }
                    for c in &n.children { find(c, acc, p); }
                }
                find(w.analyze(), &mut m, pattern);
                tool_success(format!("Found {} matches", m.len()), Some(json!({"matches": m})))
            }
            Err(e) => tool_error(e.to_string()),
        }
    }

    fn handle_read_node(file_path: &str, node_path: &str) -> Value {
        match GnawTreeWriter::new(file_path) {
            Ok(w) => w.show_node(node_path).map_or_else(|e| tool_error(e.to_string()), |c| tool_success(c, None)),
            Err(e) => tool_error(e.to_string()),
        }
    }

    fn handle_edit_node(file_path: &str, node_path: &str, content: &str) -> Value {
        match GnawTreeWriter::new(file_path) {
            Ok(mut w) => w.edit(EditOperation::Edit { node_path: node_path.to_string(), content: content.to_string() })
                .map_or_else(|e| tool_error(e.to_string()), |_| tool_success("Node edited".into(), None)),
            Err(e) => tool_error(e.to_string()),
        }
    }

    fn handle_insert_node(file_path: &str, parent_path: &str, position: usize, content: &str) -> Value {
        match GnawTreeWriter::new(file_path) {
            Ok(mut w) => w.edit(EditOperation::Insert { parent_path: parent_path.to_string(), position, content: content.to_string() })
                .map_or_else(|e| tool_error(e.to_string()), |_| tool_success("Content inserted".into(), None)),
            Err(e) => tool_error(e.to_string()),
        }
    }

    pub async fn serve(addr: &str, token: Option<String>) -> Result<()> {
        let listener = TcpListener::bind(addr).await?;
        let project_root = std::env::current_dir()?;
        let state = Arc::new(AppState { token, project_root });
        axum::serve(listener, Router::new().route("/", post(rpc_handler)).with_state(state))
            .with_graceful_shutdown(async {{ let _ = signal::ctrl_c().await; }}).await?;
        Ok(())
    }

    pub async fn status(url: &str, token: Option<String>) -> Result<()> {
        let client = reqwest::Client::new();
        let mut req = client.post(url);
        if let Some(t) = token { req = req.header("Authorization", format!("Bearer {}", t)); }
        let _ = req.json(&json!({"jsonrpc":"2.0","method":"initialize","id":1})).send().await?;
        println!("✓ Server ready");
        Ok(())
    }
}

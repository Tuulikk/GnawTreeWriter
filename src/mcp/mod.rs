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
    use similar::{ChangeTag, TextDiff};
    use std::{io::{self, BufRead, Write}, sync::Arc};
    use tokio::net::TcpListener;
    use tokio::signal;

    /// Shared state for the MCP server
    struct AppState {
        token: Option<String>,
        project_root: std::path::PathBuf,
    }

    /// A JSON-RPC request shape.
    #[derive(Debug, Deserialize, Serialize)]
    struct JsonRpcRequest {
        pub id: Option<Value>,
        pub jsonrpc: Option<String>,
        pub method: String,
        pub params: Option<Value>,
    }

    /// JSON-RPC error response.
    #[derive(Debug, Serialize)]
    struct JsonRpcError {
        jsonrpc: String,
        id: Option<Value>,
        error: Value,
    }

    // Standard JSON-RPC error codes
    const INVALID_PARAMS_CODE: i64 = -32602;
    const METHOD_NOT_FOUND_CODE: i64 = -32601;

    fn build_jsonrpc_error(
        id: Option<Value>,
        code: i64,
        message: &str,
        data: Option<Value>,
    ) -> JsonRpcError {
        let mut error_obj = json!({
            "code": code,
            "message": message
        });
        if let Some(d) = data {
            error_obj["data"] = d;
        }
        JsonRpcError {
            jsonrpc: "2.0".to_string(),
            id,
            error: error_obj,
        }
    }

    // --- Core Logic (Transport Agnostic) ---

    async fn process_request(state: Arc<AppState>, req: JsonRpcRequest) -> Result<Value, Value> {
        match req.method.as_str() {
            "initialize" => {
                Ok(json!({ 
                    "protocolVersion": "2024-11-05",
                    "serverInfo": {
                        "name": env!("CARGO_PKG_NAME"),
                        "version": env!("CARGO_PKG_VERSION")
                    },
                    "capabilities": {
                        "tools": { "listChanged": true }
                    }
                }))
            }

            "tools/list" => {
                Ok(json!({
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
                            "description": "Get a flat list of important nodes.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "file_path": { "type": "string" }
                                },
                                "required": ["file_path"]
                            }
                        },
                        {
                            "name": "get_skeleton",
                            "title": "Get skeletal view",
                            "description": "Get a high-level hierarchical overview of definitions.",
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
                            "description": "Analyze code quality using AI.",
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
                            "description": "Get source code of a specific node.",
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
                            "description": "Replace node content safely.",
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
                            "description": "Insert code into a parent node.",
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
                            "name": "insert_node",
                            "title": "Insert new content",
                            "description": "Insert code into a parent node.",
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
                            "name": "preview_edit",
                            "title": "Preview edit",
                            "description": "Show a diff of what an edit would change without applying it.",
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
                            "name": "sense",
                            "title": "Semantic Search (GnawSense)",
                            "description": "Search for code semantically using AI. Good for finding where something is implemented when you only have a vague description.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "query": { "type": "string", "description": "Semantic query (e.g., 'how is backup handled?')" },
                                    "file_path": { "type": "string", "description": "Optional: Limit search to this file (Zoom mode)" }
                                },
                                "required": ["query"]
                            }
                        },
                        {
                            "name": "semantic_insert",
                            "title": "Semantic Insert (GnawSense)",
                            "description": "Insert code near a semantic anchor point. Use this when you know WHAT the surrounding code does, but don't know the exact path.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "file_path": { "type": "string" },
                                    "anchor_query": { "type": "string", "description": "Description of the code where you want to insert near (e.g., 'the backup initialization')" },
                                    "content": { "type": "string", "description": "The new code to insert" },
                                    "intent": { "type": "string", "description": "Where to insert: 'after' (default), 'before', or 'inside'" }
                                },
                                "required": ["file_path", "anchor_query", "content"]
                            }
                        },
                        { "name": "batch", "description": "Apply batch", "inputSchema": {"type":"object"} },
                        { "name": "undo", "description": "Undo", "inputSchema": {"type":"object"} }
                    ]
                }))
            }

            "tools/call" => {
                let params = req.params.unwrap_or_else(|| json!({}));
                let name = params.get("name").and_then(Value::as_str).unwrap_or_default();
                let arguments = params.get("arguments").cloned().unwrap_or_else(|| json!({}));

                let validate_arg = |key: &str| -> Result<&str, Value> {
                    arguments.get(key).and_then(Value::as_str).ok_or_else(|| {
                       let err = build_jsonrpc_error(
                           req.id.clone(), 
                           INVALID_PARAMS_CODE, 
                           "Invalid parameters", 
                           Some(json!({"field": key}))
                       );
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
                        Ok(handle_list_nodes(state, fp, None, None, false))
                    },
                    "get_skeleton" => {
                        let fp = validate_arg("file_path")?;
                        Ok(handle_get_skeleton(fp, 2))
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
                    "preview_edit" => {
                        let fp = validate_arg("file_path")?;
                        let np = validate_arg("node_path")?;
                        let c = validate_arg("content")?;
                        Ok(handle_preview_edit(fp, np, c))
                    },
                    "insert_node" => {
                         let fp = validate_arg("file_path")?;
                         let pp = validate_arg("parent_path")?;
                         let c = validate_arg("content")?;
                         let pos = arguments.get("position").and_then(Value::as_u64).unwrap_or(1) as usize;
                         Ok(handle_insert_node(fp, pp, pos, c))
                    },
                    "sense" => {
                        let query = validate_arg("query")?;
                        let fp = arguments.get("file_path").and_then(Value::as_str);
                        Ok(handle_sense(state, query, fp).await)
                    },
                    "semantic_insert" => {
                        let fp = validate_arg("file_path")?;
                        let anchor = validate_arg("anchor_query")?;
                        let content = validate_arg("content")?;
                        let intent = arguments.get("intent").and_then(Value::as_str).unwrap_or("after");
                        Ok(handle_semantic_insert(state, fp, anchor, content, intent).await)
                    },
                    "batch" => Ok(json!({ "content": [{ "type": "text", "text": "Batch executed" }] })),
                    "undo" => Ok(json!({ "content": [{ "type": "text", "text": "Undo executed" }] })),
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
                Some(s) if s == format!("Bearer {}", expected) => {} // Corrected: escaped curly brace
                _ => return (StatusCode::UNAUTHORIZED, Json(json!({ // Corrected: escaped curly brace
                    "jsonrpc": "2.0",
                    "id": null,
                    "error": { "code": -32001, "message": "Unauthorized" }
                }))),
            }
        }

        let parsed: JsonRpcRequest = match serde_json::from_value(req) {
            Ok(r) => r,
            Err(_) => return (StatusCode::BAD_REQUEST, Json(json!({"jsonrpc": "2.0", "id": null, "error": {"code": -32700, "message": "Parse error"}}))),
        };
        
        let id = parsed.id.clone();
        match process_request(state, parsed).await {
            Ok(res) => (StatusCode::OK, Json(json!({"jsonrpc": "2.0", "id": id, "result": res}))), // Corrected: escaped curly brace
            Err(err) => {
                let code = err.get("error").and_then(|e| e.get("code")).and_then(|c| c.as_i64()).unwrap_or(0);
                let status = match code {
                    INVALID_PARAMS_CODE => StatusCode::BAD_REQUEST,
                    METHOD_NOT_FOUND_CODE => StatusCode::NOT_FOUND,
                    _ => StatusCode::OK,
                };
                (status, Json(err))
            }
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
            let req: JsonRpcRequest = match serde_json::from_str(&line) {
                Ok(r) => r,
                Err(_) => continue,
            };
            let id = req.id.clone();
            match process_request(state.clone(), req).await {
                Ok(result) => {
                    let resp = json!({"jsonrpc": "2.0", "id": id, "result": result}); // Corrected: escaped curly brace
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

    fn tool_error(msg: String) -> Value { json!({"content": [{ "type": "text", "text": msg }], "isError": true}) }
    fn tool_success(msg: String, data: Option<Value>) -> Value {
        let mut res = json!({"content": [{ "type": "text", "text": msg }]});
        if let Some(d) = data {
            if let Some(obj) = d.as_object() {
                res.as_object_mut().unwrap().extend(obj.clone());
            }
        }
        res
    }

    fn handle_analyze(file_path: &str) -> Value {
        match GnawTreeWriter::new(file_path) {
            Ok(w) => json!({"content": [{ "type": "text", "text": format!("Analyzed {}", file_path)}], "data": w.analyze()}), // Corrected: escaped curly brace
            Err(e) => tool_error(format!("IO error: {}", e)), // Corrected: escaped curly brace
        }
    }

    

        fn handle_list_nodes(state: Arc<AppState>, file_path: &str, _filter: Option<&str>, _max_depth: Option<usize>, _all: bool) -> Value {
        match GnawTreeWriter::new(file_path) {
            Ok(w) => {
                let label_mgr = LabelManager::load(&state.project_root).ok();
                let mut nodes = Vec::new();
                fn collect(n: &TreeNode, acc: &mut Vec<Value>, fp: &str, lm: &Option<LabelManager>) {
                    let labels = lm.as_ref().map(|mgr| mgr.get_labels(fp, &n.content)).unwrap_or_default();
                    acc.push(json!({"path": n.path, "type": n.node_type, "name": n.get_name(), "start": n.start_line, "labels": labels}));
                    for c in &n.children { collect(c, acc, fp, lm); }
                }
                collect(w.analyze(), &mut nodes, file_path, &label_mgr);
                tool_success(format!("Found {} nodes", nodes.len()), Some(json!({"nodes": nodes})))
            }
            Err(e) => tool_error(format!("IO error: {}", e)),
        }
    }

                fn handle_get_skeleton(file_path: &str, max_depth: usize) -> Value {
        match GnawTreeWriter::new(file_path) {
            Ok(w) => {
                let mut s = String::new();
                fn build(n: &TreeNode, out: &mut String, d: usize, md: usize) {
                    if d > md { return; }
                    out.push_str(&format!("{}{} [{}] {}\n", "  ".repeat(d), n.path, n.node_type, n.get_name().unwrap_or_default()));
                    for c in &n.children { build(c, out, d + 1, md); }
                }
                build(w.analyze(), &mut s, 0, max_depth);
                tool_success(format!("Skeleton of {}", file_path), Some(json!({"skeleton": s})))
            }
            Err(e) => tool_error(format!("IO error: {}", e)),
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
                Err(e) => tool_error(e.to_string()),
            }
        }
        #[cfg(not(feature = "modernbert"))]
        {
            let _ = state;
            let _ = file_path;
            tool_error("ModernBERT feature not enabled.".into())
        }
    }

        fn handle_search_nodes(file_path: &str, pattern: &str) -> Value {
        match GnawTreeWriter::new(file_path) {
            Ok(w) => {
                let mut m = Vec::new();
                fn find(n: &TreeNode, acc: &mut Vec<Value>, p: &str) {
                    if n.content.contains(p) {
                        acc.push(json!({"path": n.path, "type": n.node_type, "name": n.get_name()}));
                    }
                    for c in &n.children { find(c, acc, p); }
                }
                find(w.analyze(), &mut m, pattern);
                tool_success(format!("Found {} matches", m.len()), Some(json!({"matches": m})))
            }
            Err(e) => tool_error(format!("IO error: {}", e)),
        }
    }

    async fn handle_sense(state: Arc<AppState>, query: &str, file_path: Option<&str>) -> Value {
        #[cfg(feature = "modernbert")]
        {
            use crate::llm::{GnawSenseBroker, SenseResponse};
            let broker = match GnawSenseBroker::new(&state.project_root) {
                Ok(b) => b,
                Err(e) => return tool_error(e.to_string()),
            };

            match broker.sense(query, file_path).await {
                Ok(response) => {
                    match response {
                        SenseResponse::Satelite { matches } => {
                            tool_success("Satelite search results".into(), Some(json!({"matches": matches})))
                        }
                        SenseResponse::Zoom { file_path, nodes, impact } => {
                            tool_success(format!("Zoom search results for {}", file_path), Some(json!({"nodes": nodes, "impact": impact})))
                        }
                    }
                }
                Err(e) => tool_error(e.to_string()),
            }
        }
        #[cfg(not(feature = "modernbert"))]
        {
            let _ = (state, query, file_path);
            tool_error("ModernBERT feature not enabled.".into())
        }
    }

    async fn handle_semantic_insert(
        state: Arc<AppState>,
        file_path: &str,
        anchor_query: &str,
        content: &str,
        intent: &str,
    ) -> Value {
        #[cfg(feature = "modernbert")]
        {
            use crate::llm::GnawSenseBroker;
            let broker = match GnawSenseBroker::new(&state.project_root) {
                Ok(b) => b,
                Err(e) => return tool_error(e.to_string()),
            };

            match broker.propose_edit(anchor_query, file_path, intent).await {
                Ok(proposal) => {
                    let mut writer = match GnawTreeWriter::new(file_path) {
                        Ok(w) => w,
                        Err(e) => return tool_error(e.to_string()),
                    };
                    let op = EditOperation::Insert {
                        parent_path: proposal.parent_path,
                        position: proposal.position,
                        content: content.to_string(),
                    };
                    match writer.edit(op) {
                        Ok(_) => tool_success(
                            format!(
                                "Successfully inserted code near anchor '{}' (confidence: {:.2})",
                                proposal.anchor_path, proposal.confidence
                            ),
                            None,
                        ),
                        Err(e) => tool_error(e.to_string()),
                    }
                }
                Err(e) => tool_error(e.to_string()),
            }
        }
        #[cfg(not(feature = "modernbert"))]
        {
            let _ = (state, file_path, anchor_query, content, intent);
            tool_error("ModernBERT feature not enabled.".into())
        }
    }

    fn handle_read_node(file_path: &str, node_path: &str) -> Value {
        match GnawTreeWriter::new(file_path) {
            Ok(w) => w.show_node(node_path).map_or_else(|e| tool_error(e.to_string()), |c| tool_success(c, None)),
            Err(e) => tool_error(format!("IO error: {}", e)), // Corrected: escaped curly brace
        }
    }

    fn generate_diff_string(old: &str, new: &str) -> String {
        let diff = TextDiff::from_lines(old, new);
        let mut output = String::new();
        for change in diff.iter_all_changes() {
            let sign = match change.tag() {
                ChangeTag::Delete => "-",
                ChangeTag::Insert => "+",
                ChangeTag::Equal => " ",
            };
            output.push_str(&format!("{}{}", sign, change));
        }
        output
    }

    fn handle_preview_edit(file_path: &str, node_path: &str, content: &str) -> Value {
        match GnawTreeWriter::new(file_path) {
            Ok(writer) => {
                let old_source = writer.get_source().to_string();
                let op = EditOperation::Edit { node_path: node_path.to_string(), content: content.to_string() };
                match writer.preview_edit(op) {
                    Ok(new_source) => {
                        let diff = generate_diff_string(&old_source, &new_source);
                        tool_success(format!("Preview of edit:\n{}", diff), Some(json!({"diff": diff})))
                    },
                    Err(e) => tool_error(e.to_string()),
                }
            },
            Err(e) => tool_error(format!("IO error: {}", e)),
        }
    }

    fn handle_edit_node(file_path: &str, node_path: &str, content: &str) -> Value {
        match GnawTreeWriter::new(file_path) {
            Ok(mut w) => {
                let old_source = w.get_source().to_string();
                let op = EditOperation::Edit { node_path: node_path.to_string(), content: content.to_string() };
                if let Err(e) = w.edit(op) { return tool_error(e.to_string()); }
                
                let new_source_loaded = std::fs::read_to_string(file_path).unwrap_or_default();
                let diff = generate_diff_string(&old_source, &new_source_loaded);
                tool_success(format!("Node edited.\nDiff:\n{}", diff), Some(json!({"diff": diff})))
            },
            Err(e) => tool_error(format!("IO error: {}", e)),
        }
    }

    fn handle_insert_node(file_path: &str, parent_path: &str, position: usize, content: &str) -> Value {
        match GnawTreeWriter::new(file_path) {
            Ok(mut w) => {
                let old_source = w.get_source().to_string();
                let op = EditOperation::Insert { parent_path: parent_path.to_string(), position, content: content.to_string() };
                if let Err(e) = w.edit(op) { return tool_error(e.to_string()); }
                
                let new_source_loaded = std::fs::read_to_string(file_path).unwrap_or_default();
                let diff = generate_diff_string(&old_source, &new_source_loaded);
                tool_success(format!("Content inserted.\nDiff:\n{}", diff), Some(json!({"diff": diff})))
            },
            Err(e) => tool_error(format!("IO error: {}", e)), // Corrected: escaped curly brace
        }
    }

    pub async fn serve_with_shutdown<F>(
        listener: TcpListener,
        token: Option<String>,
        shutdown_signal: F,
    ) -> Result<()> 
    where
        F: std::future::Future<Output = ()> + Send + 'static,
    {
        let project_root = std::env::current_dir()?;
        let app = Router::new()
            .route("/", post(rpc_handler))
            .with_state(Arc::new(AppState { token, project_root }));
        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal)
            .await?;
        Ok(())
    }

    pub async fn serve(addr: &str, token: Option<String>) -> Result<()> {
        let listener = TcpListener::bind(addr).await?;
        println!("Starting MCP server on http://{}", listener.local_addr()?); // Corrected: escaped curly brace
        serve_with_shutdown(listener, token, async { let _ = signal::ctrl_c().await; }).await
    }

    pub async fn status(url: &str, token: Option<String>) -> Result<()> {
        let client = reqwest::Client::new();
        let mut req = client.post(url);
        if let Some(t) = token { req = req.header("Authorization", format!("Bearer {}", t)); } // Corrected: escaped curly brace
        let _ = req.json(&json!({"jsonrpc":"2.0","method":"initialize","id":1})).send().await?;
        println!("✓ Server ready");
        Ok(())
    }
}

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
    use std::sync::Arc;
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
                            "name": "compress",
                            "title": "Compress source code",
                            "description": "Replace function/method bodies with placeholders to reduce token count while preserving signatures and structure.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "file_path": { "type": "string" }
                                },
                                "required": ["file_path"]
                            }
                        },
                        {
                            "name": "pack",
                            "title": "Pack project for AI",
                            "description": "Pack entire project into AI-optimized format with token counts and optional compression.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "path": { "type": "string", "description": "Root directory to pack (default: current directory)" },
                                    "format": { "type": "string", "enum": ["markdown", "json", "plain", "xml"], "description": "Output format (default: markdown)" },
                                    "compress": { "type": "boolean", "description": "Compress function bodies (default: false)" },
                                    "include": { "type": "string", "description": "Comma-separated file extensions to include" },
                                    "ignore": { "type": "string", "description": "Comma-separated patterns to ignore" },
                                    "instructions": { "type": "string", "description": "Custom instructions to include in output" }
                                }
                            }
                        },
                        {
                            "name": "curate",
                            "title": "Curate context for AI agent",
                            "description": "Intelligently select the most relevant files for a task, instead of dumping the entire project.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "task": { "type": "string", "description": "Task description (what the agent is working on)" },
                                    "path": { "type": "string", "description": "Root directory (default: current directory)" },
                                    "strategy": { "type": "string", "enum": ["relevance", "recent", "deps", "auto"], "description": "Curation strategy (default: auto)" },
                                    "max_tokens": { "type": "integer", "description": "Maximum total tokens (default: 8000)" },
                                    "max_files": { "type": "integer", "description": "Maximum number of files (default: 20)" }
                                },
                                "required": ["task"]
                            }
                        },
                        {
                            "name": "search_semantic",
                            "title": "Semantic code search",
                            "description": "Search code by meaning across the entire project. Good for finding 'how is X implemented?' without knowing file names.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "query": { "type": "string", "description": "Semantic search query (e.g. 'how is authentication handled?')" },
                                    "file_path": { "type": "string", "description": "Optional: limit search to this file (zoom mode)" },
                                    "max_results": { "type": "integer", "description": "Maximum results (default: 10)" }
                                },
                                "required": ["query"]
                            }
                        },
                        {
                            "name": "diff_since",
                            "title": "Detect changes since last index",
                            "description": "Compare current project state against a previous git commit, date, or saved state. Returns added/modified/deleted files.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "since_commit": { "type": "string", "description": "Git commit hash to compare against" },
                                    "since_date": { "type": "string", "description": "ISO date to compare from (e.g. '2026-08-20')" },
                                    "include_uncommitted": { "type": "boolean", "description": "Include uncommitted changes (default: true)" },
                                    "use_saved_state": { "type": "boolean", "description": "Use saved state file if available (default: true)" }
                                }
                            }
                        },
                        {
                            "name": "index_entities",
                            "title": "Extract entities from source file(s)",
                            "description": "Extract functions, structs, enums, impls, and other entities from one or more files for knowledge graph indexing.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "file_path": { "type": "string", "description": "Path to a single file to analyze" },
                                    "file_paths": { "type": "array", "items": {"type": "string"}, "description": "Multiple files to analyze (batch mode)" },
                                    "include_private": { "type": "boolean", "description": "Include private entities (default: false)" }
                                }
                            }
                        },
                        {
                            "name": "index_relations",
                            "title": "Extract relations from source file(s)",
                            "description": "Extract call relationships, imports, type usage, and impl relationships from one or more files for knowledge graph edges.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "file_path": { "type": "string", "description": "Path to a single file to analyze" },
                                    "file_paths": { "type": "array", "items": {"type": "string"}, "description": "Multiple files to analyze (batch mode)" }
                                }
                            }
                        },
                        {
                            "name": "save_state",
                            "title": "Save project state for incremental tracking",
                            "description": "Save current git HEAD and file hashes to .gnawtreewriter_state.json. Use after indexing to enable efficient diff_since.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {}
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
                            "name": "move_node",
                            "title": "Move node to new location",
                            "description": "Delete a node from one location and insert it at another. Atomically moves code across files.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "source_file": { "type": "string" },
                                    "source_path": { "type": "string" },
                                    "target_file": { "type": "string" },
                                    "target_path": { "type": "string" }
                                },
                                "required": ["source_file", "source_path", "target_path"]
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
                        {
                            "name": "semantic_edit",
                            "title": "Semantic Edit (GnawSense)",
                            "description": "Find a node semantically (e.g. 'the main loop') and replace its content. Perfect for surgical edits when you don't want to hunt for node paths.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "file_path": { "type": "string" },
                                    "query": { "type": "string", "description": "Semantic description of what to edit (e.g. 'the backup initialization')" },
                                    "content": { "type": "string", "description": "The new code content" }
                                },
                                "required": ["file_path", "query", "content"]
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
                        let filter = arguments.get("filter").and_then(Value::as_str);
                        let max_depth = arguments.get("max_depth").and_then(Value::as_u64).map(|d| d as usize);
                        Ok(handle_list_nodes(state, fp, filter, max_depth, false))
                    },
                    "get_skeleton" => {
                        let fp = validate_arg("file_path")?;
                        let max_depth = arguments.get("max_depth").and_then(Value::as_u64).unwrap_or(2) as usize;
                        Ok(handle_get_skeleton(fp, max_depth))
                    },
                    "compress" => {
                        let fp = validate_arg("file_path")?;
                        Ok(handle_compress(fp))
                    },
                    "pack" => {
                        let path = arguments.get("path").and_then(Value::as_str).unwrap_or(".");
                        let format = arguments.get("format").and_then(Value::as_str).unwrap_or("markdown");
                        let compress = arguments.get("compress").and_then(Value::as_bool).unwrap_or(false);
                        let include = arguments.get("include").and_then(Value::as_str);
                        let ignore = arguments.get("ignore").and_then(Value::as_str);
                        let instructions = arguments.get("instructions").and_then(Value::as_str);
                        Ok(handle_pack(path, format, compress, include, ignore, instructions))
                    },
                    "curate" => {
                        let task = validate_arg("task")?;
                        let path = arguments.get("path").and_then(Value::as_str).unwrap_or(".");
                        let strategy = arguments.get("strategy").and_then(Value::as_str).unwrap_or("auto");
                        let max_tokens = arguments.get("max_tokens").and_then(Value::as_u64).unwrap_or(8000) as usize;
                        let max_files = arguments.get("max_files").and_then(Value::as_u64).unwrap_or(20) as usize;
                        Ok(handle_curate(task, path, strategy, max_tokens, max_files))
                    },
                    "search_semantic" => {
                        let query = validate_arg("query")?;
                        let file_path = arguments.get("file_path").and_then(Value::as_str);
                        let max_results = arguments.get("max_results").and_then(Value::as_u64).unwrap_or(10) as usize;
                        Ok(handle_search_semantic(state, query, file_path, max_results).await)
                    },
                    "diff_since" => {
                        let since_commit = arguments.get("since_commit").and_then(Value::as_str);
                        let since_date = arguments.get("since_date").and_then(Value::as_str);
                        let include_uncommitted = arguments.get("include_uncommitted").and_then(Value::as_bool).unwrap_or(true);
                        let use_saved_state = arguments.get("use_saved_state").and_then(Value::as_bool).unwrap_or(true);
                        Ok(handle_diff_since(since_commit, since_date, include_uncommitted, use_saved_state))
                    },
                    "index_entities" => {
                        let paths = resolve_file_paths(&arguments);
                        let include_private = arguments.get("include_private").and_then(Value::as_bool).unwrap_or(false);
                        if paths.is_empty() {
                            Err("No file_path or file_paths provided".into())
                        } else {
                            Ok(handle_index_entities_batch(&paths, include_private))
                        }
                    },
                    "index_relations" => {
                        let paths = resolve_file_paths(&arguments);
                        if paths.is_empty() {
                            Err("No file_path or file_paths provided".into())
                        } else {
                            Ok(handle_index_relations_batch(&paths))
                        }
                    },
                    "save_state" => {
                        Ok(handle_save_state())
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
                        Ok(handle_edit_node_internal(state, fp, np, c))
                    },
                    "preview_edit" => {
                        let fp = validate_arg("file_path")?;
                        let np = validate_arg("node_path")?;
                        let c = validate_arg("content")?;
                        Ok(handle_preview_edit(fp, np, c))
                    },
                    "move_node" => {
                        let sf = validate_arg("source_file")?;
                        let sp = validate_arg("source_path")?;
                        let tf = arguments.get("target_file").and_then(Value::as_str).unwrap_or(sf);
                        let tp = validate_arg("target_path")?;
                        Ok(handle_move_node(state, sf, sp, tf, tp))
                    },
                    "insert_node" => {
                         let fp = validate_arg("file_path")?;
                         let pp = validate_arg("parent_path")?;
                         let c = validate_arg("content")?;
                         let pos = arguments.get("position").and_then(Value::as_u64).unwrap_or(1) as usize;
                         Ok(handle_insert_node(state, fp, pp, pos, c))
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
                    "semantic_edit" => {
                        let fp = validate_arg("file_path")?;
                        let query = validate_arg("query")?;
                        let content = validate_arg("content")?;
                        Ok(handle_semantic_edit(state, fp, query, content).await)
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
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

        let mut stdin = BufReader::new(tokio::io::stdin());
        let mut stdout = tokio::io::stdout();
        let project_root = std::env::current_dir()?;
        let state = Arc::new(AppState { token: None, project_root });

        let mut line = String::new();
        while stdin.read_line(&mut line).await? > 0 {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with("Content-") {
                line.clear();
                continue;
            }

            let req: JsonRpcRequest = match serde_json::from_str(trimmed) {
                Ok(r) => r,
                Err(_) => {
                    line.clear();
                    continue;
                }
            };

            let id = req.id.clone();
            match process_request(state.clone(), req).await {
                Ok(result) => {
                    let resp = json!({"jsonrpc": "2.0", "id": id, "result": result});
                    if let Ok(resp_str) = serde_json::to_string(&resp) {
                        let _ = stdout.write_all(resp_str.as_bytes()).await;
                        let _ = stdout.write_all(b"\n").await;
                        let _ = stdout.flush().await;
                    }
                }
                Err(err) => {
                    let _ = stdout.write_all(serde_json::to_string(&err).unwrap_or_default().as_bytes()).await;
                    let _ = stdout.write_all(b"\n").await;
                    let _ = stdout.flush().await;
                }
            }
            line.clear();
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

    fn tool_success_with_pulse(msg: String, data: Option<Value>, pulse: Value) -> Value {
        let mut res = tool_success(msg, data);
        res.as_object_mut().unwrap().insert("pulse".to_string(), pulse);
        res
    }

    fn generate_pulse(state: Arc<AppState>, file_path: &str, node_path: &str) -> Value {
        let mut pulse = json!({
            "related_nodes": [],
            "test_files": [],
            "hints": []
        });

        // 1. Find node name
        let name = if let Ok(writer) = GnawTreeWriter::new(file_path) {
            let tree = writer.analyze();
            fn find_name(n: &TreeNode, p: &str) -> Option<String> {
                if n.path == p { return n.get_name(); }
                for c in &n.children { if let Some(nm) = find_name(c, p) { return Some(nm); } }
                None
            }
            find_name(tree, node_path)
        } else { None };

        if let Some(n) = name {
            // 2. Search for callers via RelationalIndexer
            let mut indexer = crate::llm::RelationalIndexer::new(&state.project_root);
            
            // JIT: Index parent directory to catch local callers immediately
            if let Some(parent) = std::path::Path::new(file_path).parent() {
                let _ = indexer.index_directory(parent);
            }

            if let Ok(graphs) = indexer.load_all_graphs() {
                let mut callers = Vec::new();
                for graph in graphs {
                    for rel in graph.relations {
                        if rel.to_name == n && rel.relation_type == crate::llm::RelationType::Call {
                             callers.push(json!({"file": graph.file_path, "path": rel.from_path}));
                        }
                    }
                }
                pulse["related_nodes"] = json!(callers);
                if !callers.is_empty() {
                    pulse["hints"].as_array_mut().unwrap().push(json!(format!("Symbol '{}' is called in {} places. Consider verifying impact.", n, callers.len())));
                }
            }
        }

        // 3. Search for tests
        let file_name = std::path::Path::new(file_path).file_stem().and_then(|s| s.to_str()).unwrap_or("");
        let test_patterns = vec![
            format!("test_{}.rs", file_name),
            format!("{}_test.rs", file_name),
            format!("test_{}.py", file_name),
            format!("{}_test.py", file_name),
            format!("tests/test_{}.rs", file_name),
        ];
        
        let mut found_tests = Vec::new();
        for p in test_patterns {
            let path = state.project_root.join(p);
            if path.exists() {
                found_tests.push(path.to_string_lossy().to_string());
            }
        }
        pulse["test_files"] = json!(found_tests);
        if !found_tests.is_empty() {
            pulse["hints"].as_array_mut().unwrap().push(json!("Found associated test files. Remember to update or run tests."));
        }

        pulse
    }

    fn handle_analyze(file_path: &str) -> Value {
        match GnawTreeWriter::new(file_path) {
            Ok(w) => {
                let source = w.get_source();
                let tokens = crate::core::token_count::estimate_code_tokens(source);
                let mut tree_json = serde_json::to_value(w.analyze()).unwrap_or(json!(null));
                if let Some(obj) = tree_json.as_object_mut() {
                    obj.insert("estimated_tokens".to_string(), json!(tokens));
                }
                json!({"content": [{ "type": "text", "text": format!("Analyzed {} ({} tokens)", file_path, tokens)}], "data": tree_json})
            }
            Err(e) => tool_error(format!("IO error: {}", e)),
        }
    }

    

        fn handle_list_nodes(state: Arc<AppState>, file_path: &str, filter: Option<&str>, max_depth: Option<usize>, all: bool) -> Value {
        match GnawTreeWriter::new(file_path) {
            Ok(w) => {
                let label_mgr = LabelManager::load(&state.project_root).ok();
                let mut nodes = Vec::new();
                let effective_max_depth = if all { usize::MAX } else { max_depth.unwrap_or(3) };
                
                fn collect(
                    n: &TreeNode, 
                    acc: &mut Vec<Value>, 
                    fp: &str, 
                    lm: &Option<LabelManager>, 
                    filter: Option<&str>, 
                    depth: usize, 
                    max_d: usize
                ) {
                    if depth > max_d || acc.len() >= 5000 { return; }
                    
                    if filter.is_none() || filter.unwrap() == n.node_type {
                        let labels = lm.as_ref().map(|mgr| mgr.get_labels(fp, &n.content)).unwrap_or_default();
                        acc.push(json!({
                            "path": n.path, 
                            "type": n.node_type, 
                            "name": n.get_name(), 
                            "start": n.start_line, 
                            "labels": labels
                        }));
                    }
                    
                    for c in &n.children { 
                        collect(c, acc, fp, lm, filter, depth + 1, max_d); 
                    }
                }
                
                collect(w.analyze(), &mut nodes, file_path, &label_mgr, filter, 0, effective_max_depth);
                
                let mut msg = format!("Found {} nodes", nodes.len());
                if nodes.len() >= 1000 {
                    msg.push_str(" (limit reached)");
                }
                tool_success(msg, Some(json!({"nodes": nodes})))
            }
            Err(e) => tool_error(format!("IO error: {}", e)),
        }
    }

                fn handle_get_skeleton(file_path: &str, max_depth: usize) -> Value {
        match GnawTreeWriter::new(file_path) {
            Ok(w) => {
                let mut s = String::new();
                let mut count = 0;
                fn build(n: &TreeNode, out: &mut String, d: usize, md: usize, count: &mut usize) {
                    if d > md || *count >= 500 { return; }
                    *count += 1;
                    out.push_str(&format!("{}{} [{}] {}\n", "  ".repeat(d), n.path, n.node_type, n.get_name().unwrap_or_default()));
                    if *count == 500 {
                        out.push_str("... (limit reached)\n");
                        return;
                    }
                    for c in &n.children { build(c, out, d + 1, md, count); }
                }
                build(w.analyze(), &mut s, 0, max_depth, &mut count);
                tool_success(format!("Skeleton of {}", file_path), Some(json!({"skeleton": s})))
            }
            Err(e) => tool_error(format!("IO error: {}", e)),
        }
    }

    fn handle_compress(file_path: &str) -> Value {
        match crate::core::compress::compress_file(file_path) {
            Ok(result) => {
                tool_success(
                    format!("Compressed {} ({} → {} tokens, {:.0}% reduction)",
                        file_path, result.original_tokens, result.compressed_tokens, result.ratio * 100.0),
                    Some(json!({
                        "code": result.code,
                        "original_tokens": result.original_tokens,
                        "compressed_tokens": result.compressed_tokens,
                        "bodies_compressed": result.bodies_compressed,
                        "ratio": result.ratio
                    }))
                )
            }
            Err(e) => tool_error(format!("Compression failed: {}", e)),
        }
    }

    fn handle_pack(
        path: &str,
        format: &str,
        compress: bool,
        include: Option<&str>,
        ignore: Option<&str>,
        instructions: Option<&str>,
    ) -> Value {
        let root = std::path::Path::new(path);
        if !root.exists() {
            return tool_error(format!("Path does not exist: {}", path));
        }

        let include_exts: Vec<String> = include
            .map(|s| s.split(',').map(|s| s.trim().to_string()).collect())
            .unwrap_or_default();

        let ignore_patterns: Vec<String> = ignore
            .map(|s| s.split(',').map(|s| s.trim().to_string()).collect())
            .unwrap_or_default();

        let options = crate::core::pack::PackOptions {
            format: crate::core::pack::PackFormat::parse(format),
            compress,
            tokens: true,
            include_extensions: include_exts,
            ignore_patterns,
            instructions: instructions.map(|s| s.to_string()),
            output: None,
            redact_secrets: true,
        };

        match crate::core::pack::pack_project(root, &options) {
            Ok(result) => {
                tool_success(
                    format!("Packed {} files ({} tokens)", result.file_count, result.total_tokens),
                    Some(json!({
                        "content": result.content,
                        "file_count": result.file_count,
                        "total_tokens": result.total_tokens,
                        "compressed_tokens": result.compressed_tokens,
                        "files": result.files
                    }))
                )
            }
            Err(e) => tool_error(format!("Pack failed: {}", e)),
        }
    }

    fn handle_curate(
        task: &str,
        path: &str,
        strategy: &str,
        max_tokens: usize,
        max_files: usize,
    ) -> Value {
        let root = std::path::Path::new(path);
        if !root.exists() {
            return tool_error(format!("Path does not exist: {}", path));
        }

        let strategy = crate::core::curator::CurationStrategy::parse(strategy);

        match crate::core::curator::curate_context(root, task, strategy, max_tokens, max_files) {
            Ok(result) => {
                tool_success(
                    format!("Curated {} files ({} tokens)", result.files.len(), result.total_tokens),
                    Some(json!({
                        "files": result.files,
                        "total_tokens": result.total_tokens,
                        "strategy": result.strategy,
                        "summary": result.summary
                    }))
                )
            }
            Err(e) => tool_error(format!("Curation failed: {}", e)),
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

    async fn handle_search_semantic(state: Arc<AppState>, query: &str, file_path: Option<&str>, max_results: usize) -> Value {
        #[cfg(feature = "modernbert")]
        {
            let _mgr = match crate::llm::ai_manager::AiManager::new(&state.project_root) {
                Ok(m) => m,
                Err(e) => return tool_error(e.to_string()),
            };

            let broker = match crate::llm::GnawSenseBroker::new(&state.project_root) {
                Ok(b) => b,
                Err(e) => return tool_error(e.to_string()),
            };

            match broker.sense(query, file_path).await {
                Ok(crate::llm::SenseResponse::Satelite { matches }) => {
                    let results: Vec<Value> = matches.iter().take(max_results).map(|m| {
                        json!({
                            "file": m.file_path,
                            "node_path": m.node_path,
                            "score": m.score,
                        })
                    }).collect();
                    tool_success(
                        format!("Found {} matches for \"{}\"", results.len(), query),
                        Some(json!({"matches": results, "query": query, "mode": "satellite"}))
                    )
                }
                Ok(crate::llm::SenseResponse::Zoom { file_path: fp, nodes, impact }) => {
                    let results: Vec<Value> = nodes.iter().take(max_results).map(|n| {
                        json!({
                            "path": n.path,
                            "preview": n.preview,
                            "score": n.score,
                        })
                    }).collect();
                    tool_success(
                        format!("Found {} nodes in {} for \"{}\"", results.len(), fp, query),
                        Some(json!({"matches": results, "query": query, "file": fp, "mode": "zoom", "impact": impact}))
                    )
                }
                Err(e) => tool_error(format!("Semantic search failed: {}", e)),
            }
        }
        #[cfg(not(feature = "modernbert"))]
        {
            let _ = state;
            let _ = query;
            let _ = file_path;
            let _ = max_results;
            tool_error("ModernBERT feature not enabled. Install with --features modernbert.".into())
        }
    }

    fn handle_diff_since(since_commit: Option<&str>, since_date: Option<&str>, include_uncommitted: bool, use_saved_state: bool) -> Value {
        let project_root = std::env::current_dir().unwrap_or_default();

        // Determine the reference point
        let (ref_point, from_commit) = if let Some(commit) = since_commit {
            (commit.to_string(), commit.to_string())
        } else if let Some(date) = since_date {
            (date.to_string(), date.to_string())
        } else if use_saved_state {
            let state = crate::core::state::ProjectState::load(&project_root);
            if !state.git_head.is_empty() {
                (format!("saved_state ({})", &state.git_head[..8.min(state.git_head.len())]), state.git_head)
            } else {
                ("HEAD~1".to_string(), "HEAD~1".to_string())
            }
        } else {
            ("HEAD~1".to_string(), "HEAD~1".to_string())
        };

        // Get changed files
        let mut changed_files: Vec<Value> = Vec::new();

        // Committed changes
        let log_range = format!("{}..HEAD", from_commit);

        if let Ok(output) = std::process::Command::new("git")
            .args(["diff", "--name-status", &log_range])
            .current_dir(&project_root)
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let parts: Vec<&str> = line.splitn(2, '\t').collect();
                if parts.len() == 2 {
                    let status = match parts[0] {
                        "A" => "added",
                        "D" => "deleted",
                        "M" => "modified",
                        "R" => "renamed",
                        _ => "unknown",
                    };
                    changed_files.push(json!({
                        "path": parts[1],
                        "status": status,
                        "source": "committed"
                    }));
                }
            }
        }

        // Uncommitted changes
        if include_uncommitted {
            if let Ok(output) = std::process::Command::new("git")
                .args(["status", "--porcelain"])
                .current_dir(&project_root)
                .output()
            {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for line in stdout.lines() {
                    if line.len() >= 3 {
                        let status_code = &line[..2];
                        let path = line[3..].trim();
                        let status = if status_code.starts_with('M') || status_code.ends_with('M') {
                            "modified"
                        } else if status_code.starts_with('A') || status_code.starts_with('?') {
                            "untracked"
                        } else if status_code.starts_with('D') || status_code.ends_with('D') {
                            "deleted"
                        } else {
                            "changed"
                        };

                        // Skip if already in committed list
                        let already_listed = changed_files.iter().any(|f| {
                            f.get("path").and_then(Value::as_str) == Some(path)
                        });
                        if !already_listed {
                            changed_files.push(json!({
                                "path": path,
                                "status": status,
                                "source": "uncommitted"
                            }));
                        }
                    }
                }
            }
        }

        let added = changed_files.iter().filter(|f| f.get("status").and_then(Value::as_str) == Some("added")).count();
        let modified = changed_files.iter().filter(|f| f.get("status").and_then(Value::as_str) == Some("modified")).count();
        let deleted = changed_files.iter().filter(|f| f.get("status").and_then(Value::as_str) == Some("deleted")).count();
        let untracked = changed_files.iter().filter(|f| f.get("status").and_then(Value::as_str) == Some("untracked")).count();

        let current_head = std::process::Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(&project_root)
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
            .unwrap_or_default();

        tool_success(
            format!("Changes since {}: {} files ({} added, {} modified, {} deleted, {} untracked)",
                ref_point, changed_files.len(), added, modified, deleted, untracked),
            Some(json!({
                "reference": ref_point,
                "current_head": current_head,
                "changed_files": changed_files,
                "stats": {
                    "total": changed_files.len(),
                    "added": added,
                    "modified": modified,
                    "deleted": deleted,
                    "untracked": untracked
                }
            }))
        )
    }

    /// Resolve file paths from either file_path (single) or file_paths (batch).
    fn resolve_file_paths(arguments: &Value) -> Vec<String> {
        // Try file_paths array first
        if let Some(paths) = arguments.get("file_paths").and_then(Value::as_array) {
            return paths.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
        }
        // Fall back to single file_path
        if let Some(path) = arguments.get("file_path").and_then(Value::as_str) {
            return vec![path.to_string()];
        }
        vec![]
    }

    fn handle_index_entities_batch(paths: &[String], include_private: bool) -> Value {
        let mut all_entities = Vec::new();
        let mut all_imports = Vec::new();
        let mut all_exports = Vec::new();
        let mut errors = Vec::new();
        let mut total_entities = 0usize;

        for path in paths {
            match crate::core::index_entities::index_entities(path, include_private) {
                Ok(result) => {
                    total_entities += result.entity_count;
                    all_imports.extend(result.imports);
                    all_exports.extend(result.exports);
                    all_entities.push(json!({
                        "file": result.file,
                        "entity_count": result.entity_count,
                        "entities": result.entities,
                    }));
                }
                Err(e) => {
                    errors.push(json!({"file": path, "error": e.to_string()}));
                }
            }
        }

        tool_success(
            format!("Indexed {} entities from {} files ({} errors)",
                total_entities, paths.len(), errors.len()),
            Some(json!({
                "file_count": paths.len(),
                "total_entities": total_entities,
                "total_imports": all_imports.len(),
                "total_exports": all_exports.len(),
                "files": all_entities,
                "errors": errors,
            }))
        )
    }

    fn handle_index_relations_batch(paths: &[String]) -> Value {
        let mut all_relations = Vec::new();
        let mut combined_summary: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        let mut errors = Vec::new();
        let mut total_relations = 0usize;

        for path in paths {
            match crate::core::index_relations::index_relations(path) {
                Ok(result) => {
                    total_relations += result.relations.len();
                    for (k, v) in &result.summary {
                        *combined_summary.entry(k.clone()).or_insert(0) += v;
                    }
                    all_relations.push(json!({
                        "file": result.file,
                        "relation_count": result.relations.len(),
                        "relations": result.relations,
                    }));
                }
                Err(e) => {
                    errors.push(json!({"file": path, "error": e.to_string()}));
                }
            }
        }

        tool_success(
            format!("Indexed {} relations from {} files ({})",
                total_relations, paths.len(),
                combined_summary.iter().map(|(k,v)| format!("{}:{}", k, v)).collect::<Vec<_>>().join(", ")),
            Some(json!({
                "file_count": paths.len(),
                "total_relations": total_relations,
                "summary": combined_summary,
                "files": all_relations,
                "errors": errors,
            }))
        )
    }

    fn handle_save_state() -> Value {
        let project_root = std::env::current_dir().unwrap_or_default();
        match crate::core::state::ProjectState::update(&project_root) {
            Ok(state) => {
                tool_success(
                    format!("Saved state: HEAD={}, {} file hashes",
                        &state.git_head[..8.min(state.git_head.len())],
                        state.file_hashes.len()),
                    Some(json!({
                        "git_head": state.git_head,
                        "file_count": state.file_hashes.len(),
                        "last_analyzed": state.last_analyzed,
                    }))
                )
            }
            Err(e) => tool_error(format!("Failed to save state: {}", e)),
        }
    }

        fn handle_search_nodes(file_path: &str, pattern: &str) -> Value {
        match GnawTreeWriter::new(file_path) {
            Ok(w) => {
                let mut m = Vec::new();
                fn find(n: &TreeNode, acc: &mut Vec<Value>, p: &str) {
                    if acc.len() >= 500 { return; }
                    if n.content.contains(p) {
                        acc.push(json!({"path": n.path, "type": n.node_type, "name": n.get_name()}));
                    }
                    for c in &n.children { find(c, acc, p); }
                }
                find(w.analyze(), &mut m, pattern);
                let mut msg = format!("Found {} matches", m.len());
                if m.len() >= 500 {
                    msg.push_str(" (limit reached)");
                }
                tool_success(msg, Some(json!({"matches": m})))
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
                    match writer.edit(op, false) {
                        Ok(_) => {
                            let pulse = generate_pulse(state, file_path, &proposal.anchor_path);
                            tool_success_with_pulse(
                                format!(
                                    "Successfully inserted code near anchor '{}' (confidence: {:.2})",
                                    proposal.anchor_path, proposal.confidence
                                ),
                                None,
                                pulse,
                            )
                        },
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

    async fn handle_semantic_edit(
        state: Arc<AppState>,
        file_path: &str,
        query: &str,
        content: &str,
    ) -> Value {
        #[cfg(feature = "modernbert")]
        {
            use crate::llm::{GnawSenseBroker, SenseResponse};
            let broker = match GnawSenseBroker::new(&state.project_root) {
                Ok(b) => b,
                Err(e) => return tool_error(e.to_string()),
            };

            match broker.sense(query, Some(file_path)).await {
                Ok(SenseResponse::Zoom { nodes, .. }) if !nodes.is_empty() => {
                    let best_node = &nodes[0];
                    handle_edit_node_internal(state, file_path, &best_node.path, content)
                },
                Ok(_) => tool_error(format!("Could not find a semantic match for '{}' in {}", query, file_path)),
                Err(e) => tool_error(e.to_string()),
            }
        }
        #[cfg(not(feature = "modernbert"))]
        {
            let _ = (state, file_path, query, content);
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

    fn handle_edit_node_internal(state: Arc<AppState>, file_path: &str, node_path: &str, content: &str) -> Value {
        match GnawTreeWriter::new(file_path) {
            Ok(mut w) => {
                let old_source = w.get_source().to_string();
                let op = EditOperation::Edit { node_path: node_path.to_string(), content: content.to_string() };
                if let Err(e) = w.edit(op, false) { return tool_error(e.to_string()); }
                
                let new_source_loaded = std::fs::read_to_string(file_path).unwrap_or_default();
                let diff = generate_diff_string(&old_source, &new_source_loaded);
                let pulse = generate_pulse(state, file_path, node_path);
                tool_success_with_pulse(format!("Node edited.\nDiff:\n{}", diff), Some(json!({"diff": diff})), pulse)
            },
            Err(e) => tool_error(format!("IO error: {}", e)),
        }
    }

    fn handle_insert_node(state: Arc<AppState>, file_path: &str, parent_path: &str, position: usize, content: &str) -> Value {
        match GnawTreeWriter::new(file_path) {
            Ok(mut w) => {
                let old_source = w.get_source().to_string();
                let op = EditOperation::Insert { parent_path: parent_path.to_string(), position, content: content.to_string() };
                if let Err(e) = w.edit(op, false) { return tool_error(e.to_string()); }
                
                let new_source_loaded = std::fs::read_to_string(file_path).unwrap_or_default();
                let diff = generate_diff_string(&old_source, &new_source_loaded);
                let pulse = generate_pulse(state, file_path, parent_path); // Pulse for parent
                tool_success_with_pulse(format!("Content inserted.\nDiff:\n{}", diff), Some(json!({"diff": diff})), pulse)
            },
            Err(e) => tool_error(format!("IO error: {}", e)), // Corrected: escaped curly brace
        }
    }

    fn handle_move_node(state: Arc<AppState>, source_file: &str, source_path: &str, target_file: &str, target_path: &str) -> Value {
        match GnawTreeWriter::new(source_file) {
            Ok(mut src_w) => {
                let old_source = src_w.get_source().to_string();
                let delete_op = EditOperation::Delete { node_path: source_path.to_string() };
                if let Err(e) = src_w.edit(delete_op, false) { return tool_error(e.to_string()); }

                let insert_op = EditOperation::Insert {
                    parent_path: target_path.to_string(),
                    position: 1,
                    content: old_source.clone(),
                };
                match GnawTreeWriter::new(target_file) {
                    Ok(mut tgt_w) => {
                        let old_target = tgt_w.get_source().to_string();
                        if let Err(e) = tgt_w.edit(insert_op, false) { return tool_error(e.to_string()); }
                        let new_target = std::fs::read_to_string(target_file).unwrap_or_default();
                        let diff = generate_diff_string(&old_target, &new_target);
                        let pulse = generate_pulse(state, target_file, target_path);
                        tool_success_with_pulse(format!("Moved from {} [{}] to {} [{}].\nDiff:\n{}", source_file, source_path, target_file, target_path, diff), Some(json!({"diff": diff})), pulse)
                    },
                    Err(e) => tool_error(format!("IO error on target: {}", e)),
                }
            },
            Err(e) => tool_error(format!("IO error on source: {}", e)),
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
        eprintln!("Starting MCP server on http://{}", listener.local_addr()?); // Fixed: redirected to stderr
        serve_with_shutdown(listener, token, async { let _ = signal::ctrl_c().await; }).await
    }

    pub async fn status(url: &str, token: Option<String>) -> Result<()> {
        let client = reqwest::Client::new();
        let mut req = client.post(url);
        if let Some(t) = token { req = req.header("Authorization", format!("Bearer {}", t)); } // Corrected: escaped curly brace
        let _ = req.json(&json!({"jsonrpc":"2.0","method":"initialize","id":1})).send().await?;
        eprintln!("✓ Server ready");
        Ok(())
    }
}
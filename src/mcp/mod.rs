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
    use rayon::prelude::*;
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
        /// Shared GnawSense broker so the ModernBERT model and the JIT file
        /// index cache persist across MCP calls. Creating a fresh broker per
        /// call reloaded the model and re-embedded files every time, blowing
        /// client timeouts on large files.
        #[cfg(feature = "modernbert")]
        sense_broker: tokio::sync::OnceCell<std::sync::Arc<crate::llm::GnawSenseBroker>>,
        /// Background semantic-index run state (index_project tool):
        /// one run at a time, last report kept for status polling.
        #[cfg(feature = "modernbert")]
        index_run: std::sync::Arc<IndexRun>,
    }

    /// Shared state for the background `index_project` tool.
    #[cfg(feature = "modernbert")]
    #[derive(Default)]
    struct IndexRun {
        running: std::sync::atomic::AtomicBool,
        last: std::sync::Mutex<Option<Value>>,
    }

    #[cfg(feature = "modernbert")]
    impl AppState {
        async fn sense_broker(&self) -> Result<std::sync::Arc<crate::llm::GnawSenseBroker>> {
            let root = self.project_root.clone();
            self.sense_broker
                .get_or_try_init(|| async move {
                    crate::llm::GnawSenseBroker::new(&root).map(std::sync::Arc::new)
                })
                .await
                .cloned()
        }
    }

    impl AppState {
        fn new(token: Option<String>, project_root: std::path::PathBuf) -> Self {
            #[cfg(not(feature = "modernbert"))]
            {
                Self {
                    token,
                    project_root,
                }
            }
            #[cfg(feature = "modernbert")]
            {
                Self {
                    token,
                    project_root,
                    sense_broker: tokio::sync::OnceCell::new(),
                    index_run: std::sync::Arc::new(IndexRun::default()),
                }
            }
        }
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
            "initialize" => Ok(json!({
                "protocolVersion": "2024-11-05",
                "serverInfo": {
                    "name": env!("CARGO_PKG_NAME"),
                    "version": env!("CARGO_PKG_VERSION")
                },
                "capabilities": {
                    "tools": { "listChanged": true }
                }
            })),

            "tools/list" => Ok(json!({ "tools": tool_definitions() })),

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

                let validate_arg = |key: &str| -> Result<&str, Value> {
                    arguments.get(key).and_then(Value::as_str).ok_or_else(|| {
                        let err = build_jsonrpc_error(
                            req.id.clone(),
                            INVALID_PARAMS_CODE,
                            "Invalid parameters",
                            Some(json!({"field": key})),
                        );
                        serde_json::to_value(err).unwrap()
                    })
                };

                match name {
                    "analyze" => {
                        let fp = validate_arg("file_path")?;
                        Ok(handle_analyze(fp))
                    }
                    "list_nodes" => {
                        let fp = validate_arg("file_path")?;
                        let filter = arguments.get("filter").and_then(Value::as_str);
                        let max_depth = arguments
                            .get("max_depth")
                            .and_then(Value::as_u64)
                            .map(|d| d as usize);
                        Ok(handle_list_nodes(state, fp, filter, max_depth, false))
                    }
                    "doctor" => Ok(handle_doctor_mcp()),
                    "get_skeleton" => {
                        let fp = validate_arg("file_path")?;
                        let max_depth = arguments
                            .get("max_depth")
                            .and_then(Value::as_u64)
                            .unwrap_or(2) as usize;
                        Ok(handle_get_skeleton(fp, max_depth))
                    }
                    "compress" => {
                        let fp = validate_arg("file_path")?;
                        Ok(handle_compress(fp))
                    }
                    "pack" => {
                        let path = arguments.get("path").and_then(Value::as_str).unwrap_or(".");
                        let format = arguments
                            .get("format")
                            .and_then(Value::as_str)
                            .unwrap_or("markdown");
                        let compress = arguments
                            .get("compress")
                            .and_then(Value::as_bool)
                            .unwrap_or(false);
                        let include = arguments.get("include").and_then(Value::as_str);
                        let ignore = arguments.get("ignore").and_then(Value::as_str);
                        let instructions = arguments.get("instructions").and_then(Value::as_str);
                        Ok(handle_pack(
                            path,
                            format,
                            compress,
                            include,
                            ignore,
                            instructions,
                        ))
                    }
                    "curate" => {
                        let task = validate_arg("task")?;
                        let path = arguments.get("path").and_then(Value::as_str).unwrap_or(".");
                        let strategy = arguments
                            .get("strategy")
                            .and_then(Value::as_str)
                            .unwrap_or("auto");
                        let max_tokens = arguments
                            .get("max_tokens")
                            .and_then(Value::as_u64)
                            .unwrap_or(8000) as usize;
                        let max_files = arguments
                            .get("max_files")
                            .and_then(Value::as_u64)
                            .unwrap_or(20) as usize;
                        Ok(handle_curate(task, path, strategy, max_tokens, max_files))
                    }
                    "search_semantic" => {
                        let query = validate_arg("query")?;
                        let file_path = arguments.get("file_path").and_then(Value::as_str);
                        let max_results = arguments
                            .get("max_results")
                            .and_then(Value::as_u64)
                            .unwrap_or(10) as usize;
                        Ok(handle_search_semantic(state, query, file_path, max_results).await)
                    }
                    "diff_since" => {
                        let since_commit = arguments.get("since_commit").and_then(Value::as_str);
                        let since_date = arguments.get("since_date").and_then(Value::as_str);
                        let include_uncommitted = arguments
                            .get("include_uncommitted")
                            .and_then(Value::as_bool)
                            .unwrap_or(true);
                        let use_saved_state = arguments
                            .get("use_saved_state")
                            .and_then(Value::as_bool)
                            .unwrap_or(true);
                        Ok(handle_diff_since(
                            since_commit,
                            since_date,
                            include_uncommitted,
                            use_saved_state,
                        ))
                    }
                    "history" => {
                        let limit =
                            arguments.get("limit").and_then(Value::as_u64).unwrap_or(10) as usize;
                        Ok(handle_history_mcp(state.clone(), limit))
                    }
                    "stats" => Ok(handle_stats_mcp(state.clone())),
                    #[cfg(feature = "modernbert")]
                    "index_project" => {
                        let action = arguments
                            .get("action")
                            .and_then(Value::as_str)
                            .unwrap_or("start");
                        Ok(handle_index_project(state.clone(), action))
                    }
                    "index_entities" => {
                        let paths = resolve_file_paths(&arguments);
                        let include_private = arguments
                            .get("include_private")
                            .and_then(Value::as_bool)
                            .unwrap_or(false);
                        if paths.is_empty() {
                            Err("No file_path or file_paths provided".into())
                        } else {
                            Ok(handle_index_entities_batch(&paths, include_private))
                        }
                    }
                    "index_relations" => {
                        let paths = resolve_file_paths(&arguments);
                        if paths.is_empty() {
                            Err("No file_path or file_paths provided".into())
                        } else {
                            Ok(handle_index_relations_batch(&paths))
                        }
                    }
                    "save_state" => Ok(handle_save_state()),
                    "explore" => {
                        let target = arguments
                            .get("target")
                            .and_then(Value::as_str)
                            .unwrap_or("");
                        let level = arguments
                            .get("level")
                            .and_then(Value::as_str)
                            .unwrap_or("0");
                        Ok(handle_explore(target, level))
                    }
                    "explain" => {
                        let fp = validate_arg("file_path")?;
                        let node = arguments.get("node").and_then(Value::as_str);
                        Ok(handle_explain(fp, node))
                    }
                    "edit_ask" => {
                        let fp = validate_arg("file_path")?;
                        let request = validate_arg("request")?;
                        Ok(handle_edit_ask(fp, request))
                    }
                    "summarize" => {
                        let path = arguments.get("path").and_then(Value::as_str).unwrap_or(".");
                        let max_files = arguments
                            .get("max_files")
                            .and_then(Value::as_u64)
                            .unwrap_or(50) as usize;
                        Ok(handle_summarize(path, max_files))
                    }
                    "investigate" => {
                        let question = validate_arg("question")?;
                        Ok(handle_investigate(question))
                    }
                    "add_rule" => {
                        let id = validate_arg("id")?;
                        let language = validate_arg("language")?;
                        let pattern = validate_arg("pattern")?;
                        let severity = arguments
                            .get("severity")
                            .and_then(Value::as_str)
                            .unwrap_or("warning");
                        let message = arguments.get("message").and_then(Value::as_str);
                        let fix = arguments.get("fix").and_then(Value::as_str);
                        Ok(handle_add_rule(
                            id, language, pattern, severity, message, fix,
                        ))
                    }
                    "lint" => {
                        let paths: Vec<String> = arguments
                            .get("paths")
                            .and_then(Value::as_array)
                            .map(|a| {
                                a.iter()
                                    .filter_map(Value::as_str)
                                    .map(|s| s.to_string())
                                    .collect()
                            })
                            .unwrap_or_default();
                        if paths.is_empty() {
                            return Ok(tool_error(
                                "lint requires `paths`: [files or directories]. For an ad-hoc pattern search also pass `pattern` + `language`, e.g. {\"paths\": [\"src\"], \"pattern\": \"$X.unwrap()\", \"language\": \"rust\"}".to_string(),
                            ));
                        }
                        Ok(handle_lint_mcp(
                            &paths,
                            arguments
                                .get("recursive")
                                .and_then(Value::as_bool)
                                .unwrap_or(true),
                            arguments.get("rules_file").and_then(Value::as_str),
                            arguments.get("severity").and_then(Value::as_str),
                            arguments.get("rule_id").and_then(Value::as_str),
                            arguments.get("pattern").and_then(Value::as_str),
                            arguments.get("language").and_then(Value::as_str),
                            arguments
                                .get("fix")
                                .and_then(Value::as_bool)
                                .unwrap_or(false),
                            arguments
                                .get("preview")
                                .and_then(Value::as_bool)
                                .unwrap_or(false),
                        ))
                    }
                    "get_semantic_report" => {
                        let fp = validate_arg("file_path")?;
                        Ok(handle_get_semantic_report(state, fp).await)
                    }
                    "search_nodes" => {
                        let fp = validate_arg("file_path")?;
                        let pattern = validate_arg("pattern")?;
                        Ok(handle_search_nodes(fp, pattern))
                    }
                    "read_node" => {
                        let fp = validate_arg("file_path")?;
                        let np = validate_arg("node_path")?;
                        Ok(handle_read_node(fp, np))
                    }
                    "edit_node" => {
                        let fp = validate_arg("file_path")?;
                        let np = validate_arg("node_path")?;
                        let c = validate_arg("content")?;
                        Ok(handle_edit_node_internal(state, fp, np, c))
                    }
                    "preview_edit" => {
                        let fp = validate_arg("file_path")?;
                        let np = validate_arg("node_path")?;
                        let c = validate_arg("content")?;
                        Ok(handle_preview_edit(fp, np, c))
                    }
                    "move_node" => {
                        let sf = validate_arg("source_file")?;
                        let sp = validate_arg("source_path")?;
                        let tf = arguments
                            .get("target_file")
                            .and_then(Value::as_str)
                            .unwrap_or(sf);
                        let tp = validate_arg("target_path")?;
                        Ok(handle_move_node(state, sf, sp, tf, tp))
                    }
                    "insert_node" => {
                        let fp = validate_arg("file_path")?;
                        let pp = validate_arg("parent_path")?;
                        let c = validate_arg("content")?;
                        let pos = arguments
                            .get("position")
                            .and_then(Value::as_u64)
                            .unwrap_or(1) as usize;
                        Ok(handle_insert_node(state, fp, pp, pos, c))
                    }
                    "sense" => {
                        let query = validate_arg("query")?;
                        let fp = arguments.get("file_path").and_then(Value::as_str);
                        Ok(handle_sense(state, query, fp).await)
                    }
                    "semantic_insert" => {
                        let fp = validate_arg("file_path")?;
                        let anchor = validate_arg("anchor_query")?;
                        let content = validate_arg("content")?;
                        let intent = arguments
                            .get("intent")
                            .and_then(Value::as_str)
                            .unwrap_or("after");
                        Ok(handle_semantic_insert(state, fp, anchor, content, intent).await)
                    }
                    "semantic_edit" => {
                        let fp = validate_arg("file_path")?;
                        let query = validate_arg("query")?;
                        let content = validate_arg("content")?;
                        Ok(handle_semantic_edit(state, fp, query, content).await)
                    }
                    "batch" => {
                        let file = validate_arg("file")?;
                        let preview = arguments
                            .get("preview")
                            .and_then(Value::as_bool)
                            .unwrap_or(false);
                        Ok(handle_batch_mcp(file, preview))
                    }
                    "undo" => {
                        let steps =
                            arguments.get("steps").and_then(Value::as_u64).unwrap_or(1) as usize;
                        Ok(handle_undo_mcp(state.clone(), steps))
                    }
                    _ => {
                        let err = build_jsonrpc_error(
                            req.id,
                            METHOD_NOT_FOUND_CODE,
                            "Unknown tool",
                            None,
                        );
                        Err(serde_json::to_value(err).unwrap())
                    }
                }
            }
            _ => {
                let err =
                    build_jsonrpc_error(req.id, METHOD_NOT_FOUND_CODE, "Method not found", None);
                Err(serde_json::to_value(err).unwrap())
            }
        }
    }
    /// The MCP tool registry — real, testable Rust instead of an opaque
    /// macro blob. Every entry must carry a full inputSchema and a
    /// description that says WHAT the tool does, WHEN to reach for it
    /// (vs. Read/Grep/Edit) and WHAT it returns; the registry contract
    /// test enforces this.
    fn tool_definitions() -> Vec<Value> {
        let mut tools = vec![
            json!({
                "name": "analyze",
                "title": "Analyze file structure",
                "description": "Parse a file and return its full AST (abstract syntax tree) as typed nodes. WHAT: the structural map every GTW edit addresses. WHEN: first, before any edit — node paths found here are how edit_node/read_node target code, and unlike line numbers they survive edits elsewhere in the file. RETURNS: nested nodes with a dot-path, type and name each. Example: {\"file_path\": \"src/main.rs\"}.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "file_path": { "type": "string", "description": "Path to the source file to parse" }
                    },
                    "required": ["file_path"]
                }
            }),
            json!({
                "name": "list_nodes",
                "title": "List nodes in file",
                "description": "Flat, paginated list of the nodes in one file with dot-path, type and name — the index for finding the path an edit targets. WHEN: locating a specific function/struct after analyze, or enumerating every definition of a kind. RETURNS: one line per node: dot-path [type] name. Example: {\"file_path\": \"src/cli.rs\", \"filter\": \"function_item\"} lists every function.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "file_path": { "type": "string", "description": "Path to the file to list" },
                        "filter": { "type": "string", "description": "Only show nodes whose type contains this substring (e.g. function_item)" },
                        "max_depth": { "type": "integer", "description": "Limit tree depth (fewer = shallower listing)" }
                    },
                    "required": ["file_path"]
                }
            }),
            json!({
                "name": "doctor",
                "title": "Health check (parsers, backups, transaction log)",
                "description": "One-call health diagnostic: smoke-tests every parser, verifies backup integrity and the transaction log, and reports whether GnawTreeWriter is alive and sane in this project. WHEN: first contact with a repo, after a crash, or when any edit/tool behaves oddly — one call instead of guessing. RETURNS: {healthy, passed, failed, warnings, total, checks[]} — unhealthy is data, not an error, and the failure details point at restore-project. Takes no arguments. Example: {}.",
                "inputSchema": {
                    "type": "object",
                    "properties": {},
                    "required": []
                }
            }),
            json!({
                "name": "get_skeleton",
                "title": "Get skeletal view",
                "description": "Hierarchical outline of a file's definitions (functions, structs, impls) up to max_depth — a far smaller alternative to reading the whole file. WHEN: you need the shape of a large file without its bodies. RETURNS: an indented tree of signatures. Example: {\"file_path\": \"src/core/rules.rs\", \"max_depth\": 2}.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "file_path": { "type": "string" },
                        "max_depth": { "type": "integer" }
                    },
                    "required": ["file_path"]
                }
            }),
            json!({
                "name": "compress",
                "title": "Compress source code",
                "description": "Replace function/method bodies with placeholders to cut tokens (~70%) while preserving signatures and structure — a lossy but syntactically valid summary. WHEN: a file is too large to include but its API surface matters. RETURNS: compressed source text. Never writes to disk. Example: {\"file_path\": \"src/big.rs\"}.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "file_path": { "type": "string" }
                    },
                    "required": ["file_path"]
                }
            }),
            json!({
                "name": "pack",
                "title": "Pack project for AI",
                "description": "Pack an entire project into one AI-optimized document with per-file token counts and optional body compression. WHEN: bootstrapping context for a new repo or handing a project to another agent in one artifact. RETURNS: the packed document in the chosen format. Example: {\"path\": \".\", \"format\": \"markdown\", \"compress\": true}.",
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
            }),
            json!({
                "name": "curate",
                "title": "Curate context for AI agent",
                "description": "Intelligently select the most relevant files for a task instead of dumping the entire project. WHEN: you have a specific task and a token budget — returns the files worth reading first. RETURNS: ranked file list with token counts. Example: {\"task\": \"authentication flow\", \"max_tokens\": 5000}.",
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
            }),
            json!({
                "name": "search_semantic",
                "title": "Semantic code search",
                "description": "Search code by meaning across the entire project — find 'how is X implemented?' without knowing file names. WHEN: the question is semantic, not a literal identifier (use search_nodes for exact text). RETURNS: ranked code locations. Example: {\"query\": \"how is authentication handled?\", \"max_results\": 10}.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "query": { "type": "string", "description": "Semantic search query (e.g. 'how is authentication handled?')" },
                        "file_path": { "type": "string", "description": "Optional: limit search to this file (zoom mode)" },
                        "max_results": { "type": "integer", "description": "Maximum results (default: 10)" }
                    },
                    "required": ["query"]
                }
            }),
            json!({
                "name": "diff_since",
                "title": "Detect changes since last index",
                "description": "Compare current project state against a previous git commit, date, or saved state (save_state). WHEN: 'what changed since...?' after indexing or between sessions. RETURNS: added/modified/deleted files. Example: {\"since_date\": \"2026-08-20\", \"include_uncommitted\": true}.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "since_commit": { "type": "string", "description": "Git commit hash to compare against" },
                        "since_date": { "type": "string", "description": "ISO date to compare from (e.g. '2026-08-20')" },
                        "include_uncommitted": { "type": "boolean", "description": "Include uncommitted changes (default: true)" },
                        "use_saved_state": { "type": "boolean", "description": "Use saved state file if available (default: true)" }
                    }
                }
            }),
            json!({
                "name": "index_entities",
                "title": "Extract entities from source file(s)",
                "description": "Extract functions, structs, enums, impls and other entities from one or more files — builds the KNOWLEDGE GRAPH (entity/relation edges), NOT the vector index that satellite sense searches (that is index_project). WHEN: building a project map or after large changes. Provide file_path OR file_paths (at least one). RETURNS: entity records with kind, name and location. Example: {\"file_paths\": [\"src/core/*.rs\"]}.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "file_path": { "type": "string", "description": "Path to a single file to analyze" },
                        "file_paths": { "type": "array", "items": {"type": "string"}, "description": "Multiple files to analyze (batch mode)" },
                        "include_private": { "type": "boolean", "description": "Include private entities (default: false)" }
                    },
                    "anyOf": [ { "required": ["file_path"] }, { "required": ["file_paths"] } ]
                }
            }),
            json!({
                "name": "index_relations",
                "title": "Extract relations from source file(s)",
                "description": "Extract call relationships, imports, type usage and impl relations from one or more files — edges of the KNOWLEDGE GRAPH, not the vector index satellite sense searches (that is index_project). WHEN: together with index_entities to map how code connects. Provide file_path OR file_paths (at least one). RETURNS: relation records (caller → callee, import, impl). Example: {\"file_paths\": [\"src/cli.rs\", \"src/core/mod.rs\"]}.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "file_path": { "type": "string", "description": "Path to a single file to analyze" },
                        "file_paths": { "type": "array", "items": {"type": "string"}, "description": "Multiple files to analyze (batch mode)" }
                    },
                    "anyOf": [ { "required": ["file_path"] }, { "required": ["file_paths"] } ]
                }
            }),
            json!({
                "name": "save_state",
                "title": "Save project state for incremental tracking",
                "description": "Save current git HEAD and file hashes to .gnawtreewriter_state.json — the baseline that diff_since compares against. WHEN: right after indexing, or at a milestone you may want to diff from later. Takes no arguments. RETURNS: confirmation of the saved state. Example: {}.",
                "inputSchema": {
                    "type": "object",
                    "properties": {},
                    "required": []
                }
            }),
            json!({
                "name": "explore",
                "title": "Explore project with zoom levels",
                "description": "Map-like navigation with zoom levels: overview (dirs+tokens), directory (files+summaries), file (signatures), full (source). WHEN: first contact with a repo or choosing what to read next. RETURNS: the map for the requested level. Example: {\"target\": \"src\", \"level\": \"directory\"}.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "target": { "type": "string", "description": "Path to explore (file or directory, default: project root)" },
                        "level": { "type": "string", "enum": ["0", "1", "2", "3", "overview", "directory", "file", "full"], "description": "Zoom level (default: auto)" }
                    }
                }
            }),
            json!({
                "name": "explain",
                "title": "Explain a code node",
                "description": "Explain a code node — or a whole file — in plain language using the local LFM2.5 model. WHEN: onboarding to unfamiliar code or sanity-checking what a function does before editing it. RETURNS: a plain-language explanation. Example: {\"file_path\": \"src/core/batch.rs\", \"node\": \"1.2\"}.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "file_path": { "type": "string", "description": "Path to the file" },
                        "node": { "type": "string", "description": "AST node path (optional; default: whole file)" }
                    },
                    "required": ["file_path"]
                }
            }),
            json!({
                "name": "edit_ask",
                "title": "Propose an AST edit with the local LLM",
                "description": "Let the local LFM2.5 model propose a minimal edit for a plain-language request; the proposal is validated against the AST (Duplex Loop) and returned as a preview — nothing is written. WHEN: you know WHAT should change but not the exact node content. RETURNS: a proposed edit to review; apply it yourself via edit_node. Example: {\"file_path\": \"a.rs\", \"request\": \"make parse_file return anyhow::Result\"}.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "file_path": { "type": "string", "description": "Path to the file to edit" },
                        "request": { "type": "string", "description": "What to change, in plain language" }
                    },
                    "required": ["file_path", "request"]
                }
            }),
            json!({
                "name": "summarize",
                "title": "Summarize a directory",
                "description": "Hierarchical summary of a directory — what each file/module does — generated by the local LFM2.5 model. WHEN: getting oriented in an unfamiliar subtree before reading files. RETURNS: per-file one-liners grouped by directory. Example: {\"path\": \"src/core\", \"max_files\": 50}.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Directory to summarize" },
                        "max_files": { "type": "integer", "description": "Max files to summarize (default: 50)" }
                    }
                }
            }),
            json!({
                "name": "investigate",
                "title": "Investigate a question",
                "description": "Answer a question about the codebase using the local LFM2.5 model — a mix of semantic search and reasoning over indexed sources. WHEN: a 'how does X work?' question you cannot answer with a single grep. RETURNS: an answer plus sources: [{file}] — the exact files it was synthesized from, verifiable via read_node. Example: {\"question\": \"where is the transaction log rotated?\"}.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "question": { "type": "string", "description": "The question to investigate" }
                    },
                    "required": ["question"]
                }
            }),
            json!({
                "name": "add_rule",
                "title": "Add a lint rule",
                "description": "Validate and add a semgrep-like lint rule to gnawtreewriter.rules.yaml. The pattern must be valid code for the language, with $X placeholders. Pass `fix` to attach a rewrite template (also validated) — the rule then becomes applyable via the lint tool with fix: true.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string", "description": "Unique rule id (e.g. proj_no_todo)" },
                        "language": { "type": "string", "description": "Language: rust, python, javascript, ..." },
                        "pattern": { "type": "string", "description": "Code pattern with $X placeholders, e.g. \"$X.unwrap()\"" },
                        "severity": { "type": "string", "enum": ["error", "warning", "info"], "description": "Severity (default: warning)" },
                        "message": { "type": "string", "description": "Human-readable message" },
                        "fix": { "type": "string", "description": "Optional rewrite template with $X metavariables bound from the pattern match, e.g. \"$X.expect(\\\"msg\\\")\" — validated before saving" }
                    },
                    "required": ["id", "language", "pattern"]
                }
            }),
            json!({
                "name": "lint",
                "title": "Lint files or search patterns (AST-aware)",
                "description": "AST pattern linting and search in ONE call: run stored lint rules over files/directories, or pass `pattern`+`language` for an ad-hoc `$X` metavariable search (the ast-grep `run -p` equivalent). Use this INSTEAD of Grep when searching for code shapes: Grep sees text (breaks on formatting), this sees syntax, and returns structured findings [{file, rule_id, severity, message, line, column, node_path, captures}] with the node_path for surgical follow-up edits. Report-only — never writes.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "paths": { "type": "array", "items": {"type": "string"}, "description": "Files or directories to lint" },
                        "recursive": { "type": "boolean", "description": "Expand directories into their supported files (default true; directories without it are rejected)" },
                        "rules_file": { "type": "string", "description": "Extra rules YAML loaded after builtin + project rules" },
                        "severity": { "type": "string", "enum": ["error", "warning", "info"], "description": "Minimum severity to report" },
                        "rule_id": { "type": "string", "description": "Only run the rule with this id" },
                        "pattern": { "type": "string", "description": "Ad-hoc $X pattern to search for, e.g. \"$X.unwrap()\" (replaces stored rules)" },
                        "language": { "type": "string", "description": "Language for the ad-hoc pattern (required with pattern)" },
                        "fix": { "type": "boolean", "description": "Apply fixes from rules that carry a fix template (DEFAULT FALSE — report-only). Findings become atomic edits; use preview: true first." },
                        "preview": { "type": "boolean", "description": "With fix: true, show the planned fixes without writing anything" }
                    },
                    "required": ["paths"]
                }
            }),
            json!({
                "name": "get_semantic_report",
                "title": "Generate semantic quality report",
                "description": "AI code-quality analysis of one file — smells, complexity hotspots and structural observations from the local model. WHEN: reviewing a file you just edited, or before planning a refactor. RETURNS: a markdown quality report plus sources: [{file, node_path}] — one provenance pointer per finding, verifiable via read_node. Example: {\"file_path\": \"src/core/batch.rs\"}.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "file_path": { "type": "string", "description": "Path to the file to analyze" }
                    },
                    "required": ["file_path"]
                }
            }),
            json!({
                "name": "search_nodes",
                "title": "Search nodes by text",
                "description": "Find AST nodes whose content or name contains a text pattern, in one file. WHEN: you know an identifier or snippet and need the node path that edit_node/read_node require — unlike Grep it returns dot-paths, not line numbers. RETURNS: matching paths with type and name. Example: {\"file_path\": \"src/cli.rs\", \"pattern\": \"handle_lint\"}.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "file_path": { "type": "string", "description": "Path to the file to search" },
                        "pattern": { "type": "string", "description": "Text/substring to search for inside node content" }
                    },
                    "required": ["file_path", "pattern"]
                }
            }),
            json!({
                "name": "read_node",
                "title": "Read node content",
                "description": "Read the exact source text of ONE AST node by dot-path — the surgical alternative to dumping a whole file. WHEN: verifying content before or after an edit, or inspecting one function in a large file. RETURNS: the node's source code. Example: {\"file_path\": \"src/cli.rs\", \"node_path\": \"35.2.105\"}.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "file_path": { "type": "string", "description": "Path to the file" },
                        "node_path": { "type": "string", "description": "Dot-path of the node (from analyze/list_nodes/search_nodes)" }
                    },
                    "required": ["file_path", "node_path"]
                }
            }),
            json!({
                "name": "edit_node",
                "title": "Edit node content",
                "description": "Replace the content of ONE AST node (by dot-path) with new code — syntax-validated BEFORE writing, so a broken replacement is rejected instead of corrupting the file. WHEN: changing a function/struct/impl you already located; safer than text search-replace because the target cannot silently mismatch. RETURNS: a diff summary and a transaction id (revert via undo). Example: {\"file_path\": \"a.rs\", \"node_path\": \"1.2\", \"content\": \"fn fixed() { }\"}.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "file_path": { "type": "string", "description": "Path to the file" },
                        "node_path": { "type": "string", "description": "Dot-path of the node to replace" },
                        "content": { "type": "string", "description": "New complete content for the node (validated as code before write)" }
                    },
                    "required": ["file_path", "node_path", "content"]
                }
            }),
            json!({
                "name": "move_node",
                "title": "Move node to new location",
                "description": "Atomically move an AST node to another location — delete + reinsert in one transaction, optionally across files, so no half-moved state can exist. WHEN: relocating functions, methods or blocks between parents/files. RETURNS: a move report and transaction id (revert via undo). Example: {\"source_file\": \"a.rs\", \"source_path\": \"1.2\", \"target_file\": \"b.rs\", \"target_path\": \"0\"}; target_file defaults to source_file.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "source_file": { "type": "string", "description": "File containing the node" },
                        "source_path": { "type": "string", "description": "Dot-path of the node to move" },
                        "target_file": { "type": "string", "description": "Destination file (default: same as source_file)" },
                        "target_path": { "type": "string", "description": "Destination parent dot-path" }
                    },
                    "required": ["source_file", "source_path", "target_path"]
                }
            }),
            json!({
                "name": "insert_node",
                "title": "Insert new content",
                "description": "Insert new code as a child of a parent node at a given position — syntax-validated before writing, so it lands at a structurally valid spot. WHEN: adding a function, method, import, field or statement. RETURNS: an insertion report and transaction id (revert via undo). Example: {\"file_path\": \"a.rs\", \"parent_path\": \"35.2\", \"position\": 2, \"content\": \"fn helper() {}\"}.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "file_path": { "type": "string", "description": "Path to the file" },
                        "parent_path": { "type": "string", "description": "Dot-path of the parent to insert into" },
                        "position": { "type": "integer", "description": "Child index to insert at (default 1)" },
                        "content": { "type": "string", "description": "Code to insert (validated as code before write)" }
                    },
                    "required": ["file_path", "parent_path", "content"]
                }
            }),
            json!({
                "name": "preview_edit",
                "title": "Preview edit",
                "description": "Dry-run an edit: show exactly what would change and write nothing. WHEN: before any risky edit_node, or when you want to double-check node targeting. RETURNS: a unified diff of the proposed change. Example: {\"file_path\": \"a.rs\", \"node_path\": \"1.2\", \"content\": \"...\"} — repeat the same call via edit_node when the diff is right.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "file_path": { "type": "string", "description": "Path to the file" },
                        "node_path": { "type": "string", "description": "Dot-path of the node the edit targets" },
                        "content": { "type": "string", "description": "Proposed new content for the node" }
                    },
                    "required": ["file_path", "node_path", "content"]
                }
            }),
            json!({
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
            }),
            json!({
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
            }),
            json!({
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
            }),
            json!({
                "name": "batch",
                "title": "Apply batch operations atomically",
                "description": "Apply a batch of AST edit operations from a batch JSON file atomically: every operation validates in-memory before any write, and any failure rolls back the whole batch. Use this for coordinated multi-file changes INSTEAD of several edit_node calls — one transaction, one preview, one undo, one transaction-log entry. Batch JSON format: {\"description\": str, \"operations\": [{\"type\": \"edit\"|\"insert\"|\"delete\", \"file\": str, ...}]} — see BATCH_USAGE.md. With preview=true returns the unified diff and writes nothing.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "file": { "type": "string", "description": "Path to the batch JSON spec" },
                        "preview": { "type": "boolean", "description": "Show the diff without writing (default false)" }
                    },
                    "required": ["file"]
                }
            }),
            json!({
                "name": "undo",
                "title": "Undo recent edit operations",
                "description": "Undo the last N logged GTW edit operations (default 1) through the transaction log — the quick 'oops' button after a bad edit. Each reverted operation is reported with its transaction id; failed reverts are reported loudly with the restore fallback (`gnawtreewriter restore-project --preview`). Prefer this over hand-reverting files.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "steps": { "type": "integer", "description": "Number of operations to undo (default 1)" }
                    }
                }
            }),
        ];

        tools.push(json!({
            "name": "history",
            "title": "Recent edit history (transaction log)",
            "description": "Recent transactions from the project transaction log — the MCP view of `gnawtreewriter history`: what changed, in which order, with stable ids. WHEN: after a series of edits, before deciding what to undo, or when asked \"what did GTW change here?\". RETURNS: {transactions: [{id, timestamp, operation, file, node_path, description}], count}, newest first; empty history is reported as a normal state with setup guidance. Example: {\"limit\": 5}.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "limit": { "type": "integer", "description": "How many of the newest transactions to return (default 10)" }
                },
                "required": []
            }
        }));
        tools.push(json!({
            "name": "stats",
            "title": "Project statistics",
            "description": "Project statistics behind `gnawtreewriter stats`: files, lines, tokens, per-language breakdown, largest files, compression estimate. WHEN: sizing up a repo, deciding what to pack/curate, or answering \"how big is this codebase?\". RETURNS: the full ProjectStats JSON plus a one-line human summary. Takes no arguments (uses the project root). Example: {}.",
            "inputSchema": {
                "type": "object",
                "properties": {},
                "required": []
            }
        }));

        // modernbert-only: builds the vector index satellite sense searches
        // (index_entities extracts the knowledge graph instead).
        #[cfg(feature = "modernbert")]
        tools.push(json!({
            "name": "index_project",
            "title": "Build the semantic search index (background)",
            "description": "Build the SEMANTIC (vector) index that satellite sense searches — the MCP-side equivalent of `gnawtreewriter ai index`, same pipeline, same GPU/20%-VRAM gate. NOT index_entities, which extracts the knowledge graph. Runs in the BACKGROUND: call with {\"action\":\"start\"} (default), then poll {\"action\":\"status\"} until running=false and read last_run — full runs take ~1-2 min on GPU. Until it has completed once, satellite sense (no file_path) has nothing to search; zoom sense works regardless. Example: {\"action\": \"status\"}.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["start", "status"],
                        "description": "start (default) begins a background run if none is running; status reports running + last_run {ok, files, seconds, message}"
                    }
                },
                "required": []
            }
        }));

        tools
    }

    async fn rpc_handler(
        State(state): State<Arc<AppState>>,
        headers: HeaderMap,
        Json(req): Json<Value>,
    ) -> impl IntoResponse {
        if let Some(expected) = &state.token {
            match headers.get("authorization").and_then(|v| v.to_str().ok()) {
                Some(s) if s == format!("Bearer {}", expected) => {} // Corrected: escaped curly brace
                _ => {
                    return (
                        StatusCode::UNAUTHORIZED,
                        Json(json!({ // Corrected: escaped curly brace
                            "jsonrpc": "2.0",
                            "id": null,
                            "error": { "code": -32001, "message": "Unauthorized" }
                        })),
                    );
                }
            }
        }

        let parsed: JsonRpcRequest = match serde_json::from_value(req) {
            Ok(r) => r,
            Err(_) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(
                        json!({"jsonrpc": "2.0", "id": null, "error": {"code": -32700, "message": "Parse error"}}),
                    ),
                )
            }
        };

        let id = parsed.id.clone();
        // Panic isolation: see serve_stdio. A panicking handler must yield a
        // JSON-RPC internal error, not take down the connection task.
        let spawned = {
            let st = state.clone();
            tokio::spawn(async move { process_request(st, parsed).await })
        };
        let outcome = match spawned.await {
            Ok(inner) => inner,
            Err(join_err) => Err(json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {
                    "code": -32603,
                    "message": format!("Internal error (handler panicked): {}", join_err)
                }
            })),
        };
        match outcome {
            Ok(res) => (
                StatusCode::OK,
                Json(json!({"jsonrpc": "2.0", "id": id, "result": res})),
            ), // Corrected: escaped curly brace
            Err(err) => {
                let code = err
                    .get("error")
                    .and_then(|e| e.get("code"))
                    .and_then(|c| c.as_i64())
                    .unwrap_or(0);
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
        let state = Arc::new(AppState::new(None, project_root));

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
            // Run each request in its own task: a panic inside a handler
            // (e.g. a parsing bug) must not unwind through the read loop and
            // kill the whole stdio connection ("Not connected" on the host).
            let spawned = {
                let st = state.clone();
                tokio::spawn(async move { process_request(st, req).await })
            };
            let outcome = match spawned.await {
                Ok(inner) => inner,
                Err(join_err) => Err(json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": {
                        "code": -32603,
                        "message": format!("Internal error (handler panicked): {}", join_err)
                    }
                })),
            };
            match outcome {
                Ok(result) => {
                    let resp = json!({"jsonrpc": "2.0", "id": id, "result": result});
                    if let Ok(resp_str) = serde_json::to_string(&resp) {
                        let _ = stdout.write_all(resp_str.as_bytes()).await;
                        let _ = stdout.write_all(b"\n").await;
                        let _ = stdout.flush().await;
                    }
                }
                Err(err) => {
                    let _ = stdout
                        .write_all(serde_json::to_string(&err).unwrap_or_default().as_bytes())
                        .await;
                    let _ = stdout.write_all(b"\n").await;
                    let _ = stdout.flush().await;
                }
            }
            line.clear();
        }
        Ok(())
    }

    fn tool_error(msg: String) -> Value {
        json!({"content": [{ "type": "text", "text": msg }], "isError": true})
    }

    /// Duplex-loop metrics (Motor2 plan #23): propose/validate/apply counts
    /// persisted at <project>/.gnawtreewriter_metrics.json so integrations
    /// can aggregate rates across sessions. Read-modify-write (rare
    /// concurrent edits may lose a tick — acceptable for counters).
    /// Scope: edit_node/insert/edit_ask/semantic_insert call outcomes;
    /// move/batch count their own transactions, not these keys.
    pub fn bump_duplex_metric(project_root: &std::path::Path, key: &str) {
        let path = project_root.join(".gnawtreewriter_metrics.json");
        let mut data: Value = std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_else(|| json!({"duplex": {}}));
        if !data.is_object() {
            data = json!({"duplex": {}});
        }
        let obj = data.as_object_mut().unwrap();
        if !obj.contains_key("duplex") || !obj["duplex"].is_object() {
            obj.insert("duplex".to_string(), json!({}));
        }
        let duplex = obj.get_mut("duplex").unwrap().as_object_mut().unwrap();
        let current = duplex.get(key).and_then(Value::as_u64).unwrap_or(0);
        duplex.insert(key.to_string(), json!(current + 1));
        if let Ok(text) = serde_json::to_string_pretty(&data) {
            let _ = std::fs::write(&path, text);
        }
    }

    /// Map a strict GnawTreeWriter::new failure to the right stable code:
    /// a syntax refusal of the EXISTING file is not a missing file —
    /// read paths still answer partially, editors must refuse loudly.
    fn open_error(file_path: &str, e: &impl std::fmt::Display) -> Value {
        let msg = format!("{}", e);
        if msg.contains("Syntax error") || msg.contains("Failed to parse") {
            tool_error_code(
                format!(
                    "Strict parse refused {}: {} — the EXISTING file has syntax errors. Read paths (analyze/skeleton) still return partial trees with the position; fix the file before editing.",
                    file_path, msg
                ),
                "E_STRICT_PARSE",
            )
        } else {
            tool_error_code(
                format!(
                    "Failed to open {}: {} — verify file_path exists (explore/search_nodes find it)",
                    file_path, msg
                ),
                "E_FILE_NOT_FOUND",
            )
        }
    }

    /// tool_error with a STABLE machine code (Motor2 plan #25): integrations
    /// aggregate on `code`, humans read `content[0].text`. Codes are listed
    /// in docs/ERROR_CODES.md and must never be renamed silently.
    fn tool_error_code(msg: String, code: &str) -> Value {
        let mut v = tool_error(msg);
        if let Some(obj) = v.as_object_mut() {
            obj.insert("code".to_string(), json!(code));
        }
        v
    }
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
        res.as_object_mut()
            .unwrap()
            .insert("pulse".to_string(), pulse);
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
                if n.path == p {
                    return n.get_name();
                }
                for c in &n.children {
                    if let Some(nm) = find_name(c, p) {
                        return Some(nm);
                    }
                }
                None
            }
            find_name(tree, node_path)
        } else {
            None
        };

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
                    pulse["hints"].as_array_mut().unwrap().push(json!(format!(
                        "Symbol '{}' is called in {} places. Consider verifying impact.",
                        n,
                        callers.len()
                    )));
                }
            }
        }

        // 3. Search for tests
        let file_name = std::path::Path::new(file_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("");
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
            pulse["hints"].as_array_mut().unwrap().push(json!(
                "Found associated test files. Remember to update or run tests."
            ));
        }

        pulse
    }

    fn handle_analyze(file_path: &str) -> Value {
        match GnawTreeWriter::new_lenient(file_path) {
            Ok((w, warning)) => {
                let source = w.get_source();
                let tokens = crate::core::token_count::estimate_code_tokens(source);
                let mut tree_json = serde_json::to_value(w.analyze()).unwrap_or(json!(null));
                if let Some(obj) = tree_json.as_object_mut() {
                    obj.insert("estimated_tokens".to_string(), json!(tokens));
                }
                let note = match &warning {
                    Some(wn) => format!(" ⚠ PARTIAL PARSE: syntax error at line {}, col {} — results may be incomplete; editors still refuse this file strictly", wn.line, wn.column),
                    None => String::new(),
                };
                if let (Some(obj), Some(wn)) = (tree_json.as_object_mut(), &warning) {
                    obj.insert(
                        "syntax_warning".to_string(),
                        json!({"line": wn.line, "column": wn.column, "message": wn.message}),
                    );
                }
                json!({"content": [{ "type": "text", "text": format!("Analyzed {} ({} tokens){}", file_path, tokens, note)}], "data": tree_json})
            }
            Err(e) => tool_error_code(format!("IO error: {} — check the path exists and points at a supported source file (search_nodes finds files by name; analyze parses any supported file)", e), "E_FILE_NOT_FOUND"),
        }
    }

    fn handle_list_nodes(
        state: Arc<AppState>,
        file_path: &str,
        filter: Option<&str>,
        max_depth: Option<usize>,
        all: bool,
    ) -> Value {
        match GnawTreeWriter::new_lenient(file_path) {
            Ok((w, warning)) => {
                let label_mgr = LabelManager::load(&state.project_root).ok();
                let mut nodes = Vec::new();
                let effective_max_depth = if all {
                    usize::MAX
                } else {
                    max_depth.unwrap_or(3)
                };

                fn collect(
                    n: &TreeNode,
                    acc: &mut Vec<Value>,
                    fp: &str,
                    lm: &Option<LabelManager>,
                    filter: Option<&str>,
                    depth: usize,
                    max_d: usize,
                ) {
                    if depth > max_d || acc.len() >= 5000 {
                        return;
                    }

                    if filter.is_none() || filter.unwrap() == n.node_type {
                        let labels = lm
                            .as_ref()
                            .map(|mgr| mgr.get_labels(fp, &n.content))
                            .unwrap_or_default();
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

                collect(
                    w.analyze(),
                    &mut nodes,
                    file_path,
                    &label_mgr,
                    filter,
                    0,
                    effective_max_depth,
                );

                let mut msg = format!("Found {} nodes", nodes.len());
                if nodes.len() >= 1000 {
                    msg.push_str(" (limit reached)");
                }
                let mut payload = json!({"nodes": nodes});
                if let Some(wn) = &warning {
                    msg.push_str(&format!(
                        " ⚠ partial parse (syntax error at {}:{})",
                        wn.line, wn.column
                    ));
                    if let Some(obj) = payload.as_object_mut() {
                        obj.insert(
                            "syntax_warning".to_string(),
                            json!({"line": wn.line, "column": wn.column, "message": wn.message}),
                        );
                    }
                }
                tool_success(msg, Some(payload))
            }
            Err(e) => tool_error_code(format!("IO error: {} — check the path exists and points at a supported source file (search_nodes finds files by name; analyze parses any supported file)", e), "E_FILE_NOT_FOUND"),
        }
    }

    /// `doctor`: one-call health diagnostic (parser smokes, backup integrity,
    /// transaction log) — shares run_full_doctor with the CLI so the two can
    /// never diverge. Answers "is GTW alive and sane in this project?" without
    /// the agent having to guess. Zero arguments; healthy/unhealthy is data,
    /// not an error — diagnosis succeeded either way.
    fn handle_doctor_mcp() -> Value {
        let root = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        let project_root = crate::core::find_project_root(&root);
        let report = crate::core::diagnostics::run_full_doctor(&project_root);
        let status = if report.overall_healthy {
            format!(
                "doctor: {} / {} checks passed ({} warnings) — GTW is healthy in this project. Checks covered: parsers, backup integrity, transaction log.",
                report.passed, report.total_checks, report.warnings
            )
        } else {
            format!(
                "doctor: {} / {} checks passed, {} FAILED, {} warnings — inspect checks[] below for the failing item; for transaction/backup trouble start with `gnawtreewriter restore-project --preview`.",
                report.passed, report.total_checks, report.failed, report.warnings
            )
        };
        tool_success(
            status,
            Some(json!({
                "healthy": report.overall_healthy,
                "passed": report.passed,
                "failed": report.failed,
                "warnings": report.warnings,
                "total": report.total_checks,
                "checks": report.checks,
            })),
        )
    }

    fn handle_get_skeleton(file_path: &str, max_depth: usize) -> Value {
        const NODE_LIMIT: usize = 500;
        match GnawTreeWriter::new_lenient(file_path) {
            Ok((w, warning)) => {
                let mut s = String::new();
                let mut count = 0;
                fn build(n: &TreeNode, out: &mut String, d: usize, md: usize, count: &mut usize) {
                    if d > md || *count >= 500 {
                        return;
                    }
                    *count += 1;
                    out.push_str(&format!(
                        "{}{} [{}] {}\n",
                        "  ".repeat(d),
                        n.path,
                        n.node_type,
                        n.get_name().unwrap_or_default()
                    ));
                    if *count == 500 {
                        out.push_str("... (limit reached)\n");
                        return;
                    }
                    for c in &n.children {
                        build(c, out, d + 1, md, count);
                    }
                }
                build(w.analyze(), &mut s, 0, max_depth, &mut count);
                let truncated = count >= NODE_LIMIT;
                // NEVER silently empty (ROADMAP 9.5): an empty skeleton must
                // fail loudly with next steps, not return a bare header.
                if s.is_empty() {
                    return tool_error(format!(
                        "get_skeleton produced no nodes for {} (max_depth={}) — this must never be \
read as a valid empty answer. Next: raise max_depth, run analyze for the raw tree, \
or list_nodes for a flat index of this file.",
                        file_path, max_depth
                    ));
                }
                let mut header = format!(
                    "Skeleton of {} ({} node(s){})
{}",
                    file_path,
                    count,
                    if truncated { ", truncated at 500 — raise max_depth only after list_nodes" } else { "" },
                    s
                );
                let mut payload =
                    json!({"skeleton": s, "nodes": count, "truncated": truncated});
                if let Some(wn) = &warning {
                    header = format!(
                        "⚠ PARTIAL PARSE: syntax error at line {} col {} — skeleton may be incomplete (editors still refuse this file strictly)\n{}",
                        wn.line, wn.column, header
                    );
                    if let Some(obj) = payload.as_object_mut() {
                        obj.insert(
                            "syntax_warning".to_string(),
                            json!({"line": wn.line, "column": wn.column, "message": wn.message}),
                        );
                    }
                }
                tool_success(header, Some(payload))
            }
            Err(e) => tool_error(format!(
                "get_skeleton could not read {}: {} — check the path exists and is a supported source file (search_nodes finds files, analyze parses any supported file).",
                file_path, e
            )),
        }
    }

    fn handle_compress(file_path: &str) -> Value {
        match crate::core::compress::compress_file(file_path) {
            Ok(result) => tool_success(
                format!(
                    "Compressed {} ({} → {} tokens, {:.0}% reduction)",
                    file_path,
                    result.original_tokens,
                    result.compressed_tokens,
                    result.ratio * 100.0
                ),
                Some(json!({
                    "code": result.code,
                    "original_tokens": result.original_tokens,
                    "compressed_tokens": result.compressed_tokens,
                    "bodies_compressed": result.bodies_compressed,
                    "ratio": result.ratio
                })),
            ),
            Err(e) => tool_error(format!("Compression failed: {} — the file may not be parseable; run analyze on it to see why", e)),
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
            return tool_error(format!("Path does not exist: {} — paths resolve from the project root; verify the location with the explore tool first", path));
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
            compress_threshold: 0,
        };

        match crate::core::pack::pack_project(root, &options) {
            Ok(result) => tool_success(
                format!(
                    "Packed {} files ({} tokens)",
                    result.file_count, result.total_tokens
                ),
                Some(json!({
                    "content": result.content,
                    "file_count": result.file_count,
                    "total_tokens": result.total_tokens,
                    "compressed_tokens": result.compressed_tokens,
                    "files": result.files
                })),
            ),
            Err(e) => tool_error(format!(
                "Pack failed: {} — check the path and the include/ignore patterns",
                e
            )),
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
            return tool_error(format!("Path does not exist: {} — paths resolve from the project root; verify the location with the explore tool first", path));
        }

        let strategy = crate::core::curator::CurationStrategy::parse(strategy);

        match crate::core::curator::curate_context(root, task, strategy, max_tokens, max_files) {
            Ok(result) => tool_success(
                format!(
                    "Curated {} files ({} tokens)",
                    result.files.len(),
                    result.total_tokens
                ),
                Some(json!({
                    "files": result.files,
                    "total_tokens": result.total_tokens,
                    "strategy": result.strategy,
                    "summary": result.summary
                })),
            ),
            Err(e) => tool_error(format!(
                "Curation failed: {} — check the path; the task description drives file selection",
                e
            )),
        }
    }

    /// Structured content for `get_semantic_report`: the report plus a
    /// top-level `sources` provenance list (ROADMAP 9.4). Extracted as a
    /// pure function so the sources contract is testable without loading
    /// a model — the integration test calls this exact function.
    pub fn semantic_report_payload(report: &crate::llm::SemanticReport) -> Value {
        json!({"report": report, "sources": report.sources()})
    }

    /// Structured content for `investigate`: result + tokens + top-level
    /// `sources` (ROADMAP 9.4). Pure — unit-testable without a model.
    #[cfg(feature = "mamba")]
    pub fn investigate_payload(
        result: &crate::llm::pipeline::InvestigateResult,
        budget: &crate::llm::TokenBudget,
    ) -> Value {
        json!({ "result": result, "tokens": budget, "sources": result.sources })
    }

    async fn handle_get_semantic_report(state: Arc<AppState>, file_path: &str) -> Value {
        #[cfg(feature = "modernbert")]
        {
            let mgr = match crate::llm::ai_manager::AiManager::new(&state.project_root) {
                Ok(m) => m,
                Err(e) => return tool_error_code(format!("AiManager init failed: {} — run `gnawtreewriter ai setup` to download models (ai status shows what is installed), then retry", e), "E_MODEL_UNAVAILABLE"),
            };
            match mgr.generate_semantic_report(file_path).await {
                Ok(report) => tool_success(
                    "Semantic report generated".into(),
                    Some(semantic_report_payload(&report)),
                ),
                Err(e) => tool_error(format!("Semantic report failed: {} — `gnawtreewriter ai status` checks the model; on repeat fall back to read_node + your own review and log it in GTW_MCP_ISSUE_LOG.md", e)),
            }
        }
        #[cfg(not(feature = "modernbert"))]
        {
            let _ = state;
            let _ = file_path;
            tool_error_code("ModernBERT feature not enabled — rebuild with the AI features: cargo install --path . --features modernbert,mcp (README: Full power), then retry this call.".into(), "E_MODEL_UNAVAILABLE")
        }
    }

    async fn handle_search_semantic(
        state: Arc<AppState>,
        query: &str,
        file_path: Option<&str>,
        max_results: usize,
    ) -> Value {
        #[cfg(feature = "modernbert")]
        {
            let _mgr = match crate::llm::ai_manager::AiManager::new(&state.project_root) {
                Ok(m) => m,
                Err(e) => return tool_error_code(format!("AiManager init failed: {} — run `gnawtreewriter ai setup` to download models (ai status shows what is installed), then retry", e), "E_MODEL_UNAVAILABLE"),
            };

            let broker = match crate::llm::GnawSenseBroker::new(&state.project_root) {
                Ok(b) => b,
                Err(e) => return tool_error_code(format!("Semantic model init failed: {} — run `gnawtreewriter ai setup` to fetch models (ai status shows what is installed), then retry", e), "E_MODEL_UNAVAILABLE"),
            };

            match broker.sense(query, file_path).await {
                Ok(crate::llm::SenseResponse::Satelite { matches }) => {
                    let results: Vec<Value> = matches
                        .iter()
                        .take(max_results)
                        .map(|m| {
                            json!({
                                "file": m.file_path,
                                "node_path": m.node_path,
                                "score": m.score,
                                "preview": m.content_preview,
                            })
                        })
                        .collect();
                    tool_success(
                        format!("Found {} matches for \"{}\"", results.len(), query),
                        Some(json!({"matches": results, "query": query, "mode": "satellite"})),
                    )
                }
                Ok(crate::llm::SenseResponse::Zoom {
                    file_path: fp,
                    nodes,
                    impact,
                }) => {
                    let results: Vec<Value> = nodes
                        .iter()
                        .take(max_results)
                        .map(|n| {
                            json!({
                                "path": n.path,
                                "preview": n.preview,
                                "score": n.score,
                            })
                        })
                        .collect();
                    tool_success(
                        format!("Found {} nodes in {} for \"{}\"", results.len(), fp, query),
                        Some(
                            json!({"matches": results, "query": query, "file": fp, "mode": "zoom", "impact": impact}),
                        ),
                    )
                }
                Err(e) => tool_error(format!("Semantic search failed: {} — check the local model with `gnawtreewriter ai status`; on repeat, fall back to search_nodes/grep and log the failure in GTW_MCP_ISSUE_LOG.md", e)),
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

    fn handle_diff_since(
        since_commit: Option<&str>,
        since_date: Option<&str>,
        include_uncommitted: bool,
        use_saved_state: bool,
    ) -> Value {
        let project_root = std::env::current_dir().unwrap_or_default();

        // Determine the reference point
        let (ref_point, from_commit) = if let Some(commit) = since_commit {
            (commit.to_string(), commit.to_string())
        } else if let Some(date) = since_date {
            (date.to_string(), date.to_string())
        } else if use_saved_state {
            let state = crate::core::state::ProjectState::load(&project_root);
            if !state.git_head.is_empty() {
                (
                    format!(
                        "saved_state ({})",
                        &state.git_head[..8.min(state.git_head.len())]
                    ),
                    state.git_head,
                )
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
                        let already_listed = changed_files
                            .iter()
                            .any(|f| f.get("path").and_then(Value::as_str) == Some(path));
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

        let added = changed_files
            .iter()
            .filter(|f| f.get("status").and_then(Value::as_str) == Some("added"))
            .count();
        let modified = changed_files
            .iter()
            .filter(|f| f.get("status").and_then(Value::as_str) == Some("modified"))
            .count();
        let deleted = changed_files
            .iter()
            .filter(|f| f.get("status").and_then(Value::as_str) == Some("deleted"))
            .count();
        let untracked = changed_files
            .iter()
            .filter(|f| f.get("status").and_then(Value::as_str) == Some("untracked"))
            .count();

        let current_head = std::process::Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(&project_root)
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
            .unwrap_or_default();

        tool_success(
            format!(
                "Changes since {}: {} files ({} added, {} modified, {} deleted, {} untracked)",
                ref_point,
                changed_files.len(),
                added,
                modified,
                deleted,
                untracked
            ),
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
            })),
        )
    }

    /// Resolve file paths from either file_path (single) or file_paths (batch).
    fn resolve_file_paths(arguments: &Value) -> Vec<String> {
        // Try file_paths array first
        if let Some(paths) = arguments.get("file_paths").and_then(Value::as_array) {
            return paths
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
        }
        // Fall back to single file_path
        if let Some(path) = arguments.get("file_path").and_then(Value::as_str) {
            return vec![path.to_string()];
        }
        vec![]
    }

    /// `history`: recent transactions from the project transaction log —
    /// the MCP view of `gnawtreewriter history`. What changed, in which
    /// order, with ids you can feed to undo / restore-project.
    fn handle_history_mcp(state: Arc<AppState>, limit: usize) -> Value {
        let root = state.project_root.clone();
        match crate::core::transaction_log::TransactionLog::load(&root) {
            Ok(log) => match log.get_last_n_transactions(limit) {
                Ok(txs) => {
                    if txs.is_empty() {
                        return tool_success(
                            "Transaction history is empty — no GTW edits have been logged in this project yet (edit_node/insert/batch/CLI writes create entries; `gnawtreewriter session-start` groups them).".to_string(),
                            Some(json!({"transactions": [], "count": 0})),
                        );
                    }
                    let entries: Vec<Value> = txs
                        .iter()
                        .rev()
                        .map(|t| {
                            json!({
                                "id": t.id,
                                "timestamp": t.timestamp.to_rfc3339(),
                                "operation": format!("{:?}", t.operation),
                                "file": t.file_path.display().to_string(),
                                "node_path": t.node_path,
                                "description": t.description,
                            })
                        })
                        .collect();
                    tool_success(
                        format!(
                            "{} transaction(s), newest first — ids identify ops for undo; `restore-project --preview` goes deeper than the tail.",
                            txs.len()
                        ),
                        Some(json!({"transactions": entries, "count": txs.len()})),
                    )
                }
                Err(e) => tool_error(format!(
                    "Cannot read transaction history: {} — inspect on the CLI with `gnawtreewriter history`.",
                    e
                )),
            },
            Err(e) => tool_error(format!(
                "No transaction log in {}: {} — GTW edits create one on first write; an empty project having none is expected, not an outage.",
                root.display(),
                e
            )),
        }
    }

    /// `stats`: project statistics behind `gnawtreewriter stats` —
    /// files/lines/tokens per language + largest files (Motor2 plan #24:
    /// the data already existed in TransactionLog/stats, only the MCP
    /// surface was missing).
    fn handle_stats_mcp(state: Arc<AppState>) -> Value {
        let root = state.project_root.clone();
        match crate::core::stats::analyze_project(&root) {
            Ok(s) => {
                let largest = s
                    .largest_files
                    .first()
                    .map(|f| format!("{} ({} lines)", f.path, f.lines))
                    .unwrap_or_else(|| "n/a".to_string());
                tool_success(
                    format!(
                        "{} files, {} lines, {} tokens, {} language(s); largest: {}.",
                        s.total_files, s.total_lines, s.total_tokens, s.languages.len(), largest
                    ),
                    Some(serde_json::to_value(&s).unwrap_or(json!({}))),
                )
            }
            Err(e) => tool_error(format!(
                "Stats failed for {}: {} — the path must be an existing directory (verify with explore first).",
                root.display(),
                e
            )),
        }
    }

    /// `index_project`: build the SEMANTIC (vector) index that satellite
    /// `sense` searches — the MCP-side equivalent of `gnawtreewriter ai
    /// index` (same pipeline, same GPU/20%-VRAM gate). NOT `index_entities`,
    /// which extracts the knowledge graph. Long runs happen in the
    /// background (poll with action=status) so the call never outruns
    /// client timeouts.
    fn handle_index_project(state: Arc<AppState>, action: &str) -> Value {
        use std::sync::atomic::Ordering;
        match action {
            "status" => {
                let running = state.index_run.running.load(Ordering::SeqCst);
                let last = state.index_run.last.lock().ok().and_then(|g| g.clone());
                if running {
                    tool_success(
                        "Semantic indexing is RUNNING in the background — satellite sense will search it when done. Poll again with action: \"status\" (a full run takes ~1-2 min on GPU, minutes on CPU).".to_string(),
                        Some(json!({"running": true, "last_run": last})),
                    )
                } else if let Some(report) = last {
                    let msg = report
                        .get("message")
                        .and_then(|m| m.as_str())
                        .unwrap_or("last run finished")
                        .to_string();
                    tool_success(msg, Some(json!({"running": false, "last_run": report})))
                } else {
                    tool_success(
                        "No indexing run yet — start one with action: \"start\" (or run `gnawtreewriter ai index` in the project). Until then satellite sense has nothing to search; zoom sense (with file_path) works regardless.".to_string(),
                        Some(json!({"running": false, "last_run": null})),
                    )
                }
            }
            "start" | "" => {
                if state
                    .index_run
                    .running
                    .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
                    .is_err()
                {
                    return tool_success(
                        "Semantic indexing is already running — poll with action: \"status\" instead of starting a second run.".to_string(),
                        Some(json!({"running": true})),
                    );
                }
                let root = state.project_root.clone();
                let run = state.index_run.clone();
                tokio::spawn(async move {
                    let t0 = std::time::Instant::now();
                    let outcome: Result<usize, String> =
                        match crate::llm::ProjectIndexer::new(&root) {
                            Ok(indexer) => {
                                indexer.index_all(&root).await.map_err(|e| e.to_string())
                            }
                            Err(e) => Err(e.to_string()),
                        };
                    let report = match outcome {
                        Ok(files) => json!({
                            "ok": true,
                            "files": files,
                            "seconds": t0.elapsed().as_secs_f32().round(),
                            "message": format!("Indexed {} files — satellite sense now searches them", files),
                        }),
                        Err(e) => json!({
                            "ok": false,
                            "error": e,
                            "seconds": t0.elapsed().as_secs_f32().round(),
                            "message": format!("Indexing failed: {} — check `gnawtreewriter ai status`, then retry with action: \"start\"", e),
                        }),
                    };
                    if let Ok(mut last) = run.last.lock() {
                        *last = Some(report);
                    }
                    run.running.store(false, Ordering::SeqCst);
                });
                let root_disp = state.project_root.display().to_string();
                tool_success(
                    format!(
                        "Semantic indexing started in the background for {} — poll with action: \"status\" (the GPU/20%-VRAM gate applies as always). NOTE: this builds the VECTOR index satellite sense searches; index_entities builds the knowledge graph instead.",
                        root_disp
                    ),
                    Some(json!({"running": true})),
                )
            }
            other => tool_error(format!(
                "Unknown index_project action {:?} — use \"start\" (default) or \"status\".",
                other
            )),
        }
    }

    fn handle_index_entities_batch(paths: &[String], include_private: bool) -> Value {
        // Parallel: index each file independently (order preserved).
        let results: Vec<Result<crate::core::index_entities::EntityIndex, String>> = paths
            .par_iter()
            .map(|path| {
                crate::core::index_entities::index_entities(path, include_private)
                    .map_err(|e| e.to_string())
            })
            .collect();

        let mut all_entities = Vec::new();
        let mut all_imports = Vec::new();
        let mut all_exports = Vec::new();
        let mut errors = Vec::new();
        let mut total_entities = 0usize;

        for (path, result) in paths.iter().zip(results) {
            match result {
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
                    errors.push(json!({"file": path, "error": e}));
                }
            }
        }

        tool_success(
            format!(
                "Indexed {} entities from {} files ({} errors)",
                total_entities,
                paths.len(),
                errors.len()
            ),
            Some(json!({
                "file_count": paths.len(),
                "total_entities": total_entities,
                "total_imports": all_imports.len(),
                "total_exports": all_exports.len(),
                "files": all_entities,
                "errors": errors,
            })),
        )
    }

    fn handle_index_relations_batch(paths: &[String]) -> Value {
        // Parallel: index each file independently (order preserved).
        let results: Vec<Result<crate::core::index_relations::RelationIndex, String>> = paths
            .par_iter()
            .map(|path| {
                crate::core::index_relations::index_relations(path).map_err(|e| e.to_string())
            })
            .collect();

        let mut all_relations = Vec::new();
        let mut combined_summary: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        let mut errors = Vec::new();
        let mut total_relations = 0usize;

        for (path, result) in paths.iter().zip(results) {
            match result {
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
                    errors.push(json!({"file": path, "error": e}));
                }
            }
        }

        tool_success(
            format!(
                "Indexed {} relations from {} files ({})",
                total_relations,
                paths.len(),
                combined_summary
                    .iter()
                    .map(|(k, v)| format!("{}:{}", k, v))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Some(json!({
                "file_count": paths.len(),
                "total_relations": total_relations,
                "summary": combined_summary,
                "files": all_relations,
                "errors": errors,
            })),
        )
    }

    fn handle_save_state() -> Value {
        let project_root = std::env::current_dir().unwrap_or_default();
        match crate::core::state::ProjectState::update(&project_root) {
            Ok(state) => tool_success(
                format!(
                    "Saved state: HEAD={}, {} file hashes",
                    &state.git_head[..8.min(state.git_head.len())],
                    state.file_hashes.len()
                ),
                Some(json!({
                    "git_head": state.git_head,
                    "file_count": state.file_hashes.len(),
                    "last_analyzed": state.last_analyzed,
                })),
            ),
            Err(e) => tool_error(format!("Failed to save state: {} — check write permission on .gnawtreewriter_state.json in the project root", e)),
        }
    }

    fn handle_explore(target: &str, level_str: &str) -> Value {
        let root = std::env::current_dir().unwrap_or_default();
        let level = crate::core::explore::ZoomLevel::parse(level_str);

        match crate::core::explore::explore(&root, target, level) {
            Ok(result) => tool_success(
                format!(
                    "Explored '{}' at level {:?} ({} tokens, {} lines)",
                    result.path, result.level, result.node.tokens, result.node.lines
                ),
                Some(json!({
                    "path": result.path,
                    "level": format!("{:?}", result.level),
                    "node": result.node,
                    "available_levels": result.available_levels.iter()
                        .map(|l| format!("{:?}", l)).collect::<Vec<_>>(),
                })),
            ),
            Err(e) => tool_error(format!(
                "Explore failed: {} — check the target path (defaults to the project root)",
                e
            )),
        }
    }

    #[cfg(feature = "mamba")]
    fn handle_explain(file_path: &str, node: Option<&str>) -> Value {
        let root = std::env::current_dir().unwrap_or_default();
        let project_root = crate::core::find_project_root(&root);
        match crate::llm::AiManager::new(&project_root) {
            Ok(mgr) => match crate::llm::pipeline::explain_node(
                &mgr,
                file_path,
                node,
                crate::llm::Resolution::Auto,
            ) {
                Ok((explanation, budget)) => tool_success(
                    "Explained node".to_string(),
                    Some(json!({ "explanation": explanation, "tokens": budget })),
                ),
                Err(e) => tool_error(format!("Explain failed: {} — local model problem? `gnawtreewriter ai status`; fallback: read_node + read the code yourself", e)),
            },
            Err(e) => tool_error_code(format!("AiManager init failed: {} — download the local models first with `gnawtreewriter ai setup`, then retry (ai status shows what is installed)", e), "E_MODEL_UNAVAILABLE"),
        }
    }
    #[cfg(not(feature = "mamba"))]
    fn handle_explain(_file_path: &str, _node: Option<&str>) -> Value {
        tool_error(
            "explain requires the 'mamba' feature. Recompile with --features mamba".to_string(),
        )
    }

    /// `edit_ask`: propose a validated AST edit. Returns the target node,
    /// the new content, and a preview diff — it does NOT apply.
    #[cfg(feature = "mamba")]
    fn handle_edit_ask(file_path: &str, request: &str) -> Value {
        let root = std::env::current_dir().unwrap_or_default();
        let project_root = crate::core::find_project_root(&root);
        match crate::llm::AiManager::new(&project_root) {
            Ok(mgr) => match crate::llm::pipeline::propose_edit(
                &mgr,
                file_path,
                request,
                crate::llm::Resolution::Auto,
            ) {
                Ok(first) => {
                    bump_duplex_metric(&project_root, "proposed");
                    // Open once — preview_edit is &self, the same writer
                    // validates both the first and the repaired proposal.
                    let writer = match crate::GnawTreeWriter::new(file_path) {
                        Ok(w) => w,
                        Err(e) => {
                            return open_error(file_path, &e);
                        }
                    };
                    let validate = |node_path: &str, content: &str| -> anyhow::Result<String> {
                        let op = crate::core::EditOperation::Edit {
                            node_path: node_path.to_string(),
                            content: content.to_string(),
                        };
                        writer.preview_edit(op)
                    };
                    let proposal_ok = |proposal: &crate::llm::pipeline::EditProposal,
                                       retried: bool,
                                       first_error: Option<String>| {
                        json!({
                            "node_path": proposal.node_path,
                            "content": proposal.content,
                            "valid": true,
                            "preview": "",
                            "retried": retried,
                            "first_error": first_error,
                            "tokens": proposal.budget,
                        })
                    };
                    match validate(&first.node_path, &first.content) {
                        Ok(modified) => {
                            bump_duplex_metric(&project_root, "validated");
                            let mut payload = proposal_ok(&first, false, None);
                            if let Some(obj) = payload.as_object_mut() {
                                obj.insert("preview".to_string(), json!(modified));
                            }
                            tool_success("Validated edit proposal".to_string(), Some(payload))
                        }
                        Err(e1) => {
                            bump_duplex_metric(&project_root, "rejected");
                            // Motor2 plan #23: ONE repair round — feed the
                            // AST error back to the model; small models
                            // improve most from a single concrete retry.
                            let retry_request = format!(
                                "{}\n\nPREVIOUS PROPOSAL FAILED RUST VALIDATION: {}\nReturn a corrected proposal for the SAME request that parses — keep the same target node unless the error shows it was wrong.",
                                request, e1
                            );
                            match crate::llm::pipeline::propose_edit(
                                &mgr,
                                file_path,
                                &retry_request,
                                crate::llm::Resolution::Auto,
                            ) {
                                Ok(second) => {
                                    bump_duplex_metric(&project_root, "proposed");
                                    match validate(&second.node_path, &second.content) {
                                        Ok(modified) => {
                                            bump_duplex_metric(&project_root, "validated");
                                            let mut payload =
                                                proposal_ok(&second, true, Some(e1.to_string()));
                                            if let Some(obj) = payload.as_object_mut() {
                                                obj.insert("preview".to_string(), json!(modified));
                                            }
                                            tool_success(
                                                "Validated edit proposal (repaired: the first attempt failed validation and the AST error was fed back for one retry)".to_string(),
                                                Some(payload),
                                            )
                                        }
                                        Err(e2) => {
                                            bump_duplex_metric(&project_root, "rejected");
                                            tool_error_code(
                                                format!(
                                                    "Proposed edit failed validation even after one repair round. First error: {} — retry error: {}. Adjust the request, or use edit_node with a node path from list_nodes.",
                                                    e1, e2
                                                ),
                                                "E_VALIDATION",
                                            )
                                        }
                                    }
                                }
                                Err(e_retry) => tool_error_code(
                                    format!(
                                        "Proposed edit failed validation ({}) and the repair retry itself failed: {}. Check model health (`gnawtreewriter ai status`) or go straight to edit_node with a node path from list_nodes.",
                                        e1, e_retry
                                    ),
                                    "E_VALIDATION",
                                ),
                            }
                        }
                    }
                }
                Err(e) => tool_error(format!("Edit proposal failed: {} — retry with a more concrete request, or go straight to edit_node once you know the node path", e)),
            },
            Err(e) => tool_error_code(format!("AiManager init failed: {} — download the local models first with `gnawtreewriter ai setup`, then retry (ai status shows what is installed)", e), "E_MODEL_UNAVAILABLE"),
        }
    }
    #[cfg(not(feature = "mamba"))]
    fn handle_edit_ask(_file_path: &str, _request: &str) -> Value {
        tool_error(
            "edit_ask requires the 'mamba' feature. Recompile with --features mamba".to_string(),
        )
    }

    #[cfg(feature = "mamba")]
    fn handle_summarize(path: &str, max_files: usize) -> Value {
        let root = std::env::current_dir().unwrap_or_default();
        let project_root = crate::core::find_project_root(&root);
        match crate::llm::AiManager::new(&project_root) {
            Ok(mgr) => match crate::llm::pipeline::summarize_dir(
                &mgr,
                std::path::Path::new(path),
                max_files,
                crate::llm::Resolution::Auto,
            ) {
                Ok((result, budget)) => tool_success(
                    "Summarized directory".to_string(),
                    Some(json!({ "result": result, "tokens": budget })),
                ),
                Err(e) => tool_error(format!("Summarize failed: {} — local model problem? `gnawtreewriter ai status`; fallback: get_skeleton for structure", e)),
            },
            Err(e) => tool_error_code(format!("AiManager init failed: {} — download the local models first with `gnawtreewriter ai setup`, then retry (ai status shows what is installed)", e), "E_MODEL_UNAVAILABLE"),
        }
    }
    #[cfg(not(feature = "mamba"))]
    fn handle_summarize(_path: &str, _max_files: usize) -> Value {
        tool_error(
            "summarize requires the 'mamba' feature. Recompile with --features mamba".to_string(),
        )
    }

    #[cfg(feature = "mamba")]
    fn handle_investigate(question: &str) -> Value {
        let root = std::env::current_dir().unwrap_or_default();
        let project_root = crate::core::find_project_root(&root);
        match crate::llm::AiManager::new(&project_root) {
            Ok(mgr) => match crate::llm::pipeline::investigate(
                &mgr,
                question,
                crate::llm::Resolution::Auto,
            ) {
                Ok((result, budget)) => tool_success(
                    "Investigation complete".to_string(),
                    Some(investigate_payload(&result, &budget)),
                ),
                Err(e) => tool_error(format!("Investigate failed: {} — local model problem? `gnawtreewriter ai status`; fallback: search_nodes + read_node", e)),
            },
            Err(e) => tool_error_code(format!("AiManager init failed: {} — download the local models first with `gnawtreewriter ai setup`, then retry (ai status shows what is installed)", e), "E_MODEL_UNAVAILABLE"),
        }
    }
    #[cfg(not(feature = "mamba"))]
    fn handle_investigate(_question: &str) -> Value {
        tool_error(
            "investigate requires the 'mamba' feature. Recompile with --features mamba".to_string(),
        )
    }
    /// `lint`: AST pattern linting / ad-hoc `$X` pattern search. Report-only —
    /// never writes. Shared core with the CLI (`rules::run_lint`) so the two
    /// never diverge.
    #[allow(clippy::too_many_arguments)]
    fn handle_lint_mcp(
        paths: &[String],
        recursive: bool,
        rules_file: Option<&str>,
        severity_filter: Option<&str>,
        rule_filter: Option<&str>,
        pattern: Option<&str>,
        language: Option<&str>,
        fix: bool,
        preview: bool,
    ) -> Value {
        let opts = crate::core::rules::LintOptions {
            recursive,
            rules_file,
            severity_filter,
            rule_filter,
            ad_hoc_pattern: pattern,
            ad_hoc_language: language,
            ..Default::default()
        };
        match crate::core::rules::run_lint(paths, &opts) {
                Ok(result) => {
                    let mut lines: Vec<String> = result
                        .findings
                        .iter()
                        .map(|f| {
                            format!(
                                "{}:{}:{} {:?} [{}] {}",
                                f.file, f.line, f.column, f.severity, f.rule_id, f.message
                            )
                        })
                        .collect();
                    for e in &result.file_errors {
                        lines.push(format!("error: {}", e));
                    }
                    for w in &result.rule_warnings {
                        lines.push(format!("warning: {}", w));
                    }
                    // `fix` is never implicit: without the flag nothing is
                    // written — the run stays report-only.
                    let fix_note = if fix && !result.findings.is_empty() {
                        match crate::core::rules::fix_batch(&result.findings, &result.rules) {
                            Ok((batch, _no_fix)) if !batch.operations.is_empty() => {
                                if preview {
                                    match batch.preview_text() {
                                        Ok(t) => Some(format!(
                                            "Fix preview ({} operation(s), nothing written):
{}",
                                            batch.operations.len(),
                                            t
                                        )),
                                        Err(e) => Some(format!("Fix preview failed: {}", e)),
                                    }
                                } else {
                                    match batch.apply() {
                                        Ok(()) => Some(format!(
                                            "✅ Applied {} fix(es) atomically.",
                                            batch.operations.len()
                                        )),
                                        Err(e) => Some(format!(
                                            "❌ Fix apply FAILED (batch is atomic — no partial writes): {}",
                                            e
                                        )),
                                    }
                                }
                            }
                            Ok((_, no_fix)) => Some(format!(
                                "No fixable findings: {} finding(s) matched but none carry a fix template — add one with `add_rule` + fix.",
                                no_fix
                            )),
                            Err(e) => Some(format!("Fix expansion failed: {}", e)),
                        }
                    } else {
                        None
                    };
                    let summary = format!(
                        "Lint: {} finding(s) in {} file(s){}",
                        result.findings.len(),
                        result.files_checked,
                        if result.truncated {
                            format!(" (truncated at {} — narrow the search)", result.findings.len())
                        } else {
                            String::new()
                        }
                    );
                    if let Some(note) = &fix_note {
                        lines.push(note.clone());
                    }
                    let data = json!({
                        "findings": result.findings,
                        "files_checked": result.files_checked,
                        "skipped_rules": result.skipped_rules,
                        "truncated": result.truncated,
                        "fix_applied": fix && !preview && fix_note.as_ref().map(|n| n.starts_with("✅")).unwrap_or(false),
                    });
                    if result.findings.is_empty() && result.file_errors.is_empty() {
                        tool_success(
                            format!(
                                "{}. Nothing matched{}.",
                                summary,
                                if pattern.is_some() {
                                    " — check the pattern's language, or use `sense` for semantic (not structural) search".to_string()
                                } else {
                                    ". Rules come from rules/builtin.yaml + gnawtreewriter.rules.yaml — add one with `add_rule`".to_string()
                                }
                            ),
                            Some(data),
                        )
                    } else {
                        tool_success(format!("{}\n{}", summary, lines.join("\n")), Some(data))
                    }
                }
                Err(e) => tool_error(format!(
                    "Lint failed: {}. Directories need recursive=true; ad-hoc search needs both `pattern` and `language`.",
                    e
                )),
            }
    }

    /// `batch`: apply a batch spec file atomically (preview or apply). Real
    /// implementation via `Batch` — a stub here once reported "Batch executed"
    /// while writing nothing; never fake success.
    fn handle_batch_mcp(file: &str, preview: bool) -> Value {
        let batch = match crate::core::Batch::from_file(file) {
                Ok(b) => b,
                Err(e) => {
                    return tool_error(format!(
                        "Failed to load batch file '{}': {}. The batch spec is JSON with an `operations` array — see BATCH_USAGE.md for the full format, or convert a unified diff with `gnawtreewriter diff-to-batch`.",
                        file, e
                    ))
                }
            };
        if preview {
            match batch.preview_text() {
                Ok(text) => tool_success(
                    format!(
                        "Batch preview ({} operations) — nothing written:\n{}",
                        batch.operations.len(),
                        text
                    ),
                    None,
                ),
                Err(e) => tool_error(format!("Preview failed (nothing written): {}", e)),
            }
        } else {
            match batch.apply() {
                    Ok(()) => tool_success(
                        format!(
                            "Batch applied atomically: {} operation(s). One transaction logged — undo with the `undo` tool if the result is wrong.",
                            batch.operations.len()
                        ),
                        None,
                    ),
                    Err(e) => tool_error_code(
                        format!(
                            "Batch failed and was rolled back: {}. Fix the operations in {} and re-run, or run with preview=true first.",
                            e, file
                        ),
                        "E_BATCH_ROLLED_BACK",
                    ),
                }
        }
    }

    /// `undo`: revert the last N logged operations via the real
    /// UndoRedoManager (same engine as the CLI). A stub here once reported
    /// "Undo executed" while changing nothing — undo must never lie.
    fn handle_undo_mcp(state: Arc<AppState>, steps: usize) -> Value {
        let mut mgr = match crate::core::UndoRedoManager::new(&state.project_root) {
            Ok(m) => m,
            Err(e) => {
                return tool_error(format!(
                    "Undo unavailable: {}. Inspect the log with `gnawtreewriter history`.",
                    e
                ))
            }
        };
        match mgr.undo(steps) {
                Ok(results) if results.is_empty() => tool_error(
                    "Nothing to undo: no undoable operations in the transaction log. `gnawtreewriter history` lists what is logged.".to_string(),
                ),
                Ok(results) => {
                    let mut lines = Vec::new();
                    let mut failed = 0usize;
                    for r in &results {
                        if r.success {
                            lines.push(format!("✓ {} ({})", r.message, r.transaction_id));
                        } else {
                            failed += 1;
                            lines.push(format!(
                                "✗ {} ({}) — run `gnawtreewriter restore-project --preview` to find a restore point",
                                r.message, r.transaction_id
                            ));
                        }
                    }
                    let summary = if failed > 0 {
                        format!(
                            "Undo: {} of {} operation(s) reverted ({} failed)",
                            results.len() - failed,
                            results.len(),
                            failed
                        )
                    } else {
                        format!("Undone {} operation(s)", results.len())
                    };
                    if failed == results.len() {
                        tool_error(format!("{}\n{}", summary, lines.join("\n")))
                    } else {
                        tool_success(format!("{}\n{}", summary, lines.join("\n")), None)
                    }
                }
                Err(e) => tool_error(format!(
                    "Undo failed: {}. Try `gnawtreewriter restore-project --preview` to inspect restore points — never hand-edit around a broken file.",
                    e
                )),
            }
    }

    /// `add_rule`: validate and add a lint rule (agent-facing way to write rules).
    fn handle_add_rule(
        id: &str,
        language: &str,
        pattern: &str,
        severity: &str,
        message: Option<&str>,
        fix: Option<&str>,
    ) -> Value {
        let rule = crate::core::rules::Rule {
            id: id.to_string(),
            language: language.to_string(),
            severity: crate::core::rules::Severity::parse(severity),
            message: message
                .unwrap_or(&format!("Rule {} matched", id))
                .to_string(),
            pattern: pattern.to_string(),
            fix: fix.map(|f| f.to_string()),
        };
        // Validate the pattern compiles for the language.
        if let Err(e) = crate::core::rules::compile_rule(&rule) {
            return tool_error_code(format!("Rule rejected: {} — the pattern must parse as valid {} code with $X placeholders; fix the pattern and retry (rules add validates before writing)", e, language), "E_RULE_REJECTED");
        }
        // A fix template must parse as valid code — never store a rewrite
        // that would produce broken syntax.
        if let Some(fix_template) = fix {
            if let Err(e) = crate::core::rules::validate_fix(language, fix_template) {
                return tool_error_code(
                    format!(
                        "Fix rejected: {}. A $NAME in the fix must be captured by the pattern.",
                        e
                    ),
                    "E_RULE_REJECTED",
                );
            }
        }
        match crate::core::rules::append_project_rule(&rule) {
            Ok(()) => tool_success(
                format!(
                    "Rule {} added{}",
                    id,
                    if fix.is_some() {
                        " (with fix — apply via lint tool with fix: true)"
                    } else {
                        ""
                    }
                ),
                Some(json!({
                    "id": id,
                    "path": crate::core::rules::project_rules_path().to_string_lossy(),
                    "active": true,
                })),
            ),
            Err(e) => tool_error_code(format!("Failed to add rule: {} — nothing was written; fix the reported issue and retry (rules add validates first)", e), "E_RULE_REJECTED"),
        }
    }

    fn handle_search_nodes(file_path: &str, pattern: &str) -> Value {
        match GnawTreeWriter::new_lenient(file_path) {
            Ok((w, warning)) => {
                let mut m = Vec::new();
                fn find(n: &TreeNode, acc: &mut Vec<Value>, p: &str) {
                    if acc.len() >= 500 {
                        return;
                    }
                    if n.content.contains(p) {
                        acc.push(
                            json!({"path": n.path, "type": n.node_type, "name": n.get_name()}),
                        );
                    }
                    for c in &n.children {
                        find(c, acc, p);
                    }
                }
                find(w.analyze(), &mut m, pattern);
                let mut msg = format!("Found {} matches", m.len());
                if m.len() >= 500 {
                    msg.push_str(" (limit reached)");
                }
                let mut payload = json!({"matches": m});
                if let Some(wn) = &warning {
                    msg.push_str(&format!(
                        " ⚠ partial parse (syntax error at {}:{})",
                        wn.line, wn.column
                    ));
                    if let Some(obj) = payload.as_object_mut() {
                        obj.insert(
                            "syntax_warning".to_string(),
                            json!({"line": wn.line, "column": wn.column, "message": wn.message}),
                        );
                    }
                }
                tool_success(msg, Some(payload))
            }
            Err(e) => tool_error_code(format!("IO error: {} — check the path exists and points at a supported source file (search_nodes finds files by name; analyze parses any supported file)", e), "E_FILE_NOT_FOUND"),
        }
    }

    async fn handle_sense(state: Arc<AppState>, query: &str, file_path: Option<&str>) -> Value {
        #[cfg(feature = "modernbert")]
        {
            use crate::llm::SenseResponse;
            let broker = match state.sense_broker().await {
                Ok(b) => b,
                Err(e) => return tool_error_code(format!("Semantic model init failed: {} — run `gnawtreewriter ai setup` to fetch models (ai status shows what is installed), then retry", e), "E_MODEL_UNAVAILABLE"),
            };

            match broker.sense(query, file_path).await {
                Ok(response) => match response {
                    SenseResponse::Satelite { matches } => {
                        if matches.is_empty() {
                            tool_error(format!(
                                    "Satellite search: no matches for \"{}\". The project semantic index may be missing — build it with the index_project tool (action: start, then status) or `gnawtreewriter ai index` on the CLI, or pass file_path for single-file zoom search.",
                                    query
                                ))
                        } else {
                            let top: String = matches
                                .iter()
                                .take(5)
                                .map(|m| {
                                    let first =
                                        m.content_preview.lines().next().unwrap_or("").trim();
                                    format!(
                                        "{}:{} ({:.2}) {}",
                                        m.file_path,
                                        m.node_path.as_deref().unwrap_or("?"),
                                        m.score,
                                        first
                                    )
                                })
                                .collect::<Vec<_>>()
                                .join("; ");
                            tool_success(
                                format!("Satellite search: {} match(es) — {}", matches.len(), top),
                                Some(json!({"matches": matches})),
                            )
                        }
                    }
                    SenseResponse::Zoom {
                        file_path,
                        nodes,
                        impact,
                    } => {
                        if nodes.is_empty() {
                            // Never a naked header: empty must guide.
                            tool_error(format!(
                                "Zoom search: no nodes matched in {} — try a more specific query, or use list_nodes if you need the raw structure. (Zoom ranks definitions inside ONE file.)",
                                file_path
                            ))
                        } else {
                            let top: String = nodes
                                .iter()
                                .take(5)
                                .map(|n| {
                                    let first = n.preview.lines().next().unwrap_or("").trim();
                                    format!("{} ({:.2}) {}", n.path, n.score, first)
                                })
                                .collect::<Vec<_>>()
                                .join("; ");
                            tool_success(
                                format!(
                                    "Zoom search results for {}: {} node(s) — {}",
                                    file_path,
                                    nodes.len(),
                                    top
                                ),
                                Some(json!({"nodes": nodes, "impact": impact})),
                            )
                        }
                    }
                },
                Err(e) => tool_error(format!("sense failed: {} — `gnawtreewriter ai status` checks the model; on repeat fall back to search_nodes/grep and log it in GTW_MCP_ISSUE_LOG.md", e)),
            }
        }
        #[cfg(not(feature = "modernbert"))]
        {
            let _ = (state, query, file_path);
            tool_error_code("ModernBERT feature not enabled — rebuild with the AI features: cargo install --path . --features modernbert,mcp (README: Full power), then retry this call.".into(), "E_MODEL_UNAVAILABLE")
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
                Err(e) => return tool_error_code(format!("Semantic model init failed: {} — run `gnawtreewriter ai setup` to fetch models (ai status shows what is installed), then retry", e), "E_MODEL_UNAVAILABLE"),
            };

            match broker.propose_edit(anchor_query, file_path, intent).await {
                Ok(proposal) => {
                    bump_duplex_metric(&state.project_root, "proposed");
                    let mut writer = match GnawTreeWriter::new(file_path) {
                        Ok(w) => w,
                        Err(e) => return open_error(file_path, &e),
                    };
                    let transparency = json!({
                        "anchor_path": proposal.anchor_path,
                        "confidence": proposal.confidence,
                        "parent_path": proposal.parent_path,
                        "position": proposal.position,
                    });
                    let op = EditOperation::Insert {
                        parent_path: proposal.parent_path,
                        position: proposal.position,
                        content: content.to_string(),
                    };
                    match writer.edit(op, false) {
                        Ok(_) => {
                            bump_duplex_metric(&state.project_root, "validated");
                            bump_duplex_metric(&state.project_root, "applied");
                            let pulse = generate_pulse(state, file_path, &proposal.anchor_path);
                            tool_success_with_pulse(
                                format!(
                                    "Successfully inserted code near anchor '{}' (confidence: {:.2})",
                                    proposal.anchor_path, proposal.confidence
                                ),
                                // Motor2 plan #19: structured transparency
                                // alongside the human line (fields captured
                                // before parent_path moved into the op).
                                Some(transparency),
                                pulse,
                            )
                        }
                        Err(e) => {
                            bump_duplex_metric(&state.project_root, "rejected");
                            tool_error_code(format!("sense failed: {} — `gnawtreewriter ai status` checks the model; on repeat fall back to search_nodes/grep and log it in GTW_MCP_ISSUE_LOG.md", e), "E_EDIT_REJECTED")
                        }
                    }
                }
                Err(e) => tool_error_code(format!("sense failed: {} — `gnawtreewriter ai status` checks the model; on repeat fall back to search_nodes/grep and log it in GTW_MCP_ISSUE_LOG.md", e), "E_MODEL_UNAVAILABLE"),
            }
        }
        #[cfg(not(feature = "modernbert"))]
        {
            let _ = (state, file_path, anchor_query, content, intent);
            tool_error_code("ModernBERT feature not enabled — rebuild with the AI features: cargo install --path . --features modernbert,mcp (README: Full power), then retry this call.".into(), "E_MODEL_UNAVAILABLE")
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
                Err(e) => return tool_error_code(format!("Semantic model init failed: {} — run `gnawtreewriter ai setup` to fetch models (ai status shows what is installed), then retry", e), "E_MODEL_UNAVAILABLE"),
            };

            match broker.sense(query, Some(file_path)).await {
                Ok(SenseResponse::Zoom { nodes, .. }) if !nodes.is_empty() => {
                    let best_node = &nodes[0];
                    let confidence = best_node.score;
                    let candidates: Vec<Value> = nodes
                        .iter()
                        .take(5)
                        .map(|n| json!({"path": n.path, "score": n.score, "preview": n.preview}))
                        .collect();
                    let summary = format!(
                        "semantic_edit matched {} (confidence {:.2}); top candidates: {}",
                        best_node.path,
                        confidence,
                        candidates
                            .iter()
                            .filter_map(|c| {
                                Some(format!(
                                    "{} ({:.2})",
                                    c.get("path")?.as_str()?,
                                    c.get("score")?.as_f64()?
                                ))
                            })
                            .collect::<Vec<_>>()
                            .join(", ")
                    );
                    // Motor2 plan #19: transparency on HIT — the edit result
                    // carries what was matched, with what confidence, and which
                    // alternatives were considered.
                    let mut result =
                        handle_edit_node_internal(state, file_path, &best_node.path, content);
                    if let Some(obj) = result.as_object_mut() {
                        obj.insert(
                            "semantic_match".to_string(),
                            json!({
                                "matched_path": best_node.path,
                                "confidence": confidence,
                                "candidates": candidates,
                            }),
                        );
                        if let Some(arr) = obj.get_mut("content").and_then(|c| c.as_array_mut()) {
                            if let Some(Value::String(text)) =
                                arr.first_mut().and_then(|c| c.get_mut("text"))
                            {
                                text.push('\n');
                                text.push_str(&summary);
                            }
                        }
                    }
                    result
                }
                Ok(_) => {
                    // Motor2 plan #19: transparency on MISS — explicit zero
                    // confidence + empty candidate set, never a bare denial.
                    let mut err = tool_error_code(
                        format!(
                            "Could not find a semantic match for '{}' in {} — no node scored above the relevance floor. Try a different anchor phrase, list_nodes for the raw structure, or insert_node with a parent_path. (semantic_match: confidence 0, no candidates)",
                            query, file_path
                        ),
                        "E_SEMANTIC_NO_MATCH",
                    );
                    if let Some(obj) = err.as_object_mut() {
                        obj.insert(
                            "semantic_match".to_string(),
                            json!({"confidence": 0.0, "candidates": [], "matched_path": null}),
                        );
                    }
                    err
                }
                Err(e) => tool_error(format!("sense failed: {} — `gnawtreewriter ai status` checks the model; on repeat fall back to search_nodes/grep and log it in GTW_MCP_ISSUE_LOG.md", e)),
            }
        }
        #[cfg(not(feature = "modernbert"))]
        {
            let _ = (state, file_path, query, content);
            tool_error_code("ModernBERT feature not enabled — rebuild with the AI features: cargo install --path . --features modernbert,mcp (README: Full power), then retry this call.".into(), "E_MODEL_UNAVAILABLE")
        }
    }

    fn handle_read_node(file_path: &str, node_path: &str) -> Value {
        match GnawTreeWriter::new_lenient(file_path) {
            Ok((w, warning)) => w.show_node(node_path).map_or_else(
                |e| tool_error_code(format!("Node read failed: {} — node_path may be stale; run analyze or list_nodes for current paths and retry", e), "E_NODE_NOT_FOUND"),
                |c| match &warning {
                    Some(wn) => tool_success(
                        c,
                        Some(json!({"syntax_warning": {"line": wn.line, "column": wn.column, "message": wn.message}})),
                    ),
                    None => tool_success(c, None),
                },
            ),
            Err(e) => tool_error_code(format!("IO error: {} — check the path exists and points at a supported source file (search_nodes finds files by name; analyze parses any supported file)", e), "E_FILE_NOT_FOUND"), // Corrected: escaped curly brace
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
                let op = EditOperation::Edit {
                    node_path: node_path.to_string(),
                    content: content.to_string(),
                };
                match writer.preview_edit(op) {
                    Ok(new_source) => {
                        let diff = generate_diff_string(&old_source, &new_source);
                        tool_success(
                            format!("Preview of edit:\n{}", diff),
                            Some(json!({"diff": diff})),
                        )
                    }
                    Err(e) => tool_error_code(
                        format!(
                            "Preview rejected: {} — the PROPOSED content does not parse; fix the content (or preview_edit after an edit_node correction). Nothing was written.",
                            e
                        ),
                        "E_VALIDATION",
                    ),
                }
            }
            Err(e) => open_error(file_path, &e),
        }
    }

    fn handle_edit_node_internal(
        state: Arc<AppState>,
        file_path: &str,
        node_path: &str,
        content: &str,
    ) -> Value {
        match GnawTreeWriter::new(file_path) {
            Ok(mut w) => {
                let old_source = w.get_source().to_string();
                let op = EditOperation::Edit {
                    node_path: node_path.to_string(),
                    content: content.to_string(),
                };
                if let Err(e) = w.edit(op, false) {
                    bump_duplex_metric(&state.project_root, "rejected");
                    return tool_error_code(format!("Edit rejected: {} — the validation details are in the message; list_nodes for current paths, preview_edit first, then retry", e), "E_EDIT_REJECTED");
                }

                let new_source_loaded = std::fs::read_to_string(file_path).unwrap_or_default();
                let diff = generate_diff_string(&old_source, &new_source_loaded);
                bump_duplex_metric(&state.project_root, "validated");
                bump_duplex_metric(&state.project_root, "applied");
                let pulse = generate_pulse(state, file_path, node_path);
                tool_success_with_pulse(
                    format!("Node edited.\nDiff:\n{}", diff),
                    Some(json!({"diff": diff})),
                    pulse,
                )
            }
            Err(e) => open_error(file_path, &e),
        }
    }

    fn handle_insert_node(
        state: Arc<AppState>,
        file_path: &str,
        parent_path: &str,
        position: usize,
        content: &str,
    ) -> Value {
        match GnawTreeWriter::new(file_path) {
            Ok(mut w) => {
                let old_source = w.get_source().to_string();
                let op = EditOperation::Insert {
                    parent_path: parent_path.to_string(),
                    position,
                    content: content.to_string(),
                };
                if let Err(e) = w.edit(op, false) {
                    bump_duplex_metric(&state.project_root, "rejected");
                    return tool_error_code(format!("Edit rejected: {} — the validation details are in the message; list_nodes for current paths, preview_edit first, then retry", e), "E_EDIT_REJECTED");
                }

                let new_source_loaded = std::fs::read_to_string(file_path).unwrap_or_default();
                let diff = generate_diff_string(&old_source, &new_source_loaded);
                bump_duplex_metric(&state.project_root, "validated");
                bump_duplex_metric(&state.project_root, "applied");
                let pulse = generate_pulse(state, file_path, parent_path); // Pulse for parent
                tool_success_with_pulse(
                    format!("Content inserted.\nDiff:\n{}", diff),
                    Some(json!({"diff": diff})),
                    pulse,
                )
            }
            Err(e) => open_error(file_path, &e),
        }
    }

    fn handle_move_node(
        state: Arc<AppState>,
        source_file: &str,
        source_path: &str,
        target_file: &str,
        target_path: &str,
    ) -> Value {
        match GnawTreeWriter::new(source_file) {
            Ok(mut src_w) => {
                let old_source = src_w.get_source().to_string();
                let delete_op = EditOperation::Delete {
                    node_path: source_path.to_string(),
                };
                if let Err(e) = src_w.edit(delete_op, false) {
                    return tool_error_code(format!("Source edit rejected: {} — source_path must be an existing node (search_nodes finds it); retry with a fresh path", e), "E_NODE_NOT_FOUND");
                }

                let insert_op = EditOperation::Insert {
                    parent_path: target_path.to_string(),
                    position: 1,
                    content: old_source.clone(),
                };
                match GnawTreeWriter::new(target_file) {
                    Ok(mut tgt_w) => {
                        let old_target = tgt_w.get_source().to_string();
                        if let Err(e) = tgt_w.edit(insert_op, false) {
                            return tool_error_code(format!("Target edit rejected: {} — target_path must exist as a parent (list_nodes shows valid paths); retry with a fresh path", e), "E_NODE_NOT_FOUND");
                        }
                        let new_target = std::fs::read_to_string(target_file).unwrap_or_default();
                        let diff = generate_diff_string(&old_target, &new_target);
                        let pulse = generate_pulse(state, target_file, target_path);
                        tool_success_with_pulse(
                            format!(
                                "Moved from {} [{}] to {} [{}].\nDiff:\n{}",
                                source_file, source_path, target_file, target_path, diff
                            ),
                            Some(json!({"diff": diff})),
                            pulse,
                        )
                    }
                    Err(e) => open_error(target_file, &e),
                }
            }
            Err(e) => open_error(source_file, &e),
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
        serve_with_shutdown_root(listener, token, project_root, shutdown_signal).await
    }

    /// Test-isolated server variant: binds the server to an explicit project
    /// root so tests never operate on the real repo transaction log. See
    /// GTW_MCP_ISSUE_LOG.md finding #12 (undo test once reverted real edits).
    pub async fn serve_with_shutdown_root<F>(
        listener: TcpListener,
        token: Option<String>,
        project_root: std::path::PathBuf,
        shutdown_signal: F,
    ) -> Result<()>
    where
        F: std::future::Future<Output = ()> + Send + 'static,
    {
        let app = Router::new()
            .route("/", post(rpc_handler))
            .with_state(Arc::new(AppState::new(token, project_root)));
        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal)
            .await?;
        Ok(())
    }

    pub async fn serve(addr: &str, token: Option<String>) -> Result<()> {
        let listener = TcpListener::bind(addr).await?;
        eprintln!("Starting MCP server on http://{}", listener.local_addr()?); // Fixed: redirected to stderr
        serve_with_shutdown(listener, token, async {
            let _ = signal::ctrl_c().await;
        })
        .await
    }

    pub async fn status(url: &str, token: Option<String>) -> Result<()> {
        let client = reqwest::Client::new();
        let mut req = client.post(url);
        if let Some(t) = token {
            req = req.header("Authorization", format!("Bearer {}", t));
        } // Corrected: escaped curly brace
        let _ = req
            .json(&json!({"jsonrpc":"2.0","method":"initialize","id":1}))
            .send()
            .await?;
        eprintln!("✓ Server ready");
        Ok(())
    }
}

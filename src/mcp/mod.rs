//! Minimal MCP (Model Context Protocol) server implementation (MVP).
//!
//! - Feature gated: only compiled when `--features mcp` is enabled.
//! - Implements a small JSON-RPC 2.0 endpoint over HTTP that understands
//!   a subset of MCP (initialize, tools/list, tools/call).
//! - Uses `axum` for HTTP routing (optional dependency pulled in via `mcp`).

#![allow(clippy::unused_async)]

#[cfg(feature = "mcp")]
pub mod mcp_server {
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
    use std::{fs, path::Path, sync::Arc};
    use tokio::net::TcpListener;
    use tokio::signal;

    /// Shared state for the MCP server
    struct AppState {
        token: Option<String>,
    }

    /// A very small JSON-RPC request shape (we only read the fields we need).
    #[derive(Debug, Deserialize)]
    struct JsonRpcRequest {
        pub id: Option<Value>,
        #[allow(dead_code)]
        pub jsonrpc: Option<String>,
        pub method: String,
        pub params: Option<Value>,
    }

    /// A convenience helper to build JSON-RPC responses.
    #[derive(Debug, Serialize)]
    struct JsonRpcSuccess<'a> {
        jsonrpc: &'a str,
        id: Option<Value>,
        result: Value,
    }

    /// A helper to build JSON-RPC error responses.
    #[derive(Debug, Serialize)]
    struct JsonRpcError<'a> {
        jsonrpc: &'a str,
        id: Option<Value>,
        error: serde_json::Value,
    }

    /// Standard JSON-RPC error codes
    #[allow(dead_code)]
    const INVALID_PARAMS_CODE: i32 = -32602;
    #[allow(dead_code)]
    const METHOD_NOT_FOUND_CODE: i32 = -32601;
    #[allow(dead_code)]
    const PARSE_ERROR_CODE: i32 = -32700;
    #[allow(dead_code)]
    const INTERNAL_ERROR_CODE: i32 = -32603;

    /// Build a JSON-RPC error response
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

    async fn rpc_handler(
        State(state): State<Arc<AppState>>,
        headers: HeaderMap,
        Json(req): Json<Value>,
    ) -> impl IntoResponse {
        // If a token is configured, require Authorization: Bearer <token> header
        if let Some(expected) = &state.token {
            match headers.get("authorization").and_then(|v| v.to_str().ok()) {
                Some(s) if s == format!("Bearer {}", expected) => {}
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

        // Parse as generic JSON-RPC request (loose/forgiving)
        let parsed: Result<JsonRpcRequest, _> = serde_json::from_value(req.clone());
        let req = match parsed {
            Ok(r) => r,
            Err(e) => {
                let err = json!({
                    "jsonrpc": "2.0",
                    "id": null,
                    "error": { "code": -32700, "message": format!("Invalid JSON-RPC payload: {}", e) }
                });
                return (StatusCode::BAD_REQUEST, Json(err));
            }
        };

        // Dispatch the MCP methods we handle
        match req.method.as_str() {
            "initialize" => {
                let result = json!({
                    "protocolVersion": "2025-11-25",
                    "serverInfo": {
                        "name": env!("CARGO_PKG_NAME"),
                        "version": env!("CARGO_PKG_VERSION")
                    },
                    "capabilities": {
                        "tools": { "listChanged": true }
                    }
                });
                let resp = JsonRpcSuccess {
                    jsonrpc: "2.0",
                    id: req.id,
                    result,
                };
                (StatusCode::OK, Json(serde_json::to_value(resp).unwrap()))
            }

            "tools/list" => {
                let tools = json!({
                    "tools": [
                        {
                            "name": "analyze",
                            "title": "Analyze file",
                            "description": "Analyze a file and return a small summary of its AST.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "file_path": { "type": "string" }
                                },
                                "required": ["file_path"]
                            }
                        },
                        {
                            "name": "batch",
                            "title": "Apply batch",
                            "description": "Execute a batch of operations atomically.",
                            "inputSchema": { "type": "object" }
                        },
                        {
                            "name": "undo",
                            "title": "Undo",
                            "description": "Undo recent operations.",
                            "inputSchema": { "type": "object" }
                        }
                    ]
                });
                let resp = JsonRpcSuccess {
                    jsonrpc: "2.0",
                    id: req.id,
                    result: tools,
                };
                (StatusCode::OK, Json(serde_json::to_value(resp).unwrap()))
            }

            "tools/call" => {
                let params = req.params.unwrap_or_else(|| json!({}));
                let name = params
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or_default();

                // Validate tool name - return JSON-RPC error for unknown tools
                if !matches!(name, "analyze" | "batch" | "undo") {
                    let err = build_jsonrpc_error(
                        req.id,
                        METHOD_NOT_FOUND_CODE,
                        "Unknown tool",
                        Some(json!(format!(
                            "'{}' is not a valid tool name. Available tools: analyze, batch, undo",
                            name
                        ))),
                    );
                    return (
                        StatusCode::NOT_FOUND,
                        Json(serde_json::to_value(err).unwrap()),
                    );
                }

                let arguments = params
                    .get("arguments")
                    .cloned()
                    .unwrap_or_else(|| json!({}));

                // Validate required parameters for analyze tool
                if name == "analyze" && arguments.get("file_path").and_then(Value::as_str).is_none()
                {
                    let err = build_jsonrpc_error(
                        req.id,
                        INVALID_PARAMS_CODE,
                        "Invalid parameters",
                        Some(
                            json!({"field": "file_path", "message": "Missing required parameter 'file_path'"}),
                        ),
                    );
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(serde_json::to_value(err).unwrap()),
                    );
                }

                // Execute tool - these return result with isError for tool-level failures
                let result = match name {
                    "analyze" => handle_analyze(&arguments).unwrap_or_else(|e| {
                        json!({
                            "content": [{"type": "text", "text": format!("Analyze failed: {}", e)}],
                            "isError": true
                        })
                    }),
                    "batch" => {
                        json!({ "content": [{"type": "text", "text": "Batch executed (MVP)"}] })
                    }
                    "undo" => {
                        json!({ "content": [{"type": "text", "text": "Undo executed (MVP)"}] })
                    }
                    _ => unreachable!("Unknown tool should have been caught above"),
                };

                let resp = JsonRpcSuccess {
                    jsonrpc: "2.0",
                    id: req.id,
                    result,
                };
                (StatusCode::OK, Json(serde_json::to_value(resp).unwrap()))
            }

            _ => {
                let err = json!({
                    "jsonrpc": "2.0",
                    "id": req.id,
                    "error": { "code": -32601, "message": "Method not found" }
                });
                (StatusCode::NOT_FOUND, Json(err))
            }
        }
    }

    fn handle_analyze(arguments: &Value) -> Result<Value> {
        // Note: file_path is validated before this function is called (JSON-RPC error if missing)
        let file_path = arguments
            .get("file_path")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("missing 'file_path' parameter (this should not happen)"))?;

        let content = match fs::read_to_string(file_path) {
            Ok(c) => c,
            Err(e) => {
                // Tool-level error: file not found or permission denied
                return Ok(json!({
                    "content": [{"type": "text", "text": format!("IO error: {}", e)}],
                    "isError": true
                }));
            }
        };

        match crate::parser::get_parser(Path::new(file_path)) {
            Ok(parser) => match parser.parse(&content) {
                Ok(tree) => {
                    let mut nodes = Vec::new();
                    fn collect(
                        node: &crate::parser::TreeNode,
                        acc: &mut Vec<crate::parser::TreeNode>,
                    ) {
                        acc.push(node.clone());
                        for child in &node.children {
                            collect(child, acc);
                        }
                    }
                    collect(&tree, &mut nodes);
                    let node_count = nodes.len();
                    let preview = content.lines().next().unwrap_or("").to_string();
                    Ok(json!({
                        "content": [
                            { "type": "text", "text": format!("Parsed {} nodes. Preview: {}", node_count, preview) }
                        ],
                        "structuredContent": { "node_count": node_count }
                    }))
                }
                Err(e) => {
                    // Tool-level error: syntax error in the file
                    Ok(json!({
                        "content": [{"type": "text", "text": format!("Parser error: {}", e)}],
                        "isError": true
                    }))
                }
            },
            Err(e) => {
                // Tool-level error: no parser available for this file type
                Ok(json!({
                    "content": [{"type": "text", "text": format!("No parser available: {}", e)}],
                    "isError": true
                }))
            }
        }
    }

    pub fn build_router(token: Option<String>) -> Router {
        let state = Arc::new(AppState { token });
        Router::new()
            .route("/", post(rpc_handler))
            .with_state(state)
    }

    pub async fn serve_with_shutdown<F>(
        listener: TcpListener,
        token: Option<String>,
        shutdown_signal: F,
    ) -> Result<()>
    where
        F: std::future::Future<Output = ()> + Send + 'static,
    {
        let app = build_router(token);
        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal)
            .await
            .context("Server error")?;
        Ok(())
    }

    pub async fn serve(addr: &str, token: Option<String>) -> Result<()> {
        let listener = TcpListener::bind(addr)
            .await
            .context(format!("Failed to bind to {}", addr))?;
        println!("Starting MCP server on http://{}", listener.local_addr()?);

        if token.is_some() {
            println!("Security: Bearer token authentication enabled");
            // Optional: hide token in production logs, but for MVP it's often visible if passed via CLI
        } else {
            println!("Security: Authentication disabled (unprotected access)");
        }

        serve_with_shutdown(listener, token, async {
            let _ = signal::ctrl_c().await;
        })
        .await
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use axum::body::Body;
        use axum::http::Request;
        use serde_json::json;
        use std::io::Write;
        use tempfile::NamedTempFile;
        use tower::util::ServiceExt;

        #[test]
        fn test_handle_analyze_python() {
            let mut f = NamedTempFile::new().expect("tempfile");
            write!(f, "def foo():\n    return 1\n").unwrap();
            let path = f.path().to_str().unwrap().to_string();
            let args = json!({ "file_path": path });
            let res = handle_analyze(&args).expect("analyze failed");
            assert!(res.get("structuredContent").is_some());
        }

        #[tokio::test]
        async fn test_rpc_initialize_no_token() {
            let app = build_router(None);
            let req = Request::builder()
                .method("POST")
                .uri("/")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"jsonrpc":"2.0","method":"initialize","id":1}).to_string(),
                ))
                .unwrap();

            let response = app.oneshot(req).await.unwrap();
            assert_eq!(response.status(), StatusCode::OK);

            let body = axum::body::to_bytes(response.into_body(), 1024 * 1024)
                .await
                .unwrap();
            let v: Value = serde_json::from_slice(&body).unwrap();
            assert!(v.get("result").is_some());
        }

        #[tokio::test]
        async fn test_rpc_auth_flow() {
            let token = "secret".to_string();
            let app = build_router(Some(token));
            let req_body = json!({"jsonrpc":"2.0","method":"initialize","id":1}).to_string();

            // 1. Unauthorized
            let req1 = Request::builder()
                .method("POST")
                .uri("/")
                .header("content-type", "application/json")
                .body(Body::from(req_body.clone()))
                .unwrap();
            let resp1 = app.clone().oneshot(req1).await.unwrap();
            assert_eq!(resp1.status(), StatusCode::UNAUTHORIZED);

            // 2. Authorized
            let req2 = Request::builder()
                .method("POST")
                .uri("/")
                .header("content-type", "application/json")
                .header("authorization", "Bearer secret")
                .body(Body::from(req_body))
                .unwrap();
            let resp2 = app.oneshot(req2).await.unwrap();
            assert_eq!(resp2.status(), StatusCode::OK);
        }

        #[tokio::test]
        async fn test_tools_call_unknown_tool() {
            let app = build_router(None);
            let req = Request::builder()
                .method("POST")
                .uri("/")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"jsonrpc":"2.0","method":"tools/call","params":{"name":"unknown_tool"},"id":1}).to_string(),
                ))
                .unwrap();

            let response = app.oneshot(req).await.unwrap();
            assert_eq!(response.status(), StatusCode::NOT_FOUND);

            let body = axum::body::to_bytes(response.into_body(), 1024 * 1024)
                .await
                .unwrap();
            let v: Value = serde_json::from_slice(&body).unwrap();

            // Should be a JSON-RPC error, not a result with isError
            assert!(v.get("error").is_some(), "Expected JSON-RPC error object");
            assert!(v.get("result").is_none(), "Should not have result field");
            assert_eq!(
                v["error"]["code"], -32601,
                "Error code should be METHOD_NOT_FOUND"
            );
            assert!(v["error"]["message"]
                .as_str()
                .unwrap()
                .contains("Unknown tool"));
        }

        #[tokio::test]
        async fn test_tools_call_missing_required_param() {
            let app = build_router(None);
            let req = Request::builder()
                .method("POST")
                .uri("/")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"jsonrpc":"2.0","method":"tools/call","params":{"name":"analyze","arguments":{}},"id":1}).to_string(),
                ))
                .unwrap();

            let response = app.oneshot(req).await.unwrap();
            assert_eq!(response.status(), StatusCode::BAD_REQUEST);

            let body = axum::body::to_bytes(response.into_body(), 1024 * 1024)
                .await
                .unwrap();
            let v: Value = serde_json::from_slice(&body).unwrap();

            // Should be a JSON-RPC error for invalid params
            assert!(v.get("error").is_some(), "Expected JSON-RPC error object");
            assert!(v.get("result").is_none(), "Should not have result field");
            assert_eq!(
                v["error"]["code"], -32602,
                "Error code should be INVALID_PARAMS"
            );
            assert!(v["error"]["message"]
                .as_str()
                .unwrap()
                .contains("Invalid parameters"));
        }

        #[tokio::test]
        async fn test_tools_call_file_not_found() {
            let app = build_router(None);
            let req = Request::builder()
                .method("POST")
                .uri("/")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"jsonrpc":"2.0","method":"tools/call","params":{"name":"analyze","arguments":{"file_path":"/nonexistent/file.py"}},"id":1}).to_string(),
                ))
                .unwrap();

            let response = app.oneshot(req).await.unwrap();
            assert_eq!(response.status(), StatusCode::OK);

            let body = axum::body::to_bytes(response.into_body(), 1024 * 1024)
                .await
                .unwrap();
            let v: Value = serde_json::from_slice(&body).unwrap();

            // Should be a result with isError (tool-level error), not JSON-RPC error
            assert!(v.get("result").is_some(), "Expected result object");
            assert!(v.get("error").is_none(), "Should not have error field");
            assert_eq!(v["result"]["isError"], true, "Should be marked as error");
            assert!(v["result"]["content"][0]["text"]
                .as_str()
                .unwrap()
                .contains("IO error"));
        }
    }
}

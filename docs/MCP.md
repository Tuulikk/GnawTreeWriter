# MCP Server — Model Context Protocol (MVP)

This document describes the minimal MCP (Model Context Protocol) server implementation included in GnawTreeWriter. It covers the server behavior, supported JSON-RPC methods, authentication, testing patterns, how to add new tools, recommended production practices, and examples.

> Short summary: the MCP server is an MVP JSON-RPC 2.0 endpoint (HTTP POST) that supports a small subset of MCP functionality (currently `initialize`, `tools/list`, and `tools/call`). It's feature-gated behind the `mcp` Cargo feature.

---

Table of contents
- Overview
- Running server (CLI & env)
- Checking server status
- JSON-RPC: request/response shape
- Supported methods
  - initialize
  - tools/list
  - tools/call (analyze, batch, undo — MVP)
- Authentication
- Examples (curl & code)
- Testing (unit & integration)
- Extending MCP server (add a tool)
- Security & deployment recommendations
- Troubleshooting and tips

---

## Overview

The MCP server is designed for a simple agent integration workflow:
- Exposes a single HTTP endpoint accepting JSON-RPC 2.0 POSTs.
- Implements a minimal set of MCP-style semantics (tool discovery and tool invocation).
- Intended for local, development, and simple agent scenarios (MVP).
- Uses token-based Bearer auth (optional).
- Implemented with `axum` and `tokio`.

The server is compiled only when `--features mcp` is enabled.

---

## Checking server status

The `mcp status` command queries a running MCP server to display server information and available tools.

```bash
gnawtreewriter mcp status --url <URL> [--token <TOKEN>]
```

**Options:**
- `--url <URL>`: Server URL (default: `http://127.0.0.1:8080/`)
- `--token <TOKEN>`: Optional bearer token (can also be set via `MCP_TOKEN` env var)

**Output includes:**
- Server name and version
- MCP protocol version
- Server capabilities
- List of available tools with descriptions

**Example output:**
```
=== MCP Server Status ===
Name: gnawtreewriter
Version: 0.6.0
Protocol: 2025-11-25

Capabilities:
  - tools: {"listChanged":true}

=== Available Tools ===

  • analyze
    Title: Analyze file
    Description: Analyze a file and return a small summary of its AST.

  • batch
    Title: Apply batch
    Description: Execute a batch of operations atomically.

  • undo
    Title: Undo
    Description: Undo recent operations.

=== Server is responding correctly ===
```

---

## Running the server

You can run the server via the CLI subcommand:

- Build (with MCP feature):
```bash
cargo install --path . --features mcp
```

- Run via included CLI (example):
```bash
# Run with explicit token
gnawtreewriter mcp serve --addr 127.0.0.1:8080 --token secret

# Or use an environment variable
MCP_TOKEN=secret gnawtreewriter mcp serve --addr 127.0.0.1:8080
```

- Check server status (requires server running):
```bash
# Check status with explicit token
gnawtreewriter mcp status --url http://127.0.0.1:8080/ --token secret

# Or use environment variable
MCP_TOKEN=secret gnawtreewriter mcp status --url http://127.0.0.1:8080/
```

The server binds to the address you provide and begins accepting JSON-RPC POSTs at `/`.

---

## JSON-RPC: shape, expectations and examples

Requests and responses follow JSON-RPC 2.0 basics.

- Request:
```json
{
  "jsonrpc": "2.0",
  "method": "<method_name>",
  "id": 1,
  "params": { ... }
}
```

- Successful response:
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": { ... }
}
```

- Error response (method not found / invalid payload, etc.) follows JSON-RPC standard error object:
```json
{
  "jsonrpc": "2.0",
  "id": null,
  "error": { "code": -32601, "message": "Method not found" }
}
```

### Error handling: JSON-RPC error vs tool error

The server distinguishes between protocol-level errors and tool-level failures:

**JSON-RPC `error` (protocol-level failures)**:
- **Authentication failures** (`code`: -32001)
- **Invalid JSON-RPC payload** (`code`: -32700)
- **Unknown method** (`code`: -32601)
- **Unknown tool name** (`code`: -32601) - e.g., calling "invalid_tool_name"
- **Missing required parameters** (`code`: -32602) - e.g., `analyze` without `file_path`

These errors indicate problems with the request itself and should not be retried without fixing the request.

**`result.isError = true` (tool-level failures)**:
- **File not found** - IO error when reading the specified file
- **No parser available** - File extension not supported
- **Tool execution errors** - Runtime failures during tool execution

These errors indicate the request was valid but the tool encountered issues with the data or environment. These may be retryable after fixing the underlying issue (e.g., creating the file).

**Client handling pattern**:
```javascript
// 1. Check for JSON-RPC protocol errors first
if (response.error) {
  // Protocol error - fix the request
  console.error("Protocol error:", response.error.message);
  return;
}

// 2. Check for tool-level errors
if (response.result?.isError) {
  // Tool error - may be retryable
  console.error("Tool error:", response.result.content[0].text);
  return;
}

// 3. Success
console.log("Success:", response.result);
```

The server is forgiving (it first parses value into a `JsonRpcRequest` that only reads fields needed).

---

## Supported methods

### initialize
- Purpose: Negotiate basic protocol information and advertise capabilities.
- Example response:
```json
{
  "jsonrpc":"2.0",
  "id":1,
  "result": {
    "protocolVersion": "2025-11-25",
    "serverInfo": { "name": "...", "version": "..." },
    "capabilities": { "tools": { "listChanged": true } }
  }
}
```

### tools/list
- Purpose: Return the currently available tools and their input schemas.
- Example result structure:
```json
{
  "jsonrpc":"2.0",
  "id":2,
  "result": {
    "tools": [
      { "name": "analyze", "title": "Analyze file", "inputSchema": { "type":"object" } },
      { "name": "batch", "title": "Apply batch", "inputSchema": { "type":"object" } }
    ]
  }
}
```

### tools/call
- Purpose: Invoke a named tool, passing `arguments`.
- Expected `params` shape:
```json
{
  "name": "analyze",
  "arguments": { "file_path": "<path>" }
}
```

- Tool responses are returned in `result` — typically with a `content` (presentation) array and optionally `structuredContent`.
- Example `analyze` result:
```json
{
  "jsonrpc":"2.0",
  "id":3,
  "result": {
    "content": [{"type":"text","text":"Parsed 42 nodes. Preview: def foo()..."}],
    "structuredContent": {"node_count": 42}
  }
}
```

- Error case for analyze when `file_path` is missing (JSON-RPC `error`):
```json
{
  "jsonrpc":"2.0",
  "id":3,
  "error": {
    "code": -32602,
    "message": "Invalid parameters",
    "data": {
      "field": "file_path",
      "message": "Missing required parameter 'file_path'"
    }
  }
}
```

- Error case for analyze when file is not found (tool-level `isError`):
```json
{
  "jsonrpc":"2.0",
  "id":4,
  "result": {
    "content": [{"type":"text","text":"IO error: No such file or directory (os error 2)"}],
    "isError": true
  }
}
```

- Error case for unknown tool (JSON-RPC `error`):
```json
{
  "jsonrpc":"2.0",
  "id":5,
  "error": {
    "code": -32601,
    "message": "Unknown tool",
    "data": "'invalid_tool' is not a valid tool name. Available tools: analyze, batch, undo"
  }
}
```

> Note: See "Error handling" section above for the complete distinction between JSON-RPC errors and tool-level errors.

---

## Authentication

The server supports optional bearer token authentication.
- If a token is configured (CLI `--token` or `MCP_TOKEN` env var), requests must include:
  ```
  Authorization: Bearer <token>
  ```
- Missing or incorrect token => HTTP 401 with JSON-RPC error payload:
```json
{
  "jsonrpc":"2.0",
  "id": null,
  "error": { "code": -32001, "message":"Unauthorized" }
}
```

If authentication is disabled (no token), requests are accepted from any client (careful in production).

---

## Examples

- Initialize with curl:
```bash
curl -X POST http://127.0.0.1:8080/ \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer secret" \
  -d '{"jsonrpc":"2.0","method":"initialize","id":1}'
```

- List tools:
```bash
curl -X POST http://127.0.0.1:8080/ \
  -H "Authorization: Bearer secret" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"tools/list","id":2}'
```

- Call analyze:
```bash
curl -X POST http://127.0.0.1:8080/ \
  -H "Authorization: Bearer secret" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"tools/call","id":3,"params":{"name":"analyze","arguments":{"file_path":"examples/hello.py"}}}'
```

---

## Testing the MCP server

- Unit tests: test small components and `handle_analyze` logic using direct calls to functions.
- Integration tests: start a server on an ephemeral `TcpListener`, wait for ready, then make `reqwest` calls.

A typical integration test pattern (Rust + Tokio):
1. Bind to a free port:
   ```rust
   let listener = TcpListener::bind("127.0.0.1:0").await?;
   let addr = listener.local_addr()?;
   ```
2. Start server with a shutdown signal:
   ```rust
   let (tx, rx) = oneshot::channel();
   let server = tokio::spawn(async move {
     gnawtreewriter::mcp::mcp_server::serve_with_shutdown(listener, Some("secret".into()), async {
       let _ = rx.await;
     }).await.unwrap();
   });
   ```
3. Wait for server readiness, then `POST` JSON-RPC payloads with `reqwest::Client`.
4. Shutdown server: `let _ = tx.send(()); server.await?;`

Run the test suite:
```bash
cargo test --features mcp
```

---

## Extending the MCP server — adding a new tool

To add a tool:
1. Define the tool's semantics and its input schema (JSON Schema).
2. Implement a handler function (e.g., `fn handle_my_tool(args: &Value) -> Result<Value>`).
3. Update the `rpc_handler` match to call your handler when `req.method == "tools/call"` and `name == "my_tool"`.
4. Add the tool’s description to the `tools/list` result (so agents can discover it).
5. Add unit tests for the handler and integration tests exercising the JSON-RPC path.
6. Document the tool in `docs/MCP.md` and in the README (if user-facing).

Example (pseudo-Rust):
```rust
// 1. implement handler
fn handle_reverse(args: &Value) -> Result<Value> {
    let s = args.get("text").and_then(|v| v.as_str()).ok_or_else(|| anyhow!("missing 'text'"))?;
    Ok(json!({ "content": [{ "type":"text", "text": s.chars().rev().collect::<String>() }] }))
}

// 2. wire into rpc_handler
match req.method.as_str() {
  "tools/call" => {
     let params = req.params.unwrap_or_default();
     match params.get("name").and_then(Value::as_str) {
       Some("reverse") => handle_reverse(&params.get("arguments").cloned().unwrap_or_default()) ...
```

---

## CLI help improvements

We recommend improving the `clap` doc comments to make CLI help self-explanatory:

- Add doc comments on `Commands::Mcp` and `McpSubcommands::Serve` to be shown in `gnawtreewriter --help`.
- Example doc snippet to put above the enum/fields in `src/cli.rs`:
```rust
/// Manage the built-in MCP server and daemon (experimental).
///
/// Examples:
///   gnawtreewriter mcp serve --addr 127.0.0.1:8080 --token secret
enum McpSubcommands {
    /// Start the MCP server.
///
///   --addr: address to bind (default 127.0.0.1:8080)
///   --token: optional bearer token for basic auth
    Serve {
        #[arg(long, default_value = "127.0.0.1:8080")]
        addr: String,
        #[arg(long)]
        token: Option<String>,
    }
}
```

Keeping help text concise and providing examples makes it significantly easier for users to run and test the MCP server themselves.

---

## Security & production recommendations

- Use TLS (terminate SSL at proxy e.g., NGINX/Caddy) before exposing the MCP endpoint to untrusted networks.
- Use a long, securely stored Bearer token or integrate with a proper auth layer if exposing beyond local usage.
- Implement rate limiting / request size limits if exposed publicly.
- Put server behind a reverse proxy (rate limiting, auth, TLS) for production.
- Log responsibly — avoid printing secrets in logs.

---

## Troubleshooting

- `401 Unauthorized`: check `Authorization` header and that token matches `--token` or `MCP_TOKEN`.
- `Invalid JSON-RPC payload`: ensure `Content-Type: application/json` and a valid JSON structure.
- Server not binding: ensure the chosen `--addr` is free and not blocked by firewall.

---

## Future work and considerations

- Add TLS support directly (optional).
- Expand tool suite (format, lint, refine batch semantics).
- Improve tests to include stress tests, concurrency tests, and edge cases.
- Add better schema validation for tool inputs (JSON Schema validation).
- Add discovery endpoints or richer capabilities negotiation.

---

If you want, I can:
- Add additional example clients (Python / Node) showing how to call MCP programmatically.
- Add example integration tests demonstrating `batch` with rollback behavior.
- Update CLI help strings in source code (I can open a PR with the changes).

If you'd like me to proceed implementing any of those follow-ups (or make the docs more extensive / translated), tell me which one and I'll proceed.
# Examples — MCP Clients

This directory contains small example clients that demonstrate how to call the MCP (Model Context Protocol) server implemented in GnawTreeWriter.

- Python example: `examples/python_mcp_client.py`
- Rust example: `examples/mcp_client.rs`

These are intentionally minimal and intended for local testing and experimentation. They are NOT production-ready clients — they demonstrate the expected JSON-RPC calls and simple error handling.

---

## Prerequisites

- MCP server running:
  - Example: `gnawtreewriter mcp serve --addr 127.0.0.1:8080 --token secret`
  - Or set the environment variable `MCP_TOKEN=secret` before starting the server.
- Python example: Python 3 and the `requests` package
  - Install: `pip install requests`
- Rust example: build with the crate's features; the example uses `reqwest` and `tokio`
  - Run via Cargo: `cargo run --features mcp --example mcp_client -- <args>`

---

## Quick JSON-RPC recap

- Request shape:
```json
{ "jsonrpc": "2.0", "method": "<method>", "id": 1, "params": { ... } }
```
- Response shape:
  - Either a `result` object (success)
  - Or an `error` object (protocol-level error)
- Note (MVP behavior): Some tool errors are returned inside `result` (e.g., `result.isError = true` and `result.content` contains a human-readable message). Clients should:
  1. Check for top-level `error` (JSON-RPC protocol errors).
  2. If no protocol error, inspect `result` and check `result.isError` where applicable.

---

## Python example

File: `examples/python_mcp_client.py`

Usage:
```bash
# List tools
python examples/python_mcp_client.py --url http://127.0.0.1:8080/ --token secret list

# Initialize (handshake)
python examples/python_mcp_client.py --url http://127.0.0.1:8080/ --token secret init

# Call analyze for a file
python examples/python_mcp_client.py --url http://127.0.0.1:8080/ --token secret analyze examples/foo.py

# Generic tool call with JSON-encoded parameters
python examples/python_mcp_client.py --url http://127.0.0.1:8080/ --token secret call analyze '{"file_path":"examples/foo.py"}'
```

Notes:
- The script accepts `--url` and `--token`. If `--token` is omitted, it checks `MCP_TOKEN` environment variable.
- It retries `initialize` a few times to wait for server readiness.

---

## Rust example

File: `examples/mcp_client.rs`

Build/run:
```bash
# Run 'initialize' and 'tools/list'
cargo run --features mcp --example mcp_client -- --url http://127.0.0.1:8080/ --token secret list

# Run 'analyze'
cargo run --features mcp --example mcp_client -- --token secret analyze examples/foo.py
```

Notes:
- The example is a small CLI that accepts `--url` and `--token` and a command (`init`, `list`, `analyze`, `call`).
- You can also set `MCP_URL` and `MCP_TOKEN` environment variables instead of passing flags.

---

## What to expect in outputs

- `initialize` returns server info and capabilities.
- `tools/list` returns an object with a `tools` array (each entry has `name`, `title`, `inputSchema`, ...).
- `tools/call`:
  - On success: `result` contains `content` (presentation) and optionally `structuredContent`.
  - On tool-level failure (MVP): `result` will contain `"isError": true` and `content` explaining the failure.
  - For protocol-level errors (invalid JSON, unknown method), you will get JSON-RPC `error`.

---

## Troubleshooting

- `401 Unauthorized`: check `Authorization: Bearer <token>` header or `MCP_TOKEN` value.
- `connection refused`: ensure server is running and bound to the address you use.
- If a tool returns `isError`, inspect `result.content` for human-readable diagnostics.

---

## Extending examples

These examples are intentionally simple. If you want:
- I can add a small Node.js client example.
- Add more command-line options (timeout, retries, verbose logging).
- Add example for `tools/call` with `batch`/`undo` demonstration and parsing of `structuredContent`.

---

## References

- MCP docs: `docs/MCP.md` (detailed server doc & examples)
- Server implementation: `src/mcp/mod.rs`
- Integration tests: `tests/mcp_integration.rs` (demonstrates the same patterns used by these examples)

---

If you want, I can:
- Add these examples to the repo (I'll commit them).
- Add a short CI job to run the examples against a short-lived server for smoke testing.

Which of these would you like me to do next?
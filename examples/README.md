# Examples — MCP Clients

This directory contains minimal example clients that demonstrate how to call the MCP (Model Context Protocol) server implemented in GnawTreeWriter.

- Python example: `examples/python_mcp_client.py`
- Rust example: `examples/mcp_client.rs`
- Node example: `examples/node_mcp_client.js`

These are intentionally small and meant for local testing and experimentation. They demonstrate the expected JSON-RPC calls and simple error handling but are not production-ready clients.

---

## Prerequisites

- A running MCP server (see below for quick helpers to start it locally)
- Python 3 and `requests` (for the python example): `pip install requests`
- Node 18+ (for the node example)
- Rust toolchain (stable) to build and run the Rust example

---

## Quick usage

### Rust example

Build & run:
```bash
# List tools
cargo run --features mcp --example mcp_client -- --url http://127.0.0.1:8080/ --token secret list

# Initialize (handshake)
cargo run --features mcp --example mcp_client -- --url http://127.0.0.1:8080/ --token secret init

# Analyze a file
cargo run --features mcp --example mcp_client -- --token secret analyze examples/example.rs
```

Notes:
- The example accepts `--url` and `--token`. If omitted, it uses `MCP_URL` / `MCP_TOKEN` environment variables (and defaults to `http://127.0.0.1:8080/` and `secret`).

---

## Quick scripts & editor tasks (recommended)

To make local testing and GUI integration easy we provide helper scripts and a VS Code `tasks.json` that you can use from any terminal or editor.

Files added:
- `scripts/mcp-serve.sh` — start the MCP server (foreground or background). Usage:
  - `./scripts/mcp-serve.sh` (start background server on `127.0.0.1:8080`, token `secret`)
  - `./scripts/mcp-serve.sh --addr 127.0.0.1:9000 --token secret`
  - `./scripts/mcp-serve.sh --addr 127.0.0.1:0` (ephemeral port; script will attempt to parse the port from logs)
  - `./scripts/mcp-serve.sh --foreground` (run in foreground for debugging)
- `scripts/mcp-stop.sh` — stop the server started with `mcp-serve.sh` (reads PID file)
- `scripts/mcp-client.sh` — convenient wrapper for the Rust client example:
  - `./scripts/mcp-client.sh list`
  - `./scripts/mcp-client.sh init`
  - `./scripts/mcp-client.sh analyze examples/example.rs`
  - Supports `--url` and `--token` and a `--release` flag for release builds.
- `scripts/test-mcp.sh` — test orchestration: starts the server, waits for readiness, runs `list`, `init`, and `analyze` on a temporary test file, and then stops the server. Useful for quick local verification and for running tests in CI. Example:
  - `./scripts/test-mcp.sh` (default addr `127.0.0.1:8080`, token `testtoken`)
  - `./scripts/test-mcp.sh --addr 127.0.0.1:8081 --token mytoken`
  - Use `./scripts/test-mcp.sh --keep` to leave the server running for interactive debugging.

### VS Code integration

We include `.vscode/tasks.json` with convenient tasks. To use them:
1. Open the Command Palette (Ctrl+Shift+P) → `Tasks: Run Task`.
2. Select one of:
   - `MCP: Start server (background)` — starts the server using `scripts/mcp-serve.sh`.
   - `MCP: Stop server` — stops the server via `scripts/mcp-stop.sh`.
   - `MCP: Test (list/init/analyze)` — runs `scripts/test-mcp.sh`.
   - `MCP: Client list` / `MCP: Client init` / `MCP: Client analyze (prompt)` — run the client convenience commands.

This lets you start/stop the server and run client commands from the GUI.

### Zed, Gemini CLI and other environments

- The scripts are plain POSIX shell scripts and work in any environment that has a POSIX-compatible shell (Linux, macOS, WSL on Windows).
- In editors without a built-in `tasks.json` concept, create a custom task or a launcher that calls the equivalent script:
  - Add a "Run command" that executes `./scripts/mcp-serve.sh` to start the server
  - Add another command to run `./scripts/mcp-stop.sh` to stop it
  - Run client commands via `./scripts/mcp-client.sh <cmd>`
- For "Gemini CLI" or other terminals: just run the scripts directly in the integrated terminal or use them in your automation.

---

## Developer workflow (suggested)

1. Start server (in a terminal or via your editor's task)
   - `./scripts/mcp-serve.sh`
2. In another terminal: run client checks
   - `./scripts/mcp-client.sh list`
   - `./scripts/mcp-client.sh init`
   - `./scripts/mcp-client.sh analyze examples/example.rs`
3. When done: `./scripts/mcp-stop.sh`
4. Quick full verify: `./scripts/test-mcp.sh` (this will start/stop the server for you)

This workflow is what the CI uses to validate the MCP examples.

---

## CI

- The repository's MCP examples workflow was updated to run the Rust `mcp_client` example in CI so changes to the example or server are exercised automatically.
- If you add new MCP-related examples or change protocol behavior, add or update tests under `tests/` and ensure the CI workflow covers the change.

---

## Troubleshooting

- `401 Unauthorized` — ensure the client is using the same token as the server (`--token` or `MCP_TOKEN`).
- `connection refused` — server is not running or is bound to different port. Check logs (`.mcp-server.log` or the log path you specified).
- If you get a compile or feature error about `mcp`, ensure you build with `--features mcp` (scripts and tasks already pass this flag).
- For Windows users: scripts are written for POSIX shells; use WSL or adapt the scripts to PowerShell if desired.

---

## Contributing

- Add new examples to this directory and ensure `scripts/test-mcp.sh` or the CI workflow exercises them.
- Keep `examples/*` small and focused; tests should be deterministic and not rely on external services.

---

If you'd like, I can:
- Add short editor-specific templates (Zed or other IDEs) for one-click "Start server / Run test" actions,
- Add a small Makefile with targets `make mcp-start`, `make mcp-stop`, `make mcp-test`,
- Or extend the CI to run a broader matrix of example-based tests.

Tell me which of those you'd like next and I can prepare them.
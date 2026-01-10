# MCP Server — Model Context Protocol

This document describes the MCP (Model Context Protocol) implementation in GnawTreeWriter. It covers both **Stdio** (recommended for extensions) and **HTTP** transport layers, supported tools, and integration with AI clients like Gemini CLI, Zed, and Claude Desktop.

---

## Transport Layers

GnawTreeWriter supports two ways to communicate via MCP:

### 1. Stdio (Standard Input/Output)
**Recommended for local integrations.** The AI client starts the `gnawtreewriter` process directly and communicates over a pipe.

- **Fast & Reliable:** No network overhead or port conflicts.
- **Secure:** Communication is local to the machine.
- **Automatic Lifecycle:** The server stops when the client (e.g., Gemini CLI) stops.

**Command:**
```bash
gnawtreewriter mcp stdio
```

### 2. HTTP (JSON-RPC over HTTP)
Useful for debugging or remote scenarios.

**Command:**
```bash
gnawtreewriter mcp serve --addr 127.0.0.1:8080 --token secret
```

---

## Supported Tools

The MCP server exposes the core "Gnaw" logic as tools that AI agents can use:

| Tool | Purpose | Key Arguments |
| :--- | :--- | :--- |
| `analyze` | Get full AST structure | `file_path` |
| `list_nodes` | Flat list of edit targets | `file_path`, `filter_type`, `max_depth`, `include_all` |
| `search_nodes` | Find nodes by text or name | `file_path`, `pattern` |

### Pro-tip for Large Files
- **Shallow Exploration:** Use `list_nodes` with `max_depth: 1` to see only top-level classes and functions. Important nodes now include a `name` field (e.g., function names) for easy identification.
- **Noise Reduction:** By default, `list_nodes` filters out purely structural nodes (brackets, commas). Use `include_all: true` if you need the full AST.
- **Find by Name:** Use `search_nodes` with a function or class name to find its exact path without listing the whole file. Results are sorted by specificity (deepest matches first).

### Success vs Error
- **Protocol Error:** Returned as JSON-RPC error (e.g., invalid JSON, missing required param).
- **Tool Error:** Returned with `isError: true` in the result (e.g., file not found, syntax error in new code).

---

## Gemini CLI Integration

You can use GnawTreeWriter as a native extension in the [Gemini CLI](https://google-gemini.github.io/gemini-cli/).

### Installation
1. Navigate to the project root.
2. Link the provided extension directory:
   ```bash
   gemini extensions link ./gemini-extension
   ```
3. Restart Gemini CLI.

### Usage Examples
- "Analyze `src/main.rs` and list all functions."
- "Rename the variable `x` to `counter` in `app.py` using GnawTreeWriter."
- "Add a comment to the top of `lib.rs` saying 'Version 0.6.2'."

---

## Zed Integration

To use GnawTreeWriter in [Zed](https://zed.dev/), add it to your `settings.json`:

```json
{
  "context_servers": {
    "gnawtreewriter": {
      "command": {
        "program": "/path/to/gnawtreewriter",
        "args": ["mcp", "stdio"]
      }
    }
  }
}
```

---

## Development

### Rebuilding
When updating MCP logic, rebuild with the `mcp` feature:
```bash
cargo build --release --features mcp
```

### Debugging
If you encounter connection issues, you can run the server manually and pipe JSON to it:
```bash
echo '{"jsonrpc":"2.0","method":"tools/list","id":1}' | ./target/release/gnawtreewriter mcp stdio
```

All debug logs and errors are sent to **stderr**, keeping **stdout** clean for JSON-RPC messages.

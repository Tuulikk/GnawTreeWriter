# GnawTreeWriter MCP — VSCode Extension

Registers [GnawTreeWriter](https://github.com/Tuulikk/GnawTreeWriter) as an MCP
server for Copilot Chat, enabling AST-based code editing via `mcp_gnawtreewrite_*`
tools in every VSCode window.

No configuration needed if `gnawtreewriter` is in PATH (e.g. `cargo install`).
Supports a `gnawtreewriter.binaryPath` setting for custom locations.

## Install

### Quick (open folder)

1. Open `extensions/vscode/` in VSCode as a folder.
2. Press F5 → Extension Development Host opens.
3. Verify: Command Palette → `MCP: List Servers` → `GnawTreeWriter` = Connected.

### Permanent (install from source)

```bash
# Prerequisites
npm install -g @vscode/vsce

# Package
cd extensions/vscode
vsce package

# Install .vsix
code --install-extension gnawtreewriter-mcp-0.1.0.vsix
```

### For end users (once published)

Search **GnawTreeWriter MCP** in the VSCode Extension Marketplace.

## Configuration

| Setting | Purpose |
|---|---|
| `gnawtreewriter.binaryPath` | Custom path to `gnawtreewriter` binary. Supports `${userHome}` and `${env:VAR}`. |

Leave empty to auto-detect (checks `~/.cargo/bin/`, `~/.local/bin/`, PATH).

## What it does

- Starts `gnawtreewriter mcp stdio` as a background process
- Exposes tools: `analyze`, `list_nodes`, `edit_node`, `semantic_edit`, `batch`, `undo`, ...
- Works in **every** workspace automatically — no `.vscode/mcp.json` needed
- Replaces the old `mcp.json` + `settings.json` user-level MCP registration

## License

MPL-2.0

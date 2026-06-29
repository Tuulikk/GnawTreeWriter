// GnawTreeWriter — MCP server provider for Copilot
// Mönster matchar gnaw-checkpoint och gnaw-dokubase (som fungerar).

const vscode = require("vscode");
const path = require("path");
const fs = require("fs");

// ── Binary resolution ─────────────────────────────────────────────────
// Ordning: (1) config → (2) ~/.cargo/bin/ → (3) PATH (fallback)

function findBinary() {
  const config = vscode.workspace.getConfiguration("gnawtreewriter");
  const rawPath = config.get("binaryPath", "");

  if (rawPath) {
    // Expand ${userHome} and ${env:VAR} placeholders (Dokubase pattern)
    let expanded = rawPath
      .replace(/\$\{userHome\}/g, process.env.HOME || ".")
      .replace(/\$\{env:([^}]+)\}/g, (_, v) => process.env[v] || "");
    if (expanded.startsWith("~")) {
      expanded = path.join(process.env.HOME || ".", expanded.slice(1));
    }
    if (fs.existsSync(expanded)) {
      return expanded;
    }
    console.log(`GnawTreeWriter MCP: configured binary not found: ${expanded}`);
  }

  // Check common install locations
  const candidates = [
    path.join(process.env.HOME || "", ".cargo", "bin", "gnawtreewriter"),
    path.join(process.env.HOME || "", ".local", "bin", "gnawtreewriter"),
    "/usr/local/bin/gnawtreewriter",
    "/usr/bin/gnawtreewriter",
  ];
  for (const candidate of candidates) {
    if (fs.existsSync(candidate)) {
      return candidate;
    }
  }

  // Final fallback: let Node.js resolve from PATH
  return "gnawtreewriter";
}

// ── MCP Provider ──────────────────────────────────────────────────────

class GnawTreeWriterMcpProvider {
  constructor() {
    /** @type {vscode.EventEmitter<void>} */
    this._changeEmitter = new vscode.EventEmitter();
    this._warned = false;
  }

  get onDidChangeMcpServerDefinitions() {
    return this._changeEmitter.event;
  }

  /**
   * @param {vscode.CancellationToken} token
   * @returns {vscode.McpStdioServerDefinition[]}
   */
  provideMcpServerDefinitions(token) {
    const bin = findBinary();

    // Warn once if the resolved binary doesn't exist (PATH fallback may still work)
    if (!fs.existsSync(bin)) {
      if (!this._warned) {
        this._warned = true;
        vscode.window.showWarningMessage(
          "GnawTreeWriter: binary not found on disk. Install with: " +
          "`cargo install --path /path/to/GnawTreeWriter` or set " +
          "`gnawtreewriter.binaryPath` in settings."
        );
      }
    }

    const label = "GnawTreeWriter";
    const def = new vscode.McpStdioServerDefinition(
      label,
      bin,
      ["mcp", "stdio"],
    );

    return [def];
  }
}

// ── Extension lifecycle ───────────────────────────────────────────────

/**
 * @param {vscode.ExtensionContext} context
 */
function activate(context) {
  const provider = new GnawTreeWriterMcpProvider();

  // ID måste matcha package.json contribution exakt.
  context.subscriptions.push(
    vscode.lm.registerMcpServerDefinitionProvider(
      "gnaw-software.gnawtreewriter.mcp",
      provider,
    ),
  );

  console.log("GnawTreeWriter MCP provider activated");
}

function deactivate() {}

module.exports = { activate, deactivate };

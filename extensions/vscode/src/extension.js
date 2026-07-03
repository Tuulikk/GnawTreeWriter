// GnawTreeWriter — MCP server provider for Copilot
// Mönster matchar gnaw-checkpoint och gnaw-dokubase (som fungerar).

const vscode = require("vscode");
const path = require("path");
const fs = require("fs");

// ── Binary resolution ─────────────────────────────────────────────────
// Ordning: (1) config → (2) ~/.cargo/bin/ → (3) PATH $PATH → (4) which

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
      console.log(`GnawTreeWriter MCP: found at ${candidate}`);
      return candidate;
    }
  }

  // Scan PATH directories (extension host may have different PATH than shell)
  const pathDirs = (process.env.PATH || "").split(":");
  for (const dir of pathDirs) {
    const candidate = path.join(dir, "gnawtreewriter");
    try {
      if (fs.existsSync(candidate)) {
        console.log(`GnawTreeWriter MCP: found in PATH at ${candidate}`);
        return candidate;
      }
    } catch (e) {
      // skip invalid paths
    }
  }

  // Final fallback — warn user
  return null;
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

    if (!bin) {
      if (!this._warned) {
        this._warned = true;
        vscode.window.showWarningMessage(
          "GnawTreeWriter: binary not found. Install with: " +
          "`cargo install --path /path/to/GnawTreeWriter` or set " +
          "`gnawtreewriter.binaryPath` in settings."
        );
      }
      return [];
    }

    const label = "GnawTreeWriter";
    const def = new vscode.McpStdioServerDefinition(
      label,
      bin,
      ["mcp", "stdio"],
    );

    // Set cwd — VSCode may need it for PATH-based resolution (Checkpoint pattern)
    const folders = vscode.workspace.workspaceFolders;
    if (folders && folders.length > 0) {
      def.cwd = folders[0].uri;
    }

    console.log(`GnawTreeWriter MCP: definition provided — bin=${bin}, cwd=${def.cwd ? def.cwd.fsPath : '(none)'}`);
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

  // Fire change event after registration so VSCode re-queries immediately
  provider._changeEmitter.fire();

  console.log("GnawTreeWriter MCP provider activated");
}

function deactivate() {}

module.exports = { activate, deactivate };

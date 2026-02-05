# GnawTreeWriter

**AI-native tree-based code editor** - Edit code files based on AST structure levels with surgical precision, avoiding the common pitfalls of LLM-generated code.

## 🚀 Version 0.9.1: The Surgical Update

We've just released v0.9.1, a major refinement that brings **surgical inline precision** to your editing workflow. This update bridges the gap between high-level structural editing and the need for microscopic changes within a single line.

### 🎯 Surgical Inline Editing
No more replacing entire lines just to change one variable! GnawTreeWriter v0.9.1 introduces column-aware editing (starting with Rust), allowing you to:
- **Edit a single parameter** in a dense function call without disturbing surrounding code.
- **Rename a variable** while preserving trailing comments on the same line.
- **Update types or values** with character-level accuracy within the AST.

### 💡 Pedagogical Validation
The **Duplex Loop** is now more than just a gatekeeper; it's a teacher. If an edit fails syntax validation, you get **language-specific tips** to help you or your agent fix the issue:
- **Rust**: Detects missing semicolons `;` or unbalanced braces `{}`.
- **QML**: Ensures properties have colons `:` and objects are correctly closed.
- **Python**: Checks indentation and colon placement.

---

## 🧠 GnawSense: AI-Powered Navigation & Action
Powered by **ModernBERT**, GnawSense transforms GnawTreeWriter from a precision editor into a **semantically-aware cognitive infrastructure**.

- **Semantic Search (`sense`)**: Search for logic by description (e.g., "how is backup handled?"). Includes *Satelite View* for project-wide discovery and *Zoom View* for file-specific focus.
- **Semantic Insertion (`sense-insert`)**: Insert code near a landmark without knowing its path. Just describe the anchor point (e.g., "after the login function") and let GnawSense find the correct AST position.
- **MCP Native**: All GnawSense features are exposed via the MCP server, enabling AI agents to navigate and edit your project autonomously and safely.

### 🏗️ Structural Scaffolding
Stop starting with empty files. Use the `scaffold` command to create new files with a predefined AST structure.
```bash
# Create a new Rust module with a struct and start function
gnawtreewriter scaffold src/network.rs --schema "rust:mod(name:server, struct:Config, fn:start)"
```
This ensures your files are syntaktically correct from the very first byte.

### 🛡️ TCARV Methodology Integration
GnawTreeWriter officially supports and enforces the **TCARV 1.0 (Text-Centric Architecture & Recursive Verification)** methodology.
- **Logic-First**: Define logic in text before writing code.
- **Anti-Lobotomy Policy**: Prevents agents from deleting complex code to fix build errors.
- **Git Surgery**: Bans "nuclear" rollbacks; encourages precise recovery from history.

---

### 📦 Installation Options

```bash
# Core only
cargo install --path .

# Full power (Recommended: includes GnawSense and MCP)
cargo install --path . --features modernbert,mcp
```

---

## 🛡️ The Structural Guardian

GnawTreeWriter isn't just a text editor; it's a **Structural Guardian** for your codebase. It monitors the "entropy" of your code during every edit, ensuring that AI agents (or human operators) don't accidentally perform a "lobotomy" on your logic.

- **Integrity Auditing**: Every edit is scored for structural loss. If a massive amount of logic or documentation is removed, The Guardian blocks the change.
- **The Duplex Loop**: GnawTreeWriter validates proposed changes against the AST *before* they touch your disk. If it's not valid syntax, it won't be applied.

## 📓 ALF: Agentic Logging Framework

To solve the problem of "Agent Amnesia," we built **ALF**. It's a structural journal that links high-level intent with low-level code changes.

- **Traceable Intent**: Why was this function changed? ALF knows.
- **Transaction Linking**: Every journal entry is tied to a specific `TransactionID` in the history.
- **Ecosystem Ready**: Designed to share knowledge with other tools like **GnawMimir**, creating a unified cognitive workspace.

## 📊 Engineering Case Studies

### Case 1: Preventing "Agent Lobotomy" (The Guardian)
*Scenario: An AI agent tries to "fix" a bug by deleting 40 lines of error handling logic.*
| Layer | Details |
| :--- | :--- |
| **Old State** | Complex function with nested `match` and `Result` handling. |
| **Agent Proposal** | Replacing the logic with a simple `unwrap()`. |
| **Guardian Action** | 🛑 **BLOCK**: Structural integrity check failed. |
| **Reasoning** | Significant complexity loss detected. Logic markers dropped from 12 to 1. |

### Case 2: Surgical Precision (v0.9.1 Update)
*Scenario: Changing a single parameter in a complex Rust function call.*
| Layer | Details |
| :--- | :--- |
| **Old Line** | `let res = process_data(config, true, timeout, "standard");` |
| **New Precision** | `gnawtreewriter edit file.rs "1.5.2" 'false'` |
| **Result** | `let res = process_data(config, false, timeout, "standard");` |
| **Benefit** | The rest of the line (config, timeout, etc.) remains untouched. |

---

## Key Features

- **AST-Level Precision**: Work at tree level, never worry about brackets again.
- **GnawSense**: Semantic navigation and editing via local AI.
- **Time Travel**: Project-wide restoration to any timestamp.
- **Atomic Multi-File Operations**: Coordinated edits with automatic rollback.
- **Multi-Language Support**: 17+ programming languages.

## Documentation

- **[TCARV Methodology](TCARV_1_0.md)** - The core process for AI development.
- **[AGENTS.md](AGENTS.md)** - Guidelines for AI agents.
- **[MCP.md](docs/MCP.md)** - Detailed Model Context Protocol documentation.
- **[ROADMAP.md](ROADMAP.md)** - Our journey towards v1.0.

## License

Mozilla Public License 2.0. See [LICENSE](LICENSE) for details.

---

*Built with ❤️ and multi-agent collaboration.*

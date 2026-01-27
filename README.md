# GnawTreeWriter

**AI-native tree-based code editor** - Edit code files based on AST structure levels, avoiding bracket issues and structural problems from LLM code generation.

## 🚀 Version 0.7.1: The Semantic Update

We've just released v0.7.1, a milestone that transforms GnawTreeWriter from a precision editor into a **semantically-aware cognitive infrastructure**. This update bridges the gap between vague human/AI intent and surgical AST precision.

### 🧠 GnawSense: AI-Powered Navigation & Action
Powered by **ModernBERT**, GnawSense allows you to interact with your codebase using meaning, not just exact strings or line numbers.

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
GnawTreeWriter now officially supports and enforces the **TCARV 1.0 (Text-Centric Architecture & Recursive Verification)** methodology.
- **Logic-First**: Define logic in text before writing code.
- **Anti-Lobotomy Policy**: Prevents agents from deleting complex code to fix build errors.
- **Git Surgery**: Bans "nuclear" rollbacks; encourages precise recovery from history.

---

### 📦 Installation Options

```bash
# Core only
cargo install --path .

# With AI features (GnawSense / ModernBERT)
cargo install --path . --features modernbert

# With MCP server support
cargo install --path . --features mcp

# Full power (Recommended)
cargo install --path . --features modernbert,mcp
```

---

### 📚 Quick Start: The v0.7.1 Workflow

```bash
# 1. Setup the AI model (ModernBERT)
gnawtreewriter ai setup --model modernbert

# 2. Scaffold a new project component
gnawtreewriter scaffold src/auth.rs --schema "rust:mod(name:security, fn:validate_user)"

# 3. Use GnawSense to find where to add logic
gnawtreewriter sense "logic for user validation" src/auth.rs

# 4. Semantically insert code (Preview first!)
gnawtreewriter sense-insert src/auth.rs "fn validate_user" "println!(\"Validating...\");" --preview

# 5. Apply the change
gnawtreewriter sense-insert src/auth.rs "fn validate_user" "println!(\"Validating...\");"
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

### 📊 Engineering Evidence (Sample Report)

| Layer | Details |
| :--- | :--- |
| **Intent** | Strategic Decision: Migrating to async handlers for scalability. |
| **Structure** | Node `@fn:old_system` in `src/network.rs` |
| **Actor** | @writer (Autonomous Architect) |
| **Status** | ✅ Verified via TCARV |

---

## Key Features

- **AST-Level Precision**: Work at tree level, never worry about brackets again.
- **GnawSense**: Semantic navigation and editing via local AI.
- **Time Travel**: Project-wide restoration to any timestamp.
- **Atomic Multi-File Operations**: Coordinated edits with automatic rollback.
- **Multi-Language Support**: 17+ programming languages.
- **AI-Native Architecture**: Built specifically for LLM agents and autonomous workflows.

## Documentation

- **[TCARV Methodology](TCARV_1_0.md)** - The core process for AI development.
- **[AGENTS.md](AGENTS.md)** - Guidelines for AI agents (Claude, Gemini, etc.).
- **[MCP.md](docs/MCP.md)** - Detailed Model Context Protocol documentation.
- **[ROADMAP.md](ROADMAP.md)** - Our journey towards v1.0.

## License

Mozilla Public License 2.0. See [LICENSE](LICENSE) for details.

---

*Built with ❤️ and multi-agent collaboration.*
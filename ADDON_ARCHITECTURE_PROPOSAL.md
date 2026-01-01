# Add-on Architecture: Strategic Decision

## Executive Summary

GnawTreeWriter needs a clear add-on architecture to support extensibility while maintaining core focus and quality. This document proposes a monorepo with Git submodules as the recommended approach, with detailed analysis of alternatives and a recommended implementation roadmap.

---

## Proposed Architecture: Monorepo with Git Submodules

### Repository Structure

```
gnawtreewriter/                    # Main repository
├── src/
│   ├── core/              # Core modules (batch, time travel, sessions, tags)
│   ├── parsers/           # TreeSitter and generic parsers for all languages
│   └── cli.rs             # Main CLI entry point
├── add-ons/              # Add-on submodules (Git submodules)
│   ├── lsp/               # LSP integration (stable)
│   ├── mcp/               # MCP daemon (stable)
│   ├── ui/                # Visualization UI (experimental)
│   ├── refactor/          # Advanced refactoring operations (stable)
│   └── vision/           # AI analysis and planning tools (experimental)
└── docs/                   # Documentation
```

**Key Principle:** GnawTreeWriter core provides foundational capabilities; add-ons extend functionality for specific use cases (LSP, MCP, visualization, etc.).

---

## Core vs Add-on Boundaries

### Core (Must Exist in GnawTreeWriter)

**Definition:** Features that are essential, always-on, and define the tool's purpose.

**Core Features:**
- ✅ Multi-edit (Batch operations)
- ✅ Time travel (Restore, Sessions, Undo/Redo)
- ✅ Named references (Tags)
- ✅ Implicit sessions (auto-start)
- ✅ Validation-first approach
- ✅ Universal parsers (Python, Rust, TS, JS, QML, Go, etc.)
- ✅ CLI framework (analyze, edit, insert, delete, list, etc.)

**Core Versioning:** Core (GnawTreeWriter) = v0.4.0, Add-ons follow their own versioning.

### Add-ons (Optional, Extensible)

**Definition:** Features that enhance or extend core capabilities for specific use cases, domains, or workflows.

**Add-on Categories:**

**1. Language Server Protocol (LSP)**
- Semantic analysis
- Symbol resolution (go-to-definition)
- Hover information
- Real-time error highlighting
- Auto-completion
- Cross-language support

**2. Model Context Protocol (MCP)**
- AI agent coordination and orchestration
- Tool calling and execution
- Resource management
- Multi-agent workflows

**3. Visualization**
- Interactive time-travel UI
- Visual diff viewer (syntax-highlighted)
- Timeline explorer
- Drag-and-drop editing

**4. Advanced Refactoring**
- Extract function/method
- Rename symbol (cross-file)
- Change signature
- Inline variable/function

**5. Analysis Tools**
- Static code analysis
- Complexity metrics
- Code smell detection
- Performance profiling

**6. Utilities**
- Language-specific tools
- Formatter integrations
- CI/CD hooks

**Stability Classification:**
- **Stable:** LSP, MCP, Refactor, CLI utilities (lint, diff, analyze)
- **Experimental:** Visualization, Profiling, Language-specific analysis
- **Community:** User-contributed scripts, templates

---

## Alternative Analysis

### Alternative 1: Monorepo with Git Submodules ✅ [RECOMMENDED]

**Pros:**
- Clear ownership (GnawTreeWriter = core, add-ons = extensions)
- Easy versioning (core and each add-on independent)
- Professional appearance (tuulikk/gnawtreewriter organization)
- Community-friendly (easy to fork and contribute to specific add-ons)
- Scalable (parallel development on different add-ons)
- Open-source best practices (follows Kubernetes, VS Code, etc.)

**Cons:**
- Git history can become complex across submodules
- Dependency management requires coordination
- More complex CI/CD setup

**Verdict:** Best balance of professionalism and community-friendliness.

### Alternative 2: Separate GitHub Repositories

**Pros:**
- Independent versioning
- Clear product identity
- Easier to sell specific add-ons as premium products
- Simplified dependency management

**Cons:**
- No unified "GnawTreeWriter" presence
- Harder to coordinate development
- Duplicate code/effort
- Lost "community cohesion"

**Verdict:** Good for monetization, bad for community cohesion.

### Alternative 3: Current with `extras/` Directory

**Pros:**
- Simple (one repo, one history)
- Easy to navigate

**Cons:**
- Unclear boundaries between core and extras
- Difficult to version add-ons independently
- Hard to release add-ons as standalone products
- Vision/analysis in `extras/` feels like "core" but isn't essential

**Verdict:** Good for now, but doesn't scale to add-ons ecosystem.

---

## Recommended Implementation Strategy

### Phase 1: Core Focus (v0.4.0 - v0.5.0)

**Goal:** Stabilize and perfect core features before major add-on work.

**Tasks:**
- [x] Batch operations (Multi-edit) - ✅ COMPLETE
- [x] Comprehensive testing (19/19 passing)
- [x] Documentation (README, CHANGELOG, ROADMAP)
- [ ] LSP integration (as first add-on)

### Phase 2: LSP Add-on (v0.4.1)

**Goal:** Provide semantic intelligence and IDE-like features.

**Implementation:**
```bash
# Create add-on repository
mkdir -p add-ons/lsp
cd add-ons/lsp
cargo new gnawtreewriter-lsp

# Create plugin interface for core
# (Define how LSP add-on communicates with GnawTreeWriter core)
```

**Key Features:**
- Hover information for nodes
- Go-to-definition for symbols
- Cross-file symbol resolution
- Real-time error highlighting

### Phase 3: MCP Add-on (v0.4.2)

**Goal:** Enable AI agent coordination and orchestration.

**Implementation:**
```bash
# Create add-on repository
mkdir -p add-ons/mcp
cd add-ons/mcp
cargo new gnawtreewriter-mcp
```

**Key Features:**
- Agent session management
- Tool registration and discovery
- Resource allocation
- Multi-agent workflow support

### Phase 4: Visualization Add-on (v0.5.0)

**Goal:** Provide visual interface for time travel and code analysis.

**Implementation:**
- Start as experimental in `extras/` or standalone add-on
- If successful, graduate to main submodules

---

## Integration Strategy

### Add-on to Core Communication

**Option A: JSON API (Simple)**
```rust
// Core provides simple read/parse API
// Add-ons call core functions to analyze and modify trees

impl GnawTreeWriter {
    pub fn read_ast(&self) -> &TreeNode;
    pub fn modify_ast(&mut self, operations: &[Operation]);
}
```

**Option B: Plugin System (Extensible)**
```rust
// Core defines plugin trait
// Add-ons implement this trait to hook into core events

pub trait GnawTreePlugin {
    fn on_before_edit(&self, operation: &Operation);
    fn on_after_edit(&self, operation: &Operation);
    fn validate_operation(&self, operation: &Operation) -> Result<()>;
}
```

---

## Conclusion

**Recommendation:** Implement **Alternative 1 (Monorepo with Git Submodules)** as it provides the best balance of:
- Professional appearance
- Clear product boundaries
- Community-friendliness
- Scalability
- Extensibility

**Next Steps:**
1. Create `add-ons/` directory and `.gitmodules` file
2. Implement LSP add-on (first stable add-on)
3. Document add-on development process (`docs/ADD_ON_DEVELOPMENT.md`)
4. Update ROADMAP.md with add-on implementation phases
5. Create initial add-on repositories (lsp, mcp)

---

**For AI Agents:**
This architecture provides:
1. **Stability** - Core features are production-ready and well-tested
2. **Extensibility** - Add-ons can enhance specific workflows (LSP, MCP, etc.)
3. **Flexibility** - Choose which add-ons you need
4. **Context** - Core provides project-wide understanding; add-ons provide specialized features

**For Users:**
- Install core for essential editing
- Add optional add-ons for enhanced features (LSP, MCP, visualization)
- Mix and match based on your workflow needs

---

**Installation Issue Resolution:**

The Fedora installation issue (GCC compilation errors with tree-sitter-qmljs) is an external dependency problem, not a GnawTreeWriter code issue. Workarounds:

**Recommended for Fedora users:**
```bash
# Use source build (bypasses cargo install compilation issues)
git clone https://github.com/Tuulikk/GnawTreeWriter.git
cd GnawTreeWriter
cargo build --release
./target/release/gnawtreewriter --version
```

This approach works reliably on all systems.

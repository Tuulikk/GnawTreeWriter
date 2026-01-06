# GnawTreeWriter Roadmap

## Overview

GnawTreeWriter is a tree-based code editor optimized for LLM-assisted editing. This roadmap outlines the evolution from a precise CLI tool to an intelligent agent-integrated platform.

## Current Status: v0.6.0 (Released 2025-01-05)

### ✅ Completed Features

- **Multi-language support**: Python, Rust, TypeScript, PHP, HTML, QML, **Go**, XML.
- **TreeSitter Foundation**: Robust parsing for all core languages.
- **Smart Indentation**: Automatic preservation of code style during insertions.
- **Syntax Validation**: In-memory re-parsing before saving changes.
- **QML Intents**: Dedicated commands for `add-property` and `add-component`.
- **Diff Preview**: Visual unified diff display using `similar` library.
- **Automatic Backups**: Non-git safety net creating JSON snapshots before every edit.
- **XML Support**: Implemented stable XML parsing using `xmltree` (declaration, DOCTYPE, comments, CDATA, attributes, nested elements) with line-number mapping and unit tests.
- **Batch Operations**: Atomic multi-file editing via JSON specification with in-memory validation, unified diff preview, automatic rollback on failure, and per-file transaction logging. Ideal for AI agent workflows and coordinated refactoring.
- **Quick Command**: Fast, low-overhead edits supporting node-edit mode (AST-based) and find/replace mode (text-based) with preview, backup, and parser validation.
- **Diff-to-Batch**: Converts unified diffs (git diff format) to batch operation specifications for safe, atomic application with full validation and rollback support.

---

## Phase 1: Reliability & Safety (The Non-Git Safety Net)
**Target: v0.3.0 - Q1 2026**

Focus on making the tool bulletproof and independent of Git for session-level recovery.

### **Core Safety & Recovery System**

- [x] **Transaction Log System**: 
  - JSON-based log file (`.gnawtreewriter_session.json`) tracking all operations
  - Human-readable format: `{"timestamp": "2025-01-02T15:30:45Z", "operation": "edit", "file": "app.py", "path": "0.1", "before_hash": "abc123", "after_hash": "def456", "description": "Updated function signature"}`
  - Session-scoped: cleared on explicit `gnawtreewriter session-start`, persists through crashes
  - Enables forensic analysis: "What happened to my code between 14:00 and 15:00?"
  - **STATUS**: ✅ COMPLETE - Integrated with all edit operations

- [x] **Multi-File Time Restoration System**:
  - `gnawtreewriter restore-project <timestamp> [--preview]` - Restore entire project to specific time
  - `gnawtreewriter restore-files --since <time> --files <patterns>` - Selective file restoration
  - `gnawtreewriter restore-session <session-id>` - Undo entire AI agent session
  - Project-wide time travel with atomic multi-file operations
  - Timestamp-based restoration with hash validation fallback
  - **STATUS**: ✅ COMPLETE - Working restoration engine implemented

- [x] **`undo` & `redo` Commands**:
  - `gnawtreewriter undo [--steps N]` - Reverse N operations (default 1)
  - `gnawtreewriter redo [--steps N]` - Re-apply N reversed operations  
  - `gnawtreewriter history [--format json/table]` - Show operation timeline
  - Navigate backup history without Git dependency
  - Atomic operation reversal: if undo fails, leave system in previous state
  - **STATUS**: Framework complete in `src/core/undo_redo.rs`, CLI commands added

- [x] **Enhanced Restore System**:
  - `gnawtreewriter restore <timestamp|operation-id> [--preview]`
  - Point-in-time recovery: "Restore app.py to state at 14:30"
  - Selective restoration: restore individual files or nodes
  - Preview system with comprehensive restoration planning
  - **STATUS**: ✅ COMPLETE - Full restoration engine with backup integration

- [x] **Interactive Help System**:
  - `gnawtreewriter examples [--topic <topic>]` - Practical workflow examples
  - `gnawtreewriter wizard [--task <task>]` - Interactive guidance system
  - Enhanced command help with detailed descriptions and use cases
  - Topic-specific help: editing, qml, restoration, workflow, troubleshooting
  - **STATUS**: ✅ COMPLETE - Revolutionary help system for AI agents and humans

- [x] **AI Agent Testing Framework**:
  - Comprehensive test scenarios document (AI_AGENT_TEST_SCENARIOS.md)
  - 8 detailed test scenarios from discovery to integration
  - Structured evaluation framework with rating system (1-5 scale)
  - Sample test files and complete environment setup
  - **STATUS**: ✅ COMPLETE - Ready for AI agent evaluation and feedback

- [ ] **Stable Node Addressing**:
  - Content-based node IDs: `node_abc123def` (hash of node content + position)
  - Graceful fallback to path-based addressing when content changes
  - Cross-edit stability: same logical node keeps same ID across minor edits
  - Migration tool: convert old path-based references to content-based IDs

---

## Phase 2: Multi-Project & Commercial Features  
**Target: v0.3.0 - Q1 2026**

Transform from single-project tool to multi-project development platform.

### **Multi-Project Architecture**

- [ ] **Project Manager System**:
  - Support for multiple concurrent projects per user
  - Team Starter: 15 concurrent projects ($19/month)
  - Professional: Unlimited projects ($49/month)
  - Project switching and archival without data loss

- [ ] **Configurable Limits System**:
  - Dynamic project limits based on user tier
  - Feature flag architecture for instant competitive response
  - A/B testing framework for optimal limit discovery
  - Graceful upgrade prompts and user experience

- [ ] **Basic Team Coordination**:
  - Shared session visibility across team members
  - Cross-developer project state synchronization
  - Team-scoped transaction history and restoration
  - Basic conflict resolution for concurrent edits

- [ ] **MCP Server Implementation**: 
  - Native Model Context Protocol support as built-in tool
  - Tool definitions for all major operations (edit, analyze, find, restore)
  - Context-aware responses optimized for LLM processing
  - Batch operation support: multiple edits in single MCP call

---

## Phase 3: AI Agent Integration & Intelligence
**Target: v0.4.0 - Q2 2026**

Transform into AI-native development platform.

### **Advanced AI Agent Features**

- [ ] **Smart Semantic Targeting**:
  - `--function "main"` instead of raw paths
  - `--class "UserController" --method "create"`  
  - `--property "width" --within "Rectangle"`
  - Natural language queries: `--find "the button that handles login"`
  - Fuzzy matching with confidence scores

- [ ] **LLM-Optimized Output**:
  - Token-compressed JSON formats for large ASTs
  - Hierarchical detail levels: summary → detailed → full AST
  - Context window management: smart truncation preserving important nodes
  - Streaming responses for large operations

- [ ] **Intent Extrapolation Engine**:
  - High-level commands: `gnawtreewriter refactor-extract-function app.py "calculate_total" --lines 45-60`
  - Pattern-based transformations: `gnawtreewriter apply-pattern observer app.py --class "DataModel"`
  - Architecture enforcement: `gnawtreewriter ensure-pattern repository database.py`

- [ ] **Cross-Project Intelligence**:
  - Dependency tracking across multiple projects
  - API contract monitoring and change impact analysis
  - Automated consistency checking across project boundaries
  - Smart suggestions based on cross-project patterns

---

## Phase 4: Autonomous Code Guardian & Enterprise Platform
**Target: v0.5.0 - Q3 2026**

Evolve into always-on enterprise development infrastructure.

### **Continuous Monitoring System**

- [ ] **File System Watcher & Daemon**:
  - `gnawtreewriter daemon start` - Background process monitoring all projects
  - Real-time AST updates when files change (even from external tools)
  - Change event streaming to connected AI agents
  - Conflict detection: "File changed outside GnawTreeWriter, merging changes"

- [ ] **Enterprise Policy Engine**:
  - Company-specific coding standards enforcement
  - Architecture decision record (ADR) compliance checking  
  - Automated security vulnerability patching
  - Custom policy DSL for organizational rules

- [ ] **Multi-Tenant Cloud Service**:
  - SaaS version with per-organization isolation
  - Enterprise SSO integration (SAML, OIDC)
  - Comprehensive audit logging for compliance (SOX, GDPR, HIPAA)
  - Global policy enforcement across teams and projects
  - Advanced analytics and reporting dashboard

- [ ] **Intelligent Structural Analysis**:
  - Architectural lint rules: "Controllers should not directly access database"
  - Security scanning: "No hardcoded secrets detected"
  - Performance monitoring: "Large function detected, suggest refactoring"
  - Cross-project dependency impact analysis

## Phase 5: Universal Tree Platform (2027+)
**Target: v0.6.0+ - Future Expansion**

Expand beyond code to all hierarchical systems.

### **Multi-Domain Tree Support**

- [ ] **Infrastructure as Code**:
  - Terraform/CloudFormation AST parsing and editing
  - `gnawtreewriter scale-service infrastructure.tf "web_servers" --count 5`
  - Cloud resource dependency visualization
  - Cost impact analysis for infrastructure changes

- [ ] **Configuration Management**:
  - Docker Compose, Kubernetes YAML, CI/CD pipelines
  - `gnawtreewriter add-service docker-compose.yml "redis" --image "redis:7"`
  - Environment-specific configuration templating
  - Secret management integration

- [ ] **AI-Native Development Ecosystem**:
  - Autonomous refactoring agents continuously improving architecture
  - Cross-language translation preserving tree structure
  - Predictive code evolution and architectural suggestions
  - Natural language programming: "Create REST API" → Full implementation

## Community Plugin System 🧩

### Philosophy

GnawTreeWriter supports a **community-driven plugin architecture** where:
- **Core features** (batch, time travel, sessions, tags) are maintained and tested by GnawTreeWriter
- **Extended features** (LSP, MCP, visualization, etc.) can be developed by the community
- **Users have freedom** to install only what they need
- **Contributions are welcome** and structured through a clear plugin system

This creates an ecosystem where GnawTreeWriter is the "editor platform" and add-ons are "extensions" that can be built by anyone.

### Plugin Tiers

#### Tier 1: Official Plugins (Stable, Core-Equivalent)

Plugins that are maintained and supported by the GnawTreeWriter project. These are considered part of the core platform and receive the same stability guarantees as core features.

**Examples:**
- **LSP Plugin** (`gnawtreewriter-lsp`) - Language Server Protocol integration
- **MCP Daemon** (`gnawtreewriter-mcp`) - Model Context Protocol integration
- **Visualization UI** (`gnawtreewriter-ui`) - Interactive diff and timeline explorer

**Characteristics:**
- ✅ Production-ready stability
- ✅ Full support and documentation
- ✅ Versioned with core (lsp v1.0, core v0.4.0 → same v0.4.0)
- ✅ Security audited
- ✅ Integrated with CLI (seamless experience)
- 🎯 **Considered part of "core" for users**

#### Tier 2: Community Plugins (Experimental, Community-Driven)

Plugins developed by the community or third parties. These may be experimental, have different release cycles, or target specific workflows.

**Examples:**
- **Profiling Tools** (`gnawtreewriter-profiler`) - Performance and complexity analysis
- **Language-specific Refactoring** (`gnawtreewriter-rust-refactor`) - Rust-specific refactors
- **Testing Frameworks** (`gnawtreewriter-test-gen`) - Automated test generation
- **Custom Parsers** (`gnawtreewriter-custom-parser`) - Experimental parser support

**Characteristics:**
- 🟡 Community-maintained or third-party
- 🟡 Experimental features may be unstable
- 🟡 Separate versioning (plugin v0.1.0 vs core v0.4.0)
- 🟡 Use with caution - may have bugs
- 🎯 **Opt-in only** - not enabled by default
- 🎯 **Community-driven development** - anyone can contribute

#### Tier 3: User Scripts (Local, Private)

Simple scripts or automation developed by users for their specific workflows. Not distributed, just shared.

**Examples:**
- **Project Setup Scripts** - Automate initial project configuration
- **Batch Operations** - Reusable batch JSON files for common workflows
- **Git Integration** - Custom git hooks for GnawTreeWriter
- **Deployment Scripts** - CI/CD pipeline integration

**Characteristics:**
- 🟢 Simple shell scripts, Python, etc.
- 🟢 Not packaged as plugins
- 🟢 Local use only
- 🎯 **Flexibility** - adapt GnawTreeWriter to your workflow

### Plugin Architecture

```
┌─────────────────────────────────────────────────────────┐
│              GnawTreeWriter Platform                    │
│  ┌──────────────┬───────────────┬──────────────┐│
│  │ Core Editing  │ Time Travel   │ Sessions & Tags ││
│  │ (always-on)   │ (always-on)  │ (always-on)   ││
│  │ ┌──────────────┴───────────────┬──────────────┐│
│  │        GnawTreeWriter Core CLI              │                ││
│  └──────────────────────────────────────────────────┘                │
│                                                              │
│  ┌────────────────────────────────────────────────────────────────────────┐│
│  │                 Plugin Interface Layer                  │                ││
│  └────────────────────────────────────────────────────────────────────────┘│
│                                                              │
│  ┌──────────────┬──────────────┬──────────────┬──────────────┬──────────────┐│
│  │ Official    │ Community    │ Experimental  │    User      ││
│  │  Plugins    │ Plugins      │ Plugins      │    Scripts   ││
│  │  ┌────────┬────────┐│  ┌────────┬────────┐│  ┌────────┬────────┐│
│  │  │ LSP    │  MCP   ││  │  Profiler │ Refactor││  │  Custom  │
│  │  └────────┴────────┘│  └────────┴────────┘│  └────────┴────────┘│  └────────┴────────┘│
│  └──────────────┴──────────────┴──────────────┴──────────────┴──────────────┘│
│                                                              │
│  ┌────────────────────────────────────────────────────────────────────────┐│
│  │                Integration Layer                          │                │
│  └────────────────────────────────────────────────────────────────────────┘│
│                                                              │
├──────────────────────────────────────────────────────────────────────────────┤
│              Projects using GnawTreeWriter                      │
├──────────────────────────────────────────────────────────────────────────────┤
└──────────────────────────────────────────────────────────────────────────────┘
```

### Plugin Development Workflow

#### Step 1: Planning
```bash
# Start with documentation
cat > PLUGIN_PLAN.md << 'EOF'
# Feature Name: [My Plugin]
# Purpose: [What it does]
# Target: [Official / Community / Script]

# Define functionality
# - Core integration points needed
# - New data structures (if any)
# - CLI commands (if any)
# - Configuration options

# Identify dependencies
# - Which core modules needed?
# - External dependencies?
# - Testing requirements
EOF
```

#### Step 2: Skeleton
```bash
# Create plugin directory
mkdir -p plugins/my-plugin

# Create basic structure
cat > plugins/my-plugin/Cargo.toml << 'EOF'
[package]
name = "gnawtreewriter-{{name}}"
version = "0.1.0"
edition = "2021"

[dependencies]
gnawtreewriter = "0.4.0"
anyhow = "1.0"
EOF
```

#### Step 3: Implementation
```rust
// src/core/mod.rs - Plugin trait already defined

pub trait GnawTreePlugin {
    fn name(&self) -> &'static str;
    fn version(&self) -> &str;
    fn description(&self) -> &str;
    
    // Integration hooks
    fn before_edit(&self, operation: &EditOperation) -> Result<()>;
    fn after_edit(&self, operation: &EditOperation) -> Result<()>;
    
    // CLI extension (optional)
    fn register_commands(&self, cli: &mut Command);
}

// Example plugin implementation
pub struct MyPlugin {
    writer: GnawTreeWriter,
}

impl GnawTreePlugin for MyPlugin {
    fn name(&self) -> &'static str { "My Plugin" }
    fn version(&self) -> &str { "0.1.0" }
    fn description(&self) -> &str { "My awesome plugin functionality" }
    
    fn before_edit(&self, op: &EditOperation) -> Result<()> {
        // Custom validation or logging before edit
        Ok(())
    }
    
    fn after_edit(&self, op: &EditOperation) -> Result<()> {
        // Post-edit actions (notifications, analysis, etc.)
        Ok(())
    }
}
```

#### Step 4: Registration
```rust
// src/core/plugin_manager.rs - Core loads and registers plugins

pub struct PluginManager {
    plugins: Vec<Box<dyn GnawTreePlugin>>,
}

impl PluginManager {
    pub fn load(&mut self, path: &Path) -> Result<()> {
        // Load plugin from directory
        let plugin = MyPlugin::new(&path)?;
        self.plugins.push(plugin);
        Ok(())
    }
    
    pub fn register_cli_commands(&self, cli: &mut Command) {
        for plugin in &self.plugins {
            plugin.register_commands(cli);
        }
    }
}
```

#### Step 5: Testing
```bash
# Isolated plugin tests
cd plugins/my-plugin
cargo test

# Integration tests with core
cd ..
cargo test --test-path plugins/my-plugin

# Manual testing
gnawtreewriter plugin my-plugin test-file.txt --dry-run
gnawtreewriter --plugin my-plugin edit test-file.txt "0" "test content"
```

### Plugin Distribution

#### Option 1: As Repository Submodules (Future - Mature)

```bash
# Add to main repo
git submodule add https://github.com/username/gnawtreewriter-lsp.git
git submodule update --init --recursive
```

**Pros:**
- ✅ Clean versioning
- ✅ Official project organization
- ✅ Easy to browse and explore

**Cons:**
- ❌ Requires `git submodule update` for users
- ❌ More complex git history

#### Option 2: As Standalone Repositories (Current - Simple)

```bash
# Clone separately
git clone https://github.com/username/gnawtreewriter-lsp.git
cd gnawtreewriter-lsp
cargo install --path ../gnawtreewriter
```

**Pros:**
- ✅ Simple for users
- ✅ Independent versioning
- ✅ Easy discovery via GitHub

**Cons:**
- ❌ Manual integration required
- ❌ Versioning separate from core

#### Option 3: Local Development (For User Scripts)

```bash
# Create plugin in local gnawtreewriter/plugins/ directory
mkdir -p ~/.gnawtreewriter/plugins/my-plugin
cp -r my-plugin/ ~/.gnawtreewriter/plugins/my-plugin/
```

**Pros:**
- ✅ Private, not shared
- ✅ Perfect for personal workflows
- ✅ No version management needed

**Cons:**
- ❌ Not shareable
- ❌ Doesn't contribute to ecosystem

### Plugin Integration Guidelines

#### For Official Plugin Developers

1. **Follow Architecture** - Use `GnawTreePlugin` trait
2. **Core Compatibility** - Support current core version (0.4.0)
3. **Comprehensive Testing** - Unit, integration, and manual tests
4. **Documentation** - README, examples, and API reference
5. **Version Alignment** - Match core versioning (if official plugin)
6. **Security** - Audit for vulnerabilities before release
7. **Maintenance** - Keep updated with core changes
8. **Communication** - Use issues and PRs for development

#### For Community Plugin Developers

1. **Start Small** - Begin with minimal functionality
2. **Document Everything** - Installation, usage, examples
3. **Test Thoroughly** - Manual testing on real projects
4. **Be Transparent** - Clearly mark as experimental/beta
5. **Version Independently** - Don't couple to core versioning
6. **Solicit Feedback** - Get community input early and often
7. **Handle Issues Gracefully** - Respond to bug reports quickly
8. **Share Your Work** - Publish on GitHub with good documentation

#### For User Script Writers

1. **Keep It Simple** - Shell or Python scripts work best
2. **Use Core Features** - Leverage batch, time travel, tags
3. **Document Usage** - Explain how to run and customize
4. **Share If Useful** - Put in gist or small repo if others might benefit
5. **Consider Making Plugin** - If your script becomes popular, package it as proper plugin

### Community Resources

- **Plugin Development Guide**: See `docs/PLUGIN_DEVELOPMENT.md`
- **API Reference**: See `docs/PLUGIN_API.md`
- **Example Plugins**: See `plugins/` directory for reference implementations
- **Community Discussions**: Use GitHub Discussions for plugin ideas and help
- **Plugin Showcase**: Share your plugins in community events and blog posts

### Integration with Existing Features

#### Plugins Can Extend:

**1. Batch Operations**
```bash
# Plugin adds new operation type
gnawtreewriter batch ops.json  # Uses batch system
```

**2. Time Travel**
```bash
# Plugin creates restore points
gnawtreewriter restore-session <plugin_id>
```

**3. Named References (Tags)**
```bash
# Plugin registers tag types
gnawtreewriter tag add my-file "custom.node" "Custom Tag"
```

**4. Sessions**
```bash
# Plugin creates session snapshots
gnawtreewriter session-start
```

### Benefits for Users

1. **Freedom** - Install only what you need
2. **Choice** - Mix official and community plugins
3. **Innovation** - Community can experiment with new features
4. **Customization** - Tailor GnawTreeWriter to your workflow
5. **Cost-Effective** - Free core + free community plugins
6. **Ecosystem Growth** - More features without core bloat

### Benefits for GnawTreeWriter

1. **Extensibility Without Bloat** - Core remains lightweight
2. **Community Growth** - Plugins extend functionality without core changes
3. **Innovation Source** - Community drives new ideas
4. **Ecosystem Maturity** - Diverse plugins make GnawTreeWriter more useful
5. **Maintenance Distribution** - Community shares load of plugin maintenance

### Future Evolution

As the community grows:

1. **Official Plugins** may graduate from experimental to stable to official status
2. **Popular Community Plugins** may be promoted to official if well-maintained
3. **Plugin Marketplace** - Easy discovery and installation of plugins
4. **Standardization** - Community defines best practices for plugin development
5. **Modular Architecture** - Ability to separate concerns (e.g., different LSP servers for different needs)

---

## Integration Ecosystem

- [ ] **LSP Server**: Universal structured editing for all IDEs
- [ ] **GitHub App**: Automated PR reviews and suggestions
- [ ] **IDE Extensions**: Native plugins for VS Code, IntelliJ, Neovim  
- [ ] **API Gateway**: RESTful API for third-party tool integration

---

## Implementation Priorities
GnawTreeWriter follows a **core + add-on architecture** to maintain focus while enabling extensibility:

#### Core Principles

**Core** (always present, no installation required):
- **AST-based editing** - The fundamental editing capability
- **Time travel** - Project-wide backups and restoration
- **Sessions & Transactions** - Audit trail and undo/redo
- **Batch operations** - Atomic multi-file edits
- **Named references (tags)** - Stable node addressing
- **Universal parsers** - Support for all major languages

**Add-ons** (opt-in, separate deployment):
- **LSP integration** - Semantic analysis, symbol resolution
- **MCP daemon** - AI agent coordination and orchestration
- **Advanced refactoring** - Rename, extract, change signatures
- **Visualization** - Better diff views, time travel UI
- **Language server** - Per-language semantic features

#### Plugin System Design

```
┌─────────────────────────────────────────┐
│  GnawTreeWriter Core                   │
│  ┌──────────┬──────────┐        │
│  │ AST Edit  │ Time Travel│        │
│  │ Sessions  │ Batch Ops │        │
│  │ Tags      │ Multi-Lang │        │
│  └──────────┴──────────┘        │
└─────────────────────────────────────────┘
              ▲
              │ Plug-in Interface
              ▲
┌────────────────────┬──────────────┬──────────────┐
│  LSP Add-on │ MCP Daemon │ Visualization │
│  (semantic)  │ (coord)    │  (UI)       │
└────────────────────┴──────────────┴──────────────┘
```

#### Add-on Examples

**LSP Add-on** (`gnawtreewriter-lsp`):
```bash
gnawtreewriter lsp-analyze project/
gnawtreewriter lsp-hover src/main.rs:45
gnawtreewriter lsp-find-definition AuthManager
```

**MCP Daemon** (`gnawtreewriter-mcp`):
```bash
gnawtreewriter mcp daemon start
gnawtreewriter mcp list-sessions
gnawtreewriter mcp execute-agents
```

**Visualization Add-on** (`gnawtreewriter-ui`):
```bash
gnawtreewriter timeline view
gnawtreewriter diff interactive
gnawtreewriter restore wizard
```

#### Benefits

- **Core remains lightweight** - No bloat from advanced features
- **User choice** - Install only what you need
- **Easy contribution** - Clear boundaries for community contributions
- **Version compatibility** - Add-ons can evolve independently
- **Monetization ready** - Enterprise features can be add-ons

#### Development Roadmap

- **Phase 1** (v0.4.0): LSP add-on prototype
- **Phase 2** (v0.5.0): MCP daemon implementation
- **Phase 3** (v0.6.0): Visualization add-on
- **Phase 4** (v0.7.0): Plugin marketplace

**See [docs/ADD_ON_ARCHITECTURE.md](docs/ADD_ON_ARCHITECTURE.md) for detailed design.**

---

## Implementation Priorities

### **Phase 1 Quick Wins** (Next 3 months)
1. Transaction logging system (foundation for all future features)
2. Undo/redo commands (immediate developer value)
3. Enhanced restore with preview (safety & confidence)
4. Content-based node IDs (stability for AI agents)

### **Phase 2 AI Integration** (Months 4-9)  
1. MCP server implementation (unlock AI ecosystem)
2. Semantic targeting (make AI agents more effective)
3. LLM-optimized output formats (performance & usability)
4. Intent extrapolation engine (high-level automation)

### **Multi-Agent Development Strategy**

Based on AI agent strengths observed:

- **Gemini 3**: Architecture decisions, documentation, long-term planning
- **Claude**: Implementation details, error handling, user experience  
- **GLM-4.7**: Fast iteration on specific features (with careful monitoring)
- **Raptor Mini**: User experience feedback, edge case identification

### **Success Metrics**

- **Phase 1**: Zero data loss incidents, <5 second recovery time
- **Phase 2**: 80% of AI editing tasks use GnawTreeWriter instead of raw text
- **Phase 3**: 90% reduction in structural code errors across team
- **Phase 4**: Enterprise adoption with measurable productivity gains

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## Documentation

- [ARCHITECTURE.md](docs/ARCHITECTURE.md) - Technical design  
- [FUTURE_CONCEPTS.md](docs/FUTURE_CONCEPTS.md) - Deep dive into planned features
- [LLM_INTEGRATION.md](docs/LLM_INTEGRATION.md) - Guide for AI agents
- [MULTI_AGENT_DEVELOPMENT.md](docs/MULTI_AGENT_DEVELOPMENT.md) - Collaboration strategies ✓

## Recent Progress (2025-12-27)

### ✅ **Phase 1 HISTORIC MILESTONE - COMPLETE!**
- **Revolutionary multi-file time restoration system** implemented and working
- **Transaction logging fully integrated** with all edit operations  
- **Complete restoration engine** with backup file integration working
- **Project-wide time travel** verified: `restore-project`, `restore-files`, `restore-session`
- **Timestamp-based restoration** with hash validation fallback
- **Multi-agent development documentation** complete
- **Revolutionary help system** with examples, wizards, and interactive guidance
- **AI agent test framework** with comprehensive evaluation scenarios
- **Professional documentation** ready for community and enterprise adoption

### 🎉 **VERIFIED WORKING CAPABILITIES:**
- ✅ Project restoration: Successfully restored files to specific timestamps
- ✅ Multi-file coordination: Atomic restoration operations across multiple files
- ✅ Session-based restoration: Undo entire AI agent workflow sessions
- ✅ Backup file integration: Robust backup parsing and content restoration
- ✅ Error handling: Graceful fallbacks and detailed error reporting
- ✅ Preview system: Safe restoration planning before execution
- ✅ Help system: Interactive wizards, examples, and comprehensive command help
- ✅ AI testing framework: 8 detailed scenarios with structured evaluation
- ✅ GitHub publication: Complete repository ready for community adoption

### ✅ Recent Additions (2025-12-28)
- [x] Implicit Sessions — Sessions auto-start on first edit and persist across commands
- [x] Built-in Diff View — `gnawtreewriter diff` shows exact changes per transaction
- [x] Generic Node support — Generic parser for unknown files; backups/history and text-based editing for all files

### 🔄 **Phase 1 Enhancement Tasks**
- [ ] Add content-based node ID system for enhanced stability
- [ ] Polish hash-matching algorithm for optimal restoration performance
- [ ] Collect and incorporate AI agent feedback from test scenarios
- [ ] Add restoration statistics and analytics dashboard

### 🔮 **Planned Next Steps**
- [ ] Named References (Priority #4): Implement `tag` support and CLI tooling to create, list and remove named references to node paths (improves script resilience)
- [ ] LSP Add-ons: Explore LSP integration as optional add-ons (semantic analysis, completions and diagnostics) — envisioned as separate, opt-in add-on(s) rather than core functionality
- [ ] Diff Enhancements: Improve diff-to-batch with AST-aware conversion (use node paths instead of line numbers), multi-file batch support in single JSON, and conflict detection
- [ ] Quick Command Enhancements: Add `--first` flag for single occurrence replacement in find/replace mode, more specific error messages for AST-related failures

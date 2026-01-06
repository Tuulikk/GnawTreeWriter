# GnawTreeWriter Roadmap

## Overview

GnawTreeWriter is a tree-based code editor optimized for LLM-assisted editing. This roadmap outlines the evolution from a precise CLI tool to an intelligent agent-integrated platform.

The roadmap is divided into two sections:
- **Open Source** - Core functionality, community-driven features, available to everyone
- **Premium/Enterprise** - Commercial features, team collaboration, enterprise integrations

---

## Current Status: v0.6.0 (Released 2025-01-05)

### ✅ Completed Features

- **Multi-language support**: Python, Rust, TypeScript, JavaScript, PHP, HTML, QML, Go, Java, Zig, C, C++, Bash, XML, YAML, TOML, CSS, Markdown
- **TreeSitter Foundation**: Robust parsing for all core languages
- **Smart Indentation**: Automatic preservation of code style during insertions
- **Syntax Validation**: In-memory re-parsing before saving changes
- **Diff Preview**: Visual unified diff display using `similar` library
- **Automatic Backups**: Non-git safety net creating JSON snapshots before every edit
- **Batch Operations**: Atomic multi-file editing via JSON specification with rollback
- **Quick Command**: Fast, low-overhead edits supporting AST and text-based modes
- **Diff-to-Batch**: Converts unified diffs to batch operation specifications
- **ModernBERT AI Integration**: Local, privacy-focused AI features (optional)
- **Clone Operation**: Duplicate code nodes within or between files
- **Refactor/Rename**: AST-aware symbol renaming across files

---

# 🌍 Open Source Roadmap

All features in this section are and will remain **free and open source** under the project license.

---

## Phase 1: Reliability & Safety ✅ COMPLETE
**Status: v0.3.0 - DONE**

The foundation that makes GnawTreeWriter bulletproof and independent of Git for session-level recovery.

### ✅ Core Safety & Recovery System

- [x] **Transaction Log System**: JSON-based log tracking all operations with timestamps
- [x] **Multi-File Time Restoration**: `restore-project`, `restore-files`, `restore-session`
- [x] **Undo & Redo Commands**: Navigate backup history without Git dependency
- [x] **Enhanced Restore System**: Point-in-time recovery with preview
- [x] **Interactive Help System**: `examples` and `wizard` commands for guided learning
- [x] **AI Agent Testing Framework**: Comprehensive test scenarios for AI agents

### 🔄 Planned Enhancements

- [ ] **Stable Node Addressing**: Content-based node IDs (`node_abc123def`)
- [ ] **Hash-matching Optimization**: Improved restoration performance
- [ ] **Restoration Analytics**: Statistics dashboard for backup usage

---

## Phase 2: MCP Integration & Daemon
**Target: v0.7.0 - Q2 2025**

Core MCP and daemon features that make GnawTreeWriter usable by AI agents and IDEs.

### **Optional Features**

> 📦 **Note**: Both MCP server and local daemon are **optional features** that must be enabled at compile time, similar to the `modernbert` AI feature.

### **MCP Server** (Optional Feature)

**Installation Options:**

```bash
# Core only (minimal build)
cargo install --path .

# With AI features (ModernBERT)
cargo install --path . --features modernbert

# With MCP server and daemon
cargo install --path . --features mcp

# With all features (AI + MCP + CUDA)
cargo install --path . --features modernbert,mcp,cuda
```

**All features are optional** - build exactly what you need!

- [ ] **Basic MCP Tool Server**:
  - `gnawtreewriter mcp serve` - Run as MCP server
  - Expose all core operations as MCP tools (analyze, edit, batch, undo, restore)
  - Stateless request/response model for simple integration
  - Works with Claude Desktop, Cursor, and other MCP clients

- [ ] **MCP Tool Definitions**:
  - `gnawtreewriter_analyze` - Parse and return AST structure
  - `gnawtreewriter_edit` - Edit a specific node
  - `gnawtreewriter_batch` - Execute atomic multi-file operations
  - `gnawtreewriter_undo` / `gnawtreewriter_redo` - Navigate history
  - `gnawtreewriter_restore` - Time travel to specific point

### **Local File Watcher Daemon** (Optional Feature)

**Installation:**
```bash
# Install with daemon support
cargo install --path . --features mcp

# Daemon works independently of MCP server
```

- [ ] **Project Monitoring Daemon**:
  - `gnawtreewriter daemon start [--project <path>]` - Start background watcher
  - Real-time file change detection (even from external editors)
  - Automatic backup on every save (extends current backup system)
  - Conflict detection: "File changed outside GnawTreeWriter"
  
- [ ] **Daemon Features**:
  - Watch multiple projects simultaneously
  - Event streaming for connected clients
  - Low resource footprint (inotify/fsevents based)
  - Graceful shutdown and restart

> 🔧 **Architecture Note**: The boundary between built-in daemon and plugin is still being designed. Community input welcome!

> 💡 **Premium Note**: Multi-project coordination, centralized server, and team synchronization available in [Premium Phase 1](#phase-1-multi-project--team-collaboration).

---

## Phase 3: AI-Enhanced Editing

---

## Phase 3: AI-Enhanced Editing
**Target: v0.8.0 - Q3 2025**

AI features that enhance the editing experience for everyone.

### **Smart Semantic Targeting**

- [ ] **Semantic Node Selection**:
  - `--function "main"` instead of raw paths
  - `--class "UserController" --method "create"`
  - `--property "width" --within "Rectangle"`
  - Fuzzy matching with confidence scores

- [ ] **LLM-Optimized Output**:
  - Token-compressed JSON formats for large ASTs
  - Hierarchical detail levels: summary → detailed → full AST
  - Context window management with smart truncation

### **Local AI Features** (ModernBERT)

- [x] **Semantic Search**: Find code by meaning with `--semantic` flag
- [x] **AI Refactoring Suggestions**: Identify complex code patterns
- [x] **Context-Aware Completion**: AST-based code completion
- [ ] **Pattern Detection**: Identify anti-patterns and suggest improvements

> 💡 **Premium Note**: Advanced multi-project AI analysis and cross-project intelligence available in [Premium Phase 2](#phase-2-advanced-ai--cross-project-intelligence).

---

## Phase 4: Language & Parser Expansion
**Target: v0.9.0 - Q4 2025**

Expand language support and parser capabilities.

### **New Languages**

- [ ] **Kotlin**: Full TreeSitter-based parser
- [ ] **Swift**: iOS/macOS development support
- [ ] **Scala**: JVM ecosystem expansion
- [ ] **Ruby**: Dynamic language support
- [ ] **Lua**: Scripting language support

### **Parser Improvements**

- [ ] **Custom Parser SDK**: Allow users to create parsers for proprietary formats
- [ ] **Parser Hot-Reloading**: Update parsers without restarting
- [ ] **Multi-Parser Files**: Handle embedded languages (JS in HTML, etc.)

---

## Phase 5: Universal Tree Platform
**Target: v1.0.0 - 2026**

Expand beyond code to all hierarchical systems.

### **Infrastructure as Code**

- [ ] **Terraform Parsing**: Edit infrastructure with AST precision
- [ ] **CloudFormation Support**: AWS template editing
- [ ] **Kubernetes YAML**: K8s manifest manipulation

### **Configuration Management**

- [ ] **Docker Compose**: Service definition editing
- [ ] **CI/CD Pipelines**: GitHub Actions, GitLab CI, Jenkins
- [ ] **Environment Templates**: Cross-environment configuration

> 💡 **Premium Note**: Advanced infrastructure analysis, cost impact analysis, and enterprise policy enforcement available in [Premium Phase 3](#phase-3-enterprise-platform--governance).

---

## Community Plugin System 🧩

GnawTreeWriter supports a **community-driven plugin architecture** where anyone can extend functionality.

### Plugin Tiers

#### Tier 1: Official Plugins (Stable)
Maintained by the GnawTreeWriter project with full support.

- **LSP Plugin** (`gnawtreewriter-lsp`) - Language Server Protocol integration
- **Visualization UI** (`gnawtreewriter-ui`) - Interactive diff and timeline explorer

#### Tier 2: Community Plugins (Experimental)
Developed by the community with varying stability.

- **Language-specific Refactoring** - Rust, Python, Java specific tools
- **Testing Frameworks** - Automated test generation
- **Custom Parsers** - Experimental language support

#### Tier 3: User Scripts (Local)
Simple scripts for personal workflows.

- **Batch Templates** - Reusable JSON batch operations
- **Git Hooks** - Pre-commit/post-commit integration
- **CI/CD Integration** - Pipeline helper scripts

### Plugin Architecture

```
┌─────────────────────────────────────────────────────────┐
│              GnawTreeWriter Core (Open Source)          │
│  ┌──────────┬───────────────┬──────────────────┐       │
│  │ AST Edit │ Time Travel   │ Sessions & Tags  │       │
│  │ Batch Ops│ Multi-Lang    │ AI (ModernBERT)  │       │
│  │ MCP Serve│ Local Daemon  │ File Watcher     │       │
│  └──────────┴───────────────┴──────────────────┘       │
└─────────────────────────────────────────────────────────┘
                          ▲
                          │ Plugin Interface
                          ▼
┌─────────────┬─────────────┬─────────────┬─────────────┐
│ Official    │ Community   │ User        │ Premium     │
│ Plugins     │ Plugins     │ Scripts     │ Add-ons     │
│ (LSP, UI)   │ (Profiler)  │ (Batch)     │ (Server)    │
└─────────────┴─────────────┴─────────────┴─────────────┘
```

---

# 💎 Premium/Enterprise Roadmap

Commercial features for teams, enterprises, and power users. Revenue from Premium features supports continued Open Source development.

> **Pricing** (Planned):
> - **Team Starter**: $19/month - 15 projects, basic team features
> - **Professional**: $49/month - Unlimited projects, full feature set
> - **Enterprise**: Contact us - Custom limits, SSO, compliance

---

## Phase 1: Multi-Project & Team Collaboration
**Target: Q2 2025**

Transform from single-project tool to multi-project team platform.

### **Multi-Project Architecture**

- [ ] **Project Manager System**:
  - Support for multiple concurrent projects per user
  - Project switching and archival without data loss
  - Cross-project search and navigation

- [ ] **Configurable Limits System**:
  - Dynamic project limits based on user tier
  - Feature flag architecture for instant updates
  - Graceful upgrade prompts

### **Team Coordination**

- [ ] **Shared Session Visibility**: See what teammates are editing
- [ ] **Cross-Developer Sync**: Real-time project state synchronization
- [ ] **Team Transaction History**: Shared timeline across team members
- [ ] **Basic Conflict Resolution**: Handle concurrent edits gracefully

### **Coordination Server** (Self-Hosted or SaaS)

- [ ] **GnawTreeWriter Server**:
  - Deployable on your own infrastructure OR available as managed service
  - Coordinates multiple GnawTreeWriter daemons across machines
  - Centralized project registry and configuration
  - WebSocket-based real-time synchronization

- [ ] **Multi-Agent Coordination**:
  - Agent session management and conflict prevention
  - Work queue distribution across AI agents
  - Rollback coordination for multi-agent failures
  - Agent activity dashboard and monitoring

- [ ] **Team Dashboard**:
  - Real-time view of all project activity
  - Transaction history across all team members
  - Backup status and health monitoring
  - Usage analytics and reporting

> 💡 **Open Source Note**: Basic MCP server and single-project daemon are available in [Open Source Phase 2](#phase-2-mcp-integration--daemon). Premium adds multi-project coordination, centralized server, and team features.

---

## Phase 2: Advanced AI & Cross-Project Intelligence
**Target: Q3 2025**

AI features that require cloud infrastructure or team coordination.

### **Cross-Project Intelligence**

- [ ] **Dependency Tracking**: Monitor dependencies across all projects
- [ ] **API Contract Monitoring**: Detect breaking changes automatically
- [ ] **Cross-Project Search**: Semantic search across entire organization
- [ ] **Pattern Library**: Shared code patterns and templates

### **Intent Extrapolation Engine**

- [ ] **High-Level Commands**:
  - `gnawtreewriter refactor-extract-function app.py "calculate_total" --lines 45-60`
  - `gnawtreewriter apply-pattern observer app.py --class "DataModel"`
  - `gnawtreewriter ensure-pattern repository database.py`

- [ ] **Architecture Enforcement**:
  - Pattern compliance checking
  - Automated architectural suggestions
  - Code organization recommendations

> 💡 **Open Source Note**: Basic semantic search with ModernBERT is available in Open Source. Advanced cross-project analysis and cloud-based AI are Premium-only.

---

## Phase 3: Enterprise Platform & Governance
**Target: Q4 2025**

Enterprise-grade features for compliance, security, and governance.

### **Enterprise Policy Engine**

- [ ] **Coding Standards Enforcement**: Company-specific rules
- [ ] **ADR Compliance**: Architecture Decision Record checking
- [ ] **Security Scanning**: Automated vulnerability detection
- [ ] **Custom Policy DSL**: Organizational rule definitions

### **Audit & Compliance**

- [ ] **Comprehensive Audit Logging**: SOX, GDPR, HIPAA ready
- [ ] **Change Attribution**: Who changed what, when, why
- [ ] **Retention Policies**: Configurable backup retention
- [ ] **Export & Reporting**: Compliance report generation

### **Enterprise SSO & Security**

- [ ] **SAML/OIDC Integration**: Enterprise identity providers
- [ ] **Role-Based Access Control**: Granular permissions
- [ ] **IP Whitelisting**: Network security controls
- [ ] **Encryption at Rest**: Enterprise-grade data protection

---

## Phase 4: Autonomous Code Guardian
**Target: 2026**

Always-on enterprise development infrastructure.

### **Continuous Monitoring System**

- [ ] **File System Watcher Daemon**:
  - `gnawtreewriter daemon start` - Background monitoring
  - Real-time AST updates from any editor
  - Change event streaming to AI agents
  - Conflict detection and resolution

- [ ] **Intelligent Structural Analysis**:
  - Architectural lint: "Controllers should not access database directly"
  - Security scanning: "Hardcoded secret detected"
  - Performance monitoring: "Large function detected"
  - Cross-project dependency impact analysis

### **Multi-Tenant Cloud Service**

- [ ] **SaaS Platform**: Hosted GnawTreeWriter service
- [ ] **Per-Organization Isolation**: Secure multi-tenancy
- [ ] **Global Policy Enforcement**: Organization-wide rules
- [ ] **Advanced Analytics Dashboard**: Usage, patterns, insights

---

## Implementation Priorities

### Open Source Priorities
1. ✅ Transaction logging and time travel (Phase 1 - DONE)
2. 🔥 **MCP Server** - Run as MCP tool (Phase 2 - NEXT)
3. 🔥 **Local Daemon** - File watching and backup (Phase 2)
4. 🔄 Semantic targeting improvements (Phase 3)
5. 📅 Language expansion (Phase 4)
6. 📅 Infrastructure as Code support (Phase 5)

### Premium Priorities
1. 📅 **Coordination Server** - Self-hosted or SaaS (Premium Phase 1)
2. 📅 Multi-project architecture (Premium Phase 1)
3. 📅 Team synchronization and dashboards (Premium Phase 1)
4. 📅 Cross-project AI intelligence (Premium Phase 2)
5. 📅 Enterprise compliance features (Premium Phase 3)

---

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

**Open Source contributions welcome!** Premium features are developed internally but community feedback shapes priorities.

## Documentation

- [ARCHITECTURE.md](docs/ARCHITECTURE.md) - Technical design
- [FUTURE_CONCEPTS.md](docs/FUTURE_CONCEPTS.md) - Deep dive into planned features
- [LLM_INTEGRATION.md](docs/LLM_INTEGRATION.md) - Guide for AI agents
- [MULTI_AGENT_DEVELOPMENT.md](docs/MULTI_AGENT_DEVELOPMENT.md) - Collaboration strategies
- [AGENTS.md](AGENTS.md) - Contributing as an AI agent (dogfooding)

---

## Recent Progress

### v0.6.0 (2025-01-05)
- ✅ Fixed GitHub Actions CI/CD for ModernBERT
- ✅ All builds pass with `-D warnings`
- ✅ Improved conditional compilation handling
- ✅ Extensive dogfooding - fixes made using GnawTreeWriter!

### v0.5.0 (2025-01-06)
- ✅ ModernBERT AI Integration (semantic search, refactoring, completion)
- ✅ Clone operation for code duplication
- ✅ Zig language support

### v0.4.0 (2025-01-03)
- ✅ Java language support
- ✅ Refactor/Rename command with AST awareness
- ✅ Global `--dry-run` flag

### Phase 1 COMPLETE (2025-12-27)
- ✅ Multi-file time restoration system
- ✅ Transaction logging integrated
- ✅ Complete restoration engine
- ✅ Revolutionary help system
- ✅ AI agent test framework

---

*This roadmap is a living document. Priorities may shift based on community feedback and market needs.*

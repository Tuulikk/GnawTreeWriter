# GnawTreeWriter Roadmap

## Overview

GnawTreeWriter is a tree-based code editor optimized for LLM-assisted editing. This roadmap outlines the evolution from a precise CLI tool to an intelligent agent-integrated platform.

## Current Status: v0.2.1 (Released 2025-12-26)

### ✅ Completed Features

- **Multi-language support**: Python, Rust, TypeScript, PHP, HTML, QML, **Go**.
- **TreeSitter Foundation**: Robust parsing for all core languages.
- **Smart Indentation**: Automatic preservation of code style during insertions.
- **Syntax Validation**: In-memory re-parsing before saving changes.
- **QML Intents**: Dedicated commands for `add-property` and `add-component`.
- **Diff Preview**: Visual unified diff display using the `similar` library.
- **Automatic Backups**: Non-git safety net creating JSON snapshots before every edit.

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
  - **STATUS**: Core implementation complete in `src/core/transaction_log.rs`

- [x] **`undo` & `redo` Commands**:
  - `gnawtreewriter undo [--steps N]` - Reverse N operations (default 1)
  - `gnawtreewriter redo [--steps N]` - Re-apply N reversed operations  
  - `gnawtreewriter history [--format json/table]` - Show operation timeline
  - Navigate backup history without Git dependency
  - Atomic operation reversal: if undo fails, leave system in previous state
  - **STATUS**: Framework complete in `src/core/undo_redo.rs`, CLI commands added

- [x] **Enhanced Restore System**:
  - `gnawtreewriter restore <timestamp|operation-id> [--preview]`
  - `gnawtreewriter list-snapshots [--file path] [--since timestamp]`
  - Point-in-time recovery: "Restore app.py to state at 14:30"
  - Selective restoration: restore individual files or nodes
  - Diff preview before restoration
  - **STATUS**: CLI interface complete, restore logic framework ready

- [ ] **Stable Node Addressing**:
  - Content-based node IDs: `node_abc123def` (hash of node content + position)
  - Graceful fallback to path-based addressing when content changes
  - Cross-edit stability: same logical node keeps same ID across minor edits
  - Migration tool: convert old path-based references to content-based IDs

---

## Phase 2: AI Agent Integration & Intelligence
**Target: v0.4.0 - Q2 2026**

Transform from tool to AI-native development platform.

### **MCP & Agent Integration**

- [ ] **MCP Server Implementation**: 
  - Native Model Context Protocol support as built-in tool
  - Tool definitions for all major operations (edit, analyze, find, restore)
  - Context-aware responses optimized for LLM processing
  - Batch operation support: multiple edits in single MCP call

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

---

## Phase 3: Autonomous Code Guardian 
**Target: v0.5.0 - Q3 2026**

Evolve into always-on development infrastructure.

### **Continuous Monitoring System**

- [ ] **File System Watcher**:
  - `gnawtreewriter daemon start` - Background process monitoring project files
  - Real-time AST updates when files change (even from external tools)
  - Change event streaming to connected AI agents
  - Conflict detection: "File changed outside GnawTreeWriter, merging changes"

- [ ] **Intelligent Structural Analysis**:
  - Architectural lint rules: "Controllers should not directly access database"
  - QML-specific rules: "All Rectangle components must have explicit dimensions"
  - Security scanning: "No hardcoded API keys or secrets detected"
  - Performance warnings: "Large function detected (>100 lines), consider refactoring"

- [ ] **Visual Development Interface**:
  - Web-based AST explorer with real-time updates
  - Interactive tree manipulation: drag-and-drop node reordering
  - Multi-file project view with dependency graphs
  - Collaboration features: share AST views with team members

- [ ] **Workflow Automation Engine**:
  - GnawScript DSL for complex operations:
    ```gnaw
    project.find("*.qml")
      .filter(component="Rectangle") 
      .where(missing="width,height")
      .auto_fix(add_properties=["width: 100", "height: 100"])
    ```
  - Template system: reusable transformation patterns
  - CI/CD integration: automated code quality enforcement

---

## Phase 4: Universal Tree Platform
**Target: v0.6.0 - Q4 2026**

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

- [ ] **Data Structure Manipulation**:
  - JSON/YAML/TOML editing with schema validation
  - Database schema migrations via AST
  - API specification editing (OpenAPI/GraphQL)

### **Enterprise & Cloud Evolution**

- [ ] **Multi-Tenant Cloud Service**:
  - SaaS version with per-organization isolation
  - Enterprise SSO integration
  - Audit logging for compliance (SOX, GDPR)
  - Global policy enforcement across teams

- [ ] **Advanced Policy Engine**:
  - Company-specific coding standards enforcement
  - Architecture decision record (ADR) compliance checking  
  - Automated security vulnerability patching
  - License compliance scanning and management

## Future Horizons (2027+)

### **AI-Native Development Ecosystem**

- [ ] **Autonomous Refactoring Agent**: AI that continuously improves codebase architecture
- [ ] **Cross-Language Translation**: Convert between languages while preserving tree structure
- [ ] **Predictive Code Evolution**: Suggest architectural changes before they become necessary
- [ ] **Natural Language Programming**: "Create a REST API for user management" → Full implementation

### **Integration Ecosystem**

- [ ] **LSP Server**: Universal structured editing for all IDEs
- [ ] **GitHub App**: Automated PR reviews and suggestions
- [ ] **IDE Extensions**: Native plugins for VS Code, IntelliJ, Neovim  
- [ ] **API Gateway**: RESTful API for third-party tool integration

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

## Recent Progress (2025-01-02)

### ✅ **Phase 1 Foundation Complete**
- Transaction logging system implemented
- Undo/redo command framework built
- CLI commands added: `undo`, `redo`, `history`, `restore`, `session-start`, `status`
- Multi-agent development documentation created
- Roadmap expanded with universal tree platform vision

### 🔄 **Next Immediate Steps**
- Integrate transaction logging with existing edit operations
- Complete undo/redo operation logic for all operation types
- Test transaction log persistence and recovery
- Add content-based node ID system
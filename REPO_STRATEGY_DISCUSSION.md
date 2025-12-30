# Repository Structure Decision: All Open vs. Modular Add-ons

## Current State

As of commit 64e875a, GnawTreeWriter has:
- ✅ Core functionality (batch, time travel, tags, etc.) in MAIN repo
- 📝 Documentation created (vision analysis, roadmap, batch usage)
- 🔍 Analysis of "best LLM editor" positioning
- ⚠️ Installation issue on Fedora (GCC crash with tree-sitter-qmljs)
- 🔧 Fix documentation updated (README on English, add-on architecture)

## The Conflict

We have a tension between two important goals:

### Goal 1: All Content Should Be Open & Readable
- Documentation should be in MAIN repo (open source)
- Code should be in MAIN repo (open source)
- Nothing should be closed or proprietary
- Community should be able to inspect, learn, and contribute

### Goal 2: Add-ons Should Be Developable Separately
- Add-ons (LSP, MCP, UI, etc.) should be able to be developed independently
- Different developers should be able to work on specific add-ons
- Main repo should not be blocked by waiting for add-on features
- Integration should be possible (add-ons call into core)

---

## Option A: Monorepo with Git Submodules (RECOMMENDED)

### Structure

```
gnawtreewriter/                 # Main repository (open)
├── src/
│   ├── core/               # Core modules (open)
│   │   ├── batch.rs         # Batch operations (open)
│   │   ├── time_travel.rs    # Time travel (open)
│   │   ├── tags.rs           # Named references (open)
│   │   ├── restoration.rs    # Restoration engine (open)
│   │   ├── session.rs        # Session management (open)
│   │   └── undo_redo.rs      # Undo/redo (open)
│   ├── parsers/             # All parsers (open)
│   └── cli.rs              # Main CLI (open)
├── add-ons/                  # Add-on submodules
│   ├── .gitmodules          # Submodule definitions
│   ├── lsp/                 # LSP add-on (open)
│   ├── mcp/                 # MCP daemon (open)
│   ├── ui/                  # Visualization UI (open)
│   └── refactor/            # Advanced refactoring (open)
├── docs/                   # Documentation (open)
├── examples/                # Example code (open)
└── tests/                   # Tests (open)
```

### How It Works

**Main Repo:**
- Contains ALL core functionality
- Open source, accessible to everyone
- Add-ons referenced via Git submodules

**Add-ons:**
- Each add-on is a separate Git repository
- Can be developed independently
- Can have their own release cycles
- Integrated into main repo via Git submodules

### Example Workflow

```bash
# Developer wants to work on LSP add-on
cd gnawtreewriter/add-ons/lsp
# Make changes
git commit -m "Add hover support"

# Add-on maintainer updates main repo
cd gnawtreewriter
git submodule update --remote --merge
git add add-ons/lsp
git commit -m "Update LSP add-on to version 0.1.0"
git push origin master
```

### Pros

✅ **ALL OPEN**: Every file and add-on is open and accessible
✅ **Independent Development**: Each add-on can evolve on its own schedule
✅ **Clear Boundaries**: Core vs Add-ons clearly separated
✅ **Professional Organization**: Follows industry best practices (Kubernetes, VS Code, etc.)
✅ **Flexibility**: Users can install specific add-ons without others
✅ **Version Independence**: LSP v0.1.0 vs Core v0.4.0 is possible
✅ **Community Friendly**: Easy to fork just the add-on you care about

### Cons

❌ **Git Complexity**: Git submodules can be confusing for new users
❌ **Clone Weight**: Need `--recursive` to get all add-ons
❌ **Setup Friction**: New users must know to run `git submodule update`
❌ **Release Coordination**: Requires coordination between repos for releases
❌ **Documentation Spread**: Docs might be in main or in add-on repos

---

## Option B: Monorepo with Modules in Main Repo (ALTERNATIVE)

### Structure

```
gnawtreewriter/                 # Main repository (open)
├── src/
│   ├── core/               # Core modules (open)
│   ├── lsp/                # LSP module in main (open)
│   ├── mcp/                # MCP module in main (open)
│   ├── ui/                 # UI module in main (open)
│   └── refactor/            # Refactoring module in main (open)
├── docs/                   # Documentation (open)
├── examples/                # Example code (open)
└── tests/                   # Tests (open)
```

### How It Works

**All modules in main repo:**
- Everything is in one place
- Core and add-ons share the same repo
- All open source, all accessible
- Can use same versioning

### Separation Strategy

```rust
// src/core/mod.rs - Only add core modules
pub mod batch;
pub mod time_travel;
pub mod tags;
// DON'T include: pub mod lsp; pub mod mcp; pub mod ui;

// Add-ons can conditionally enable features
#[cfg(feature = "lsp")]
pub mod lsp;

#[cfg(feature = "mcp")]
pub mod mcp;
```

**Cargo.toml:**
```toml
[features]
default = ["core"]

# Add-on features (optional)
lsp = ["dep:lsp-server"]
mcp = ["dep:mcp-daemon"]
ui = ["dep:visualization-ui"]
```

### Pros

✅ **ALL OPEN**: Everything in one accessible place
✅ **Simple Git**: No submodules, just clone and go
✅ **Easy Setup**: `git clone` gets everything
✅ **Shared Versioning**: Core and add-ons versioned together
✅ **Same CI/CD**: One pipeline for everything
✅ **No Clone Weight**: Single `git clone` command

### Cons

❌ **Coupled Releases**: Add-ons blocked by core release cycle
❌ **Bloat**: Main repo becomes very large with many add-ons
❌ **No Independent Evolution**: Add-ons must wait for core releases
❌ **Harder for Community**: Forking entire monorepo is heavy
❌ **Complex CI**: Need to conditionally build different features

---

## Option C: Separate GitHub Orgs (OPEN, BUT DECOUPLED)

### Structure

```
github.com/Tuulikk/                    # Organization
├── gnawtreewriter              # Main repo (open)
│   ├── src/core/            # Core (open)
│   ├── docs/                 # Docs (open)
│   └── ...
├── gnawtreewriter-lsp          # LSP add-on repo (open)
├── gnawtreewriter-mcp          # MCP daemon repo (open)
└── gnawtreewriter-ui            # Visualization UI repo (open)
```

### Pros

✅ **ALL OPEN**: Every repo is open and independent
✅ **Independent Development**: Each add-on is fully independent
✅ **Independent Release Cycles**: LSP v1.0.0 while Core v0.4.0
✅ **Clear Ownership**: Each add-on has its own maintainers
✅ **Selective Installation**: Users install only what they need
✅ **Professional Appearance**: Separate products for each capability

### Cons

❌ **Decoupled Integration**: Add-ons must call into main repo code (complex)
❌ **Fragmented Documentation**: Docs scattered across multiple repos
❌ **Version Conflicts**: Add-on versions might not align with core
❌ **Integration Overhead**: Need to maintain API compatibility
❌ **User Friction**: Users must clone/install multiple repos

---

## Recommendation

### For Current Phase (v0.4.0 → v0.5.0): **OPTION B (Modules in Main)**

**Why Option B is best right now:**

1. **Simplicity**: Single repo, everything together
2. **All Open**: No submodules, no separate repos
3. **Easy for Users**: One `git clone` gets everything
4. **No Setup Friction**: No `git submodule update` commands
5. **Early Stage**: Add-ons are experimental, better in main repo
6. **Fast Iteration**: Can modify core and add-ons together
7. **Less Coordination**: Single PR/issue tracker

**But we prepare for future:**

- Design clean integration points for when add-ons mature
- Document add-on architecture in ROADMAP.md
- Create `ADD_ON_INTEGRATION.md` guide for future modularization

### For Mature Phase (v1.0.0+): **OPTION A (Git Submodules)**

**Why Option A becomes best later:**

1. **Large Scale**: LSP, MCP, UI become mature, large codebases
2. **Independent Teams**: Different teams can own different add-ons
3. **Version Independence**: Add-ons can release frequently without core
4. **Community Contributions**: Easier to contribute to specific add-on
5. **Professional Appearance**: Separate repos for each add-on (LSP, MCP, UI)
6. **Best Practices**: Follows Kubernetes, VS Code patterns

---

## Summary Table

| Aspect | Option A (Submodules) | Option B (Modules) | Option C (Separate Orgs) |
|---------|------------------------|-------------------|-----------------------|
| All Open | ✅ Yes | ✅ Yes | ✅ Yes |
| Independent Dev | ✅ Yes | ❌ No | ✅ Yes |
| Easy Setup | ❌ Moderate | ✅ Yes | ✅ Yes |
| Simple Git | ❌ Submodules | ✅ Single repo | ✅ Multiple clones |
| Professional | ✅ Yes | ❌ Bloat | ✅ Yes |
| Future Ready | ✅ Yes | ✅ Moderate | ❌ Yes |

---

## Our Decision

### Recommended: **OPTION B (Modules in Main)** for v0.4.0 → v0.5.0

**Reasoning:**
- We're in early stage, add-ons are experimental
- Simplicity is more valuable right now
- We want to encourage community contributions
- We want to keep code accessible and easy to explore
- We can modularize to Option A later when scale demands it

**Transition Path:**
- Keep everything in main repo for now
- Use conditional compilation (`#[cfg(feature = "lsp")]`)
- Document architecture clearly in ROADMAP.md
- Design integration points for future separation

### Implementation Steps

1. **Create add-on directories**:
   ```
   mkdir -p src/lsp src/mcp src/ui
   touch src/lsp/mod.rs src/mcp/mod.rs src/ui/mod.rs
   ```

2. **Implement basic integration points**:
   ```rust
   // In src/core/mod.rs
   #[cfg(feature = "lsp")]
   pub use lsp::LspIntegration;
   ```

3. **Update Cargo.toml**:
   ```toml
   [features]
   default = ["core"]
   lsp = []
   mcp = []
   ui = []
   ```

4. **Document development process**:
   - Create `docs/ADD_ON_DEVELOPMENT.md`
   - Explain how to contribute add-ons
   - Define integration APIs

5. **Update ROADMAP.md**:
   - Add Option B as current strategy
   - Keep Option A as future plan (v1.0.0+)

---

## Next Actions

1. Review this document
2. Decide: Option A, B, or C?
3. Update ROADMAP.md with chosen strategy
4. Commit and push all documentation (vision, roadmap, batch, add-on strategy)
5. Create GitHub issue referencing installation fix and add-on architecture

---

**For users**: Everything remains open, accessible, and freely available.

**For developers**: Clear path for contributing add-ons in modular way (in future).

**For the project**: Scalable architecture that can grow from simple to complex while staying open and accessible.

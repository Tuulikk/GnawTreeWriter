# ALF: Agentic Logging Framework

This file serves as the **GnawTreeWriter Project Journal**, following the **ALF** standard for agentic discovery and temporal intent history.

## [2026-01-12]

### [2026-01-12 23:45:00] | ID: bugfixes | Author: Gemini-Agent
- **[Intent]**: Fix critical architectural bugs identified by OpenCode/GLM 4.7.
- **[Op]**: 
  - Refactored `edit_node_at_path` to use line indices instead of `replacen()`.
  - Fixed `Rename` preview count logic.
  - Added filtering/limiting to `search` command.
- **[Outcome]**: v0.6.11 hardened. Core editing is now collision-safe.
- **[Discovery]**: Using string search (`replacen`) for AST nodes is dangerous if the same snippet exists multiple times. Line-based replacement is mandatory for surgical precision.
- **[Label]**: `core` -> `fix:surgical-precision`, `cli` -> `feature:search-filtering`

---
### [2026-01-12 23:15:00] | ID: opencode | Author: Gemini-Agent (via OpenCode)
- **[Intent]**: Cleanup and synchronize the help system (`examples` and `wizard`) with current CLI capabilities.
- **[Op]**: Updated `src/cli.rs` documentation strings and examples.
- **[Outcome]**: v0.6.11 released. The help system is now 100% accurate.
- **[Discovery]**: The `quick` command reference was persistent in examples despite being renamed to `quick-replace` long ago. Documentation debt is now cleared.
- **[Label]**: `cli` -> `docs:synchronized`

---
### [2026-01-12 22:30:00] | ID: parity | Author: Gemini-Agent
- **[Intent]**: Achieve full parity between CLI and MCP tools based on OpenCode/GLM 4.7 bug reports.
- **[Op]**: 
  - Added `search`, `skeleton`, and `semantic-report` subcommands to CLI.
  - Corrected `handle_examples` for the `ai` topic.
  - Silenced unused field warnings in `AiManager`.
- **[Outcome]**: v0.6.10 released. All 14 core tools are now available in both CLI and MCP.
- **[Discovery]**: Regular stress-testing by other agents (OpenCode/GLM) is invaluable for finding documentation drift and tool-parity issues.
- **[Label]**: `cli` -> `parity:mcp-full`, `docs` -> `cleanup:completed`

---
### [2026-01-12 21:30:00] | ID: semantic | Author: Gemini-Agent
- **[Intent]**: Implement Semantic Selection to allow targeting nodes by name (@fn:name) instead of numeric paths.
- **[Op]**: 
  - Updated `TreeNode` with `get_name()` logic.
  - Implemented `resolve_path` in `GnawTreeWriter` core.
  - Added `Read` command and enhanced `List` command in CLI.
  - Dogfooding: Refactored `src/mcp/mod.rs` using semantic selection to remove redundant logic.
- **[Outcome]**: Successfully implemented and verified via self-editing. v0.6.9 released.
- **[Discovery]**: Semantic targeting dramatically reduces cognitive load for agents and prevents "off-by-one" node errors. Shell escaping remains a challenge; STDIN/Source files are the preferred injection methods.
- **[Label]**: `core` -> `feature:semantic-selection`, `cli` -> `feature:read-command`

---
### [2026-01-12 10:15:00] | ID: boot | Author: Gemini-Agent
- **[Intent]**: Setup the infrastructure for the "Gnaw Endurance Test" and establish the ALF standard.
- **[Op]**: `write_file AI_AGENT_TEST_SCENARIOS.md`, `write_file ALF.md`
- **[Outcome]**: Successfully established test scenarios and journaling format.
- **[Discovery]**: The ALF standard (Agentic Logging Framework) provides a Git-safe way for multiple agents to share intent and discoveries without token-heavy session scanning.
- **[Label]**: `root` -> `meta:testing-active`, `meta:alf-standard`

---
### [2026-01-12 10:45:00] | ID: rebrand | Author: Gemini-Agent
- **[Intent]**: Rebrand G-LOG to ALF (Agentic Logging Framework) for better naming uniqueness.
- **[Op]**: `mv GNAW_TEST_REPORTS.md ALF.md`, updated `ROADMAP.md`.
- **[Outcome]**: ALF is now the official project journaling standard.
- **[Label]**: `docs" -> `meta:alf-branded`

---
*Next agent: Append your reports above this line using the standard header.*
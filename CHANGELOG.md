## [0.15.0] - 2026-10-05

### Added
- **MCP `index_project` tool** (Motor2 bug 2): builds the SEMANTIC/vector
  index satellite `sense` searches — the MCP side of `ai index`, same
  pipeline, same GPU/20%-VRAM gate. Background runs (`{action: start}` /
  `status`) so calls never outrun client timeouts; single-run guard via
  compare_exchange. `index_entities`/`index_relations` descriptions now
  state they build the knowledge graph, NOT this index; the satellite
  no-matches error points at the tool first.
- **MCP `history` + `stats` tools** (Motor2 plan #24): transaction log
  newest-first with stable ids (empty log = normal state with setup
  guidance) and full ProjectStats with a one-line summary.
- **Semantic edit transparency** (Motor2 plan #19): `semantic_edit` hit
  returns `semantic_match {matched_path, confidence, candidates[5]}` on
  the result AND in the text channel; a miss attaches explicit zero
  confidence + empty candidates instead of a bare denial.
  `semantic_insert` success adds structured
  `{anchor_path, confidence, parent_path, position}`.
- **Stable error codes** (Motor2 plan #25): `tool_error_code` puts
  `code` next to the human text — `E_FILE_NOT_FOUND`, `E_STRICT_PARSE`,
  `E_NODE_NOT_FOUND`, `E_EDIT_REJECTED`, `E_VALIDATION`,
  `E_SEMANTIC_NO_MATCH`, `E_MODEL_UNAVAILABLE`, `E_RULE_REJECTED`,
  `E_BATCH_ROLLED_BACK` (~40 call sites). Registry:
  `docs/ERROR_CODES.md`, kept in sync mechanically by
  `integration_error_codes_documented` (both directions).
- **Duplex metrics** (Motor2 plan #23): `proposed`/`validated`/
  `rejected`/`applied` counters persisted at
  `.gnawtreewriter_metrics.json` from edit_node/insert/edit_ask/
  semantic_insert outcomes (move/batch keep their own transactions —
  documented scope).
- **Retry-with-feedback** (Motor2 plan #23): `edit_ask` feeds the AST
  validation error back to the model for ONE repair round; success is
  flagged `retried: true` with `first_error`, double failure reports
  both attempts.
- **Partial-grace parsing for READ paths** (Motor2 brief): new
  `ParserEngine::parse_lenient` (default = strict; Rust overrides via
  tree-sitter's error-tolerant tree) + `GnawTreeWriter::new_lenient`.
  `analyze`/`get_skeleton`/`list_nodes`/`search_nodes`/`read_node`
  now answer broken-but-largely-valid files with the partial tree and
  an explicit `syntax_warning` (text + structured), while EDITORS stay
  strict — same file gets `E_STRICT_PARSE` with a pointer to the read
  paths, zero bytes written. Contract test:
  `integration_mcp_partial_parse_read_paths`.

### Fixed
- Strict parse refusals were coded `E_FILE_NOT_FOUND` (misleading
  aggregation — syntax errors would count as missing files); split via
  `open_error` into `E_STRICT_PARSE` vs `E_FILE_NOT_FOUND`.
- `preview_edit` validation failure carried a bogus "sense failed"
  guidance — now `E_VALIDATION` with preview-specific next steps.
- `test_debug_mcp_relations` sanity cap made size-relative (fixed 200
  broke as mcp/mod.rs grew with real features).

### Docs
- `docs/ERROR_CODES.md` (new): registry table with meanings and next
  steps; sync-tested against `tool_error_code` call sites.

## [0.14.0] - 2026-10-05

### Added
- **Satellite sense ranking quality**: `rerank_satellite` in front of raw
  cosine. Declarations (module/use/attribute heads) are demoted with a
  penalty scaled by query-term overlap, implementations get a nudge, and
  lexical matches count across preview + file/node paths; `(Chunk N)`
  prefixes are classified on the real code head. Raw window widened
  (2000 @ floor 0.1) so long, diffuse implementations are even *visible*
  to the reranker, then trimmed to 10. Inventory-intent queries ("which
  modules exist…") flip the priors: declarations ARE the answer and get
  the item bonus instead of the penalty. Result on the reference query
  "how is undo implemented…": real `impl`/`fn` nodes on top — before,
  all ten hits were module declarations and `#[command]` attributes.
- **Batched embeddings** (`ModernBertModel::get_embeddings`): one
  forward per window of up to 16 texts with padded masked mean-pooling,
  order-preserving, single row exactly equivalent to `get_embedding`.
  Wired into the project indexer (two-phase: walk collects
  (path, preview, text) triples, then one batched call per file) and
  both zoom-JIT node loops. Forward passes go from N to ~N/16.

### Fixed
- **CUDA OOM during GPU indexing**: a flat padded batch of long
  definition bodies (≤8000 chars each) blew VRAM at B=16. Batching is
  now budget-aware — greedy consecutive windows keep `B × max_len² ≤
  2^22`, so short rows (the common case) stay batched while giant rows
  degenerate to B=1, the sequential path that already ran fine on GPU.
  Full reindex after the fix: **223 files in 90 s, exit 0** (RTX 4070
  SUPER, 20%-VRAM gate passed at 28% free).

### Changed
- `#[ignore]` on the heavy ModernBERT equivalence test (debug-mode
  forwards ~2-3 min): run explicitly with
  `cargo test --test ai_modernbert_tests -- --ignored --nocapture`
  when touching `get_embeddings` — it also prints sequential-vs-batch
  timing. Short-text debug micro-bench shows 1.1-1.2× (debug overhead
  dominates); the structural win is fewer forwards + launch reduction
  + long-row OOM safety.

### Docs
- Issue log: reply to the Motor2 2026-10-05 entry (root cause of bug 1:
  tree-sitter-rust 0.24.0 treated plain `&raw` borrows as Rust 1.82
  `&raw const/mut` starts — fixed by 0.24.2, committed in 6413302;
  bug 2 confirmed as two-separate-indexes gap, MCP index tool pending).

## [0.13.0] - 2026-10-04

### Added
- **SKILL.md rewritten for agents** (ROADMAP 9.6): situations-table (16 rows —
  situation → MCP tool → complete example call), the fallback rule (one timeout
  → one detour → log to `GTW_MCP_ISSUE_LOG.md` → verify bytes on suspicious
  success, never `git checkout` around GTW), per-tool example calls for the
  core MCP set, and `doctor`-based status instead of a hardcoded version claim.
- **AGENTS.md "För agenter som ANVÄNDER GTW" section**: six-step diagnostic
  chain, the issue-log note taxonomy (7 categories — every closed entry
  corresponds to a real fix), and the escalation rule.
- **Error-guidance contract test** `integration_error_strings_carry_guidance`:
  bans raw `e.to_string()` passthrough, bare feature-gate strings, unguided
  IO errors, and the historic stub lies from live (non-comment) code — the
  "no raw error strings" rule can no longer silently regress.

### Changed
- **Every `tool_error` in the MCP server now carries next-step guidance**
  (~30 sites): IO errors point at `search_nodes`/`analyze`, feature gates at
  the rebuild command, model failures at `ai setup`/`ai status` with a
  search_nodes/grep fallback, edit rejections at `list_nodes`+`preview_edit`,
  rule failures at `rules list`. Zero raw passthroughs remain.

### Fixed
- SKILL.md contained the forbidden batch format (`search`/`replace` instead of
  `{file, path, content}` — finding #13's spec lie), `gnawtreewolf` typos (×2),
  and a stale hardcoded version claim.

### Docs
- ROADMAP 9.6 complete. Roadmap Phase 9 fully done (9.1–9.6).

## [0.12.0] - 2026-10-04

### Added
- **MCP `doctor` tool** (ROADMAP 9.5): one-call health diagnostic — parser
  smokes, backup integrity, transaction log → `{healthy, passed, failed,
  warnings, total, checks[]}` with guided text. Shares `run_full_doctor`
  with the CLI (`gnawtreewriter doctor`) so the two can never diverge
  (same pattern as `run_lint`).
- **Lint rule `rust_string_byte_slice`** (`$X[..$Y]` + fix template
  `$X.get(..$Y).unwrap_or($X)`) — the byte-slice class left over from the
  0.9.8 UTF-8 audit, smoke-proven against real hits in `xml.rs`/`qml.rs`
  and through `lint --fix --preview`.
- **`$X`-binding interpolation in rule messages** (`interpolate_message`):
  findings read "… if `code` is a String/&str" instead of the raw template;
  unbound names stay literal (messages never fail).

### Fixed
- **Finding #14: `quick-replace` could report "✓ applied" while changing
  zero bytes** — the exact lie the tool must never tell. A replace now
  fails loudly when the search text is not found (or the replacement is
  identical), with next-step guidance ("re-read the file after cargo fmt
  and retry"). No bytes are written on a no-match; preview fails the same
  way. Two regression tests.
- **`get_skeleton` could answer with a bare header** (issue-log triage):
  the skeleton now rides in `content.text`, `truncated` is an explicit
  flag at the 500-node cap, and an empty skeleton is a guided error —
  never a silent empty success. Contract test added.
- **`sense` zoom/satellite returned header-only text** (issue-log FIX
  prescription): top matches now appear in the text channel
  (`"Zoom …: 5 node(s) — 106 (0.81) …"`); empty zoom returns a guided
  error instead of a naked label.
- **Unknown `--rule` id passed silently as "No issues"** — now fails
  loudly with the loaded-rule count and pointers (`rules list`,
  `rules add`).
- **`doctor` took 80 s in backup-heavy projects**: `check_backups` now
  light-scans (count by filename, validate the 3 newest by mtime) instead
  of fully parsing every backup JSON (2 GB/128 files here); parser smokes
  run in parallel. Doctor: 80 s → 0.5 s.

### Changed
- CLI `doctor` section headers collapsed into one line (checks unchanged);
  internals delegated to the shared `run_full_doctor`.

### Docs
- ROADMAP 9.5 complete (all items, with implementation notes).
- `GTW_MCP_ISSUE_LOG.md`: all three 2026-09-30/10-01 triage checklists
  closed with verdicts; zoom/satellite text-channel FIX marked done;
  open remainder documented (satellite index-build lacks an MCP tool —
  known gap, outside 9.5 scope).

## [0.11.0] - 2026-10-04

### Added
- **GPU-accelerated indexing — opt-in, conservative by default**: GnawTreeWriter
  never touches the GPU unless told to (the tool was designed CPU-only and stays
  that way for users). Two ways to opt in:
  - persistent: `indexing.device: auto` in the new optional `gnawtreewriter.yaml`
    (values `auto`|`cpu`|`cuda`; absent file or setting = CPU)
  - one-off: `gnawtreewriter ai index --gpu`
- **20%-VRAM safety gate** (`decide_index_device` in ai_manager): `auto` uses the
  GPU only when the binary was built with `--features cuda` **and** ≥20% of VRAM
  is free — never squeezes the OS or co-resident GPU users (verified live against
  a co-resident llama-server). Unknown/invalid config values fail safe to CPU.
  Every decision is logged with its reason (guided principle: never silently
  degrade). Query paths (sense/zoom/MCP) always stay CPU by design.
- **`scripts/build-gpu.sh`** — distribution-friendly GPU build: compiles
  `--features cuda` inside an `nvidia/cuda` container (host needs only podman +
  NVIDIA driver — **no CUDA toolkit**, which Fedora does not ship), auto-detects
  compute capability from the host's `nvidia-smi`, ships exactly the CUDA
  runtime libs the host lacks (driver libs correctly left to the host), and
  installs a wrapper + binary. One command, idempotent, cached volumes for
  rebuilds (~2 min warm).
- **Docs**: AGENTS.md "GPU indexing (opt-in; devs recommended)" section — the
  recommendation for developer machines is to change the one flag in the file;
  README "GPU-accelerated indexing (optional, off by default)" section with the
  same three-line setup.

### Fixed
- **Finding #14 (GTW tool bug, logged in GTW_MCP_ISSUE_LOG.md)**: `quick-replace`
  reported "✓ applied" while changing zero bytes (oldString didn't match
  post-`cargo fmt` text) — caught by failing tests. A replace must change bytes
  or fail loudly with guidance ("search text not found — re-read the file after
  cargo fmt and retry"), never fake success.

### Docs
- `GTW_MCP_ISSUE_LOG.md`: GPU-entry + finding #14.

## [0.10.0] - 2026-10-04

### Added
- **MCP `lint` tool** (ROADMAP 9.1): AST pattern linting through MCP with full schema — paths, `recursive`, severity/rule filters, and ad-hoc `$X` pattern search (`pattern` + `language`). Shared core `rules::run_lint` with the CLI so the two never diverge; report-only, never writes. CLI gained parity flags `--pattern`/`--language`.
- **`fix:` rule support** (ROADMAP 9.2): rules may carry a rewrite template (`fix: "$X.expect(\"msg\")"` in YAML, `--fix` on `rules add`, `fix` arg on MCP `add_rule`). Templates expand with `$X` bindings from the match, are validated before saving (`validate_fix`: identifier substitution, scaffold fallback `fn probe() { … }` for expression forms like `?`-chains — unknown languages fall through to the generic parser, matching `compile_rule`), and apply through `lint --fix` (CLI) / `lint {fix: true}` (MCP) as an **atomic batch** — Edit-op per finding, node content replaced in place so paths don't shift, dedup per file+path. `--fix --preview` shows the diff and writes nothing. Writes only ever happen behind the explicit flag; `LintResult.rules` carries the effective rule set. `py_bare_except` ships with a builtin fix.
- **MCP adoption contract** (ROADMAP 9.3): all 30 tools now carry self-teaching descriptions (VAD + NÄR vs Read/Grep/Edit + RETURNERAR + EXEMPEL, ≥100 chars) and complete `inputSchema`s. Schema lies fixed against the actual dispatch: `list_nodes` exposes `filter`/`max_depth`, `explain`/`edit_ask`/`investigate` gained `required`, `index_entities`/`index_relations` gained `anyOf` (file_path | file_paths), `save_state` is honestly zero-arg. New `integration_mcp_tools_adoption_contract` test mechanically pins the catalog: unique names, description length, `required ⊆ properties`, honest zero-arg, ≥30 tools, priority tools (`batch`, `edit_node`, `insert_node`, `search_nodes`, `sense`, `semantic_edit`) with real schemas — documentation drift can no longer regress silently.
- **Source citations (`sources`) in LLM answers** (ROADMAP 9.4): `get_semantic_report` returns `sources: [{file, node_path}]` (one provenance pointer per finding, deduped); `investigate` returns `sources: [{file}]` for the evidence the answer was actually synthesized from. Pure payload builders (`semantic_report_payload`, `investigate_payload`) make the contract testable without a model — `integration_sources_semantic_report_payload` + `integration_sources_investigate_payload` (mamba) plus 6 unit tests.
- **`FileMatch.content_preview`**: satellite `sense`/`search_semantic` matches now include a `preview` straight from the index — score + content in one response saves an extra `read_node` round-trip.

### Fixed
- **"Resurrection" mystery root-caused** (GTW_MCP_ISSUE_LOG.md finding 12): the repeated silent reversion of recent edits (8 victims across `rules.rs`/`cli.rs`) was never a cache bug — `integration_mcp_tools_call_undo` called the MCP `undo` tool against the **real repo transaction log** on every `cargo test` (the server takes `project_root` from CWD), replaying the latest GTW transactions onto source files. New `serve_with_shutdown_root(listener, token, project_root, shutdown)` lets tests bind an explicit project root; the undo test now runs against a throwaway temp project and asserts the deterministic "Nothing to undo". Regression-proofed: two consecutive full-suite runs with unchanged file checksums.
- **Batch test used a nonexistent spec format** (finding 13): `integration_mcp_tools_call_batch` wrote `replace` instead of the real `BatchFile`/`BatchOp` format (`{file, path, content}`) and could never pass against a real parser — masked by the undo-test chaos. Fixed with root-node replacement.
- **`--features mamba` did not compile**: a mamba-gated `Rule` initializer in `cli.rs` predated the 9.2 `fix` field. Now parses `fix` from the rule source (parity with `rules add`).
- MCP handler signature/docs now match dispatch for `explain` (requires `file_path`), `edit_ask` (requires `file_path` + `request`), `investigate` (requires `question`).

### Changed
- `undo` MCP tool documents the transaction-log contract and points at `gnawtreewriter history` / `restore-project` for failed reverts; `batch` tool description covers atomicity + preview semantics.

### Docs
- `GTW_INSTRUCTIONS.md` regenerated: all 30 tools in a category table + the diagnostic chain (`analyze`/`get_skeleton` → `sense`/`search_nodes` → `read_node` → `edit_node`/`semantic_edit` → verify → `undo`) — the previous version had drifted to 17 tools.
- `GTW_MCP_ISSUE_LOG.md`: 9.1-continuation entry (findings 1–8), 9.2 entry (findings 9–11), 9.3 entry (findings 12–13, root-cause reclassification), 9.4 live-verification entry.
- `ROADMAP.md`: Phase 9 items 9.1–9.4 marked complete with implementation notes.
- `README.md`: Rules Engine documents `fix:` templates and `lint --fix [--preview]`; MCP tool lists include `lint`.

## [0.9.8] - 2026-10-01

### Fixed
- **MCP reliability on large files** (found via GTW_MCP_ISSUE_LOG.md omgång 3-5): `sense`/`get_skeleton` on 1000+ node files completed within client timeouts instead of cascading into timeouts and a dead "Not connected" connection.
- **Per-request panic isolation** in both stdio and HTTP MCP loops: a panicking handler now returns a JSON-RPC internal error instead of unwinding through the read loop and killing the connection.
- **Shared GnawSense broker** (`AppState` + `OnceCell`): the ModernBERT model and JIT index cache now persist across MCP calls instead of reloading per call. First zoom-sense ~25-30 s, following calls ~1 s (cache hit).
- **Char-boundary panics on multibyte UTF-8** (em-dash, CJK, emoji) — byte slicing (`&s[..97]`, `[..3000]`, byte-based `chunk_text`) replaced with char-safe truncation across sense previews, pipeline evidence caps, project indexer chunking, `edit` removed-preview and secrets redaction. Systematic UTF-8 audit; a `no_byte_slice_strings` lint rule was added to `gnawtreewriter.rules.yaml`.
- **ModernBERT rope crash on huge nodes** (content > 8192 tokens crashed the model with "inconsistent last dim size in rope") — node content truncated before embedding.
- **Degenerate `Satelite search results` response** — now reports match count and hints at building the project index when empty.
- **Bounded JIT cache** (32 files with eviction) — unbounded growth reached 511 MB RSS in long-lived MCP sessions.
- clippy `question_mark` warnings in `parser/xml.rs`.

### Changed
- Zoom indexing caps for first-call latency: max 24 embedded nodes per file (largest definitions first), max 1200 chars per node. Zoom remains a localization tool; the full project index is built with `ai index`.
- `project_indexer` chunk thresholds made rope-safe (chunk at 4000 chars from 8000, was 10000 from 15000 — CJK can cost ~1 token/char).

### Docs
- `GTW_MCP_ISSUE_LOG.md`: omgång 3 (reproduction: timeout → Not connected, degenerate satellite response), omgång 4 (root causes + fixes + validation), omgång 5 (UTF-8 audit: 7 more byte-slice bugs found and fixed).

## [0.9.7] - 2026-08-25

### Added
- **Rules engine (`lint --rules`)**: semgrep-like pattern matching against ASTs (spec: docs/RULES_ENGINE_SPEC.md).
  - YAML rules with `$X` placeholders, compiled to structural AST matching in Rust
  - Builtin rules (rust_unwrap, rust_self_assignment, py_bare_except, py_eval, js_console_log)
  - Project rules auto-loaded from `gnawtreewriter.rules.yaml`
  - `--rules <file>`, `--severity`, `--rule <id>` filters, JSON output
  - `lint` is now real lint (structural rules), not just a parse check
- **Rules guardian on edit (Duplex Loop 2.0)**: after validation, builtin rules run on the new code — error-severity findings block the edit (unless `--force`), warnings are printed but allowed. From "does it parse?" to "is it good code?".
- **Rule annotations in `edit --ask` prompts**: known rule violations in the file are injected into the LLM prompt so the model can avoid introducing/worsening them (rule-injected expertise). `edit --ask` switched to a line-based proposal format (`{"line": N, "new": "..."}`) — far more reliable for a small model than JSON-escaping whole code snippets.
- **Agent-written rules (`rules add` + MCP `add_rule`)**: validate and append a semgrep-like rule to `gnawtreewriter.rules.yaml`. The pattern must compile for the language; invalid rules are rejected. Agents can now write rules to GTW through a tool.
- **`lint --discover`**: the local LFM2.5 model proposes project-specific rules from linted files; each proposal is validated (must compile and match ≥1 file) before saving. Few-shot examples in the prompt make the model produce valid patterns.
- **Multi-edit (`edit --ask "..." --all`)**: the model proposes a change for one occurrence; GTW applies the same replacement to every identical line/occurrence in the file, validating the whole file reparses before writing. Step 5: rule-guided consistent edits.
- **Docs**: README "Rules Engine" section, ROADMAP Phase 8 updated (all 5 rule steps + LLM missions marked done), RULES_ENGINE_SPEC.md marked implemented.
- **Rule library expanded**: 19 builtin rules across rust (7), python (4), javascript (4), typescript (2) — original formulations of well-known anti-patterns (not copied from semgrep-rules; the license forbids redistributing them). Written with `$X` placeholders for the structural matcher.
- **30 more rules added** across go (9), java (10), c (9): self-assignment, error handling, memory safety, type safety, and concurrency anti-patterns. Builtin library now totals 46 rules across 7 languages.
- **40+ more rules added** across all 7 languages, bringing the builtin library to ~70 rules (10 per language): raise/assert/imports for Python, alert/prompt/setTimeout/XHR/addEventListener for JS, catch unknown/undefined check/type assertion for TS, collect/magic number/slice clone for Rust, make/for-loop/defer for Go, Object/parseInt/synchronized for Java, malloc/free/sprintf/goto for C.
- **Duplicate-finding fix**: `run_rule` now deduplicates findings by code location `(line, column)`, so a statement and its inner child matching the same pattern are reported once, not several times. Verified: bare `except: pass` reports once; separate `unwrap()` calls keep their own findings.
- **Local LLM command extension (LFM2.5-1.2B, Q4)** behind the `mamba` feature:
  - `explain <file> [--node <path>]` — plain-language explanation of a code node
  - `summarize <dir>` — hierarchical AST-skeleton map-reduce summary
  - `investigate "question"` — query expansion → index search → ranked answer with file references
  - All three available as MCP tools (`explain`, `summarize`, `investigate`)
  - `ai calibrate` — measures this machine's inference speed and saves a timing profile
- **`--resolution` flag** (fast / balanced / thorough) on explain/summarize/investigate: trades speed vs detail via chunk size and output budgets.
- **Token & time transparency**: every command reports a budget (`expected/actual tokens`, `calls`, `estimated/actual seconds`, truncation warning). No silent cut-offs.
- **`edit --ask "request"`** (+ MCP `edit_ask`): the local model proposes a minimal old→new change; GTW finds the containing AST node, applies the replacement inside it, and validates through the Duplex Loop before apply. The AST places, the model states intent — a small model stays viable because it never needs mechanical precision.
- **ROADMAP.md**: updated to current status (v0.9.7) with a new Phase 8 (Local LLM Command Extension) section and the planned LLM missions.

### Performance notes
- LFM2.5 prefill in candle is ~O(seq²), so pipeline steps chunk to small sizes and feed compact AST skeletons instead of raw source (~98% smaller) — summarize went from >3 min/file to ~15s/file.

## [0.9.6] - 2026-08-24

### Added
- **`explore` command**: Map-like navigation with 4 zoom levels (overview / directory / file / full), each node carrying token counts and drill-down hints.
- **Session-level parse cache** (`parse_cache.rs`): Files parsed once are reused across tool calls in the same session — no redundant AST parses.
- **`pack --compress-threshold N`**: Compress only files larger than N tokens; 0 (default) compresses everything.
- **`scripts/benchmark_ai.sh`**: Reproducible benchmark for explore/pack/index/state operations.

### Changed
- **Parallel processing (rayon)**: pack, explore (directory level), and MCP index batch handlers now run across all CPU cores. Output order is preserved — results remain byte-identical to the sequential versions.

### Performance (measured on own codebase, 79 files / 169k tokens)
| Operation | Before | After |
|---|---|---|
| pack --compress | 676 ms | 335 ms (-50%) |
| explore directory (level 1) | 318 ms | 130 ms (-59%) |
| index_entities (20 files) | 177 ms | 107 ms (-40%) |
| index_relations (20 files) | 84 ms | 40 ms (-52%) |

## [0.9.5] - 2026-08-23

### Added
- **AI-Friendly Context Tools** (inspired by Repomix analysis):
  - `compress` command: Replace function bodies with `⋮----` placeholders (~70% token reduction)
  - `pack` command: Package entire project into AI-optimized format (markdown/json/plain)
  - `curate` command: Intelligent file selection based on task description
  - Token counting in `analyze` output (estimated_tokens field)
  - Secret detection and redaction (18 patterns: AWS, GitHub, GitLab, Stripe, JWT, etc.)
  - Git-aware file walking (respects .gitignore, replaces hardcoded skip-lists)

- **New MCP tools**: `compress`, `pack`, `curate`

- **New modules**:
  - `file_walker.rs`: Git-aware traversal using `ignore` crate
  - `token_count.rs`: Heuristic token estimation for LLM context planning
  - `compress.rs`: AST-based code compression
  - `pack.rs`: Project packaging with token budgets
  - `secrets.rs`: Credential detection and redaction
  - `curator.rs`: Multi-strategy context curation (relevance, git changes, dependencies)

### Changed
- Replaced hardcoded skip-lists in `gnaw_find`, `blast`, `inspect`, `gnaw_refactor`, `project_indexer`, `relational_index` with unified git-aware walker

## [0.9.4] - 2026-04-30

### Added
- **`multi-replace` command**: Multiple search/replace pairs in one pass
  - Supports STDIN (`--pairs -`), inline JSON, or file path
  - Single file read, single backup, single transaction
  - Atomic: all-or-nothing validation
  - Auto-unescape of `\n`/`\t` in replacement text
  - Example: `echo '[{"search":"a","replace":"b"}]' | gtw multi-replace file.rs --pairs -`

- **Batch STDIN support**: `batch -` reads JSON from STDIN
  - No temp file needed for pipelining
  - Example: `cat ops.json | gtw batch - --preview`

### Documentation
- **SKILL.md**: Added Quick STDIN Reference table at top
- **SKILL.md**: Added "Agent Workflows" section with 10 patterns
- **GTW_AGENT_COOKBOOK.md**: New 287-line cookbook with 10 recipes for AI agents

## [0.9.3] - 2026-04-27

### Added
- **GnawSense Semantic Navigation** (AI-powered code search & insertion):
  - `sense` command: search code by meaning using local ModernBERT model
  - `sense-insert` command: insert code at semantically located anchors
  - All 4 intents supported: `after`, `before`, `inside`, `replace`
  - `--auto-index` flag: skip interactive prompt for AI agents/CI
  - `GNAW_JSON=1` environment variable for machine-readable output
  - Confidence threshold: filters < 0.2, warns < 0.5
  - Multi-language `extract_name_from_preview`: 15+ patterns (Rust, Python, Go, JS, Java, C, QML)
  - Standardized `err_modernbert_disabled()` helper with JSON support

### Fixed
- **quick-replace literal \n bug**: Auto-detects literal `\n`/`\t` in replacement text and converts to real newlines/tabs (prevented broken file writes from CLI escaping)
- **sense-insert position logic**: Fixed off-by-one in `get_next_index()` — was `idx+3+1`, now `idx+3`
- **Compiler warnings**: Eliminated all warnings (unused variable, dead code)

### Changed
- **Performance**: Model caching with `OnceLock<ModernBertModel>` — loads once, reuses across calls
- **Performance**: JIT file index cache with content-hash invalidation in `GnawSenseBroker`
- **SemanticIndex** now derives `Clone` for caching support

### Documentation
- GnawSense SKILL.md for AI agents at `~/.pi/agent/skills/gnaw-sense/`
- ROADMAP.md: detailed 5-tier GnawSense improvement plan with measured baselines
# Changelog

All notable changes to GnawTreeWriter.

## [0.9.2] - 2026-02-05

### Fixed
- **Critical Compilation Errors**: Fixed 56+ syntax errors in `src/cli.rs` caused by unescaped double quotes in help text examples. All println! statements containing nested quotes have been properly escaped.
- **Build System**: Restored compilation on Rust stable by fixing string literal syntax issues.

### Technical Details
- Problem: String literals like `println!("text "quote" more")` were interpreted as separate tokens
- Solution: Escaped all nested quotes as `println!("text \"quote\" more")`
- Affected: Help text in examples subcommand covering editing, search, restoration, and AI features
- Lines modified: ~80 println! statements across 30+ example categories

## [0.9.1] - 2026-02-04

### Added
- **Surgical Inline Editing**: Character-level precision for code edits. You can now edit specific nodes (like parameters or variable names) within a single line without affecting surrounding code.
- **Pedagogical Syntax Tips**: The editor now provides language-specific advice when an edit fails syntax validation (Rust, QML, Python).
- **Column-Aware TreeNode**: Upgraded `TreeNode` structure and the Rust parser to track and utilize character offsets for enhanced precision.

### Changed
- **Enhanced Documentation**: Updated `README.md`, `examples`, and the interactive `wizard` to reflect the new surgical precision capabilities.
- **Version Bump**: Major refinement release marking the transition to v0.9.1 "The Surgical Update".

### Fixed
- **Precision Failures**: Resolved issues where inline edits would inadvertently delete parts of the line.
- **CLI Robustness**: Improved error reporting for JSON and cross-file operations.

## [0.9.0] - 2026-01-31

### Added
- **Slint Support**: Full AST-based editing and analysis for `.slint` files. Powered by `tree-sitter-slint`.
- **AI Default**: The `modernbert` (GnawSense) and `mcp` features are now enabled by default. No more `--features` flags needed for standard usage.
- **Enhanced Status**: The `status` command now proudly displays the state of **GnawSense**, **HRM2** (Hierarchical Reasoning), and **Undo/Redo** history.
- **GnawTree Architect Skill**: A specialized agent skill (`gnawtree-architect`) to guide AI agents in surgical code editing.

### Fixed
- **Safety Nets**: Implemented node count limits (500-1000 nodes) and depth limits in `list`, `skeleton`, and MCP tools to prevent agent context crashes.
- **Memory Optimization**: Refactored `list_nodes` to avoid cloning entire subtrees, significantly reducing memory usage on large files.
- **CLI Hygiene**: Removed duplicate `Status` command handlers and cleaned up unused imports in core modules.
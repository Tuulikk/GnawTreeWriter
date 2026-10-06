## [Unreleased]

### Added
- **Headline-zone diversification, v2**: the cap now applies to the first 5 SERVED slots only, and deferred entries lead the tail pass in fused order — nothing inside the served window is ever dropped (the first version dropped head-zone violators outright in the served path and deferred them to position ~2000 in diagnostics). Also fixes a regression where the pre-graph fused sort was lost, leaving no-adjacency queries served in raw cosine order.
- **Index preview depth 97 → 240 chars**: the embedded (and stored) preview now carries 2.5× more source context, giving the embedding model real signal instead of bare signature heads. Re-index required (`ai index`); raw cosine recall@1 rose 16% → 24% and pipeline recall@100 reached 90% on the 50-case set (7302 entries / 231 files).
- **Headline-zone diversification in the sense reranker**: the per-file cap (MAX_PER_FILE=4) now binds only in the first 5 served slots; entries deferred there are served in the tail pass instead of dropped. Eval showed 9/12 misses @k=100 were exact symbols crowded out by their own file's better-ranked heads (raw rank 5-90). Recall: 60/68/74/84 @ k=5/10/20/100 (was 60/66/72/76; raw cosine 54/70/76/90).
- **recall-eval diagnostics**: per-case expected_cosine_rank (model-side), expected_pipeline_rank (full reranked order, not capped at k) and top_pipeline_files (displacement audit) — separates "model never surfaced it" from "reranker pushed it out".
- **Graph-proximity channel in the sense reranker**: `rerank_satellite_with_graphs` fuses a third reciprocal-rank channel — call-graph adjacency of the top seed files (callers + callees, BFS-best-seed order) — into the ranking. Hubs (files with >100 resolved edges, e.g. `core/mod.rs`, `cli.rs`) are excluded as seeds and neighbors: adjacency to them is topical noise. Graph JSONs store absolute paths while the semantic index stores relative ones; `graph_path_key` normalizes both from the first `src/` segment. Absent or empty graphs degrade silently to the two-channel ranking. `ai recall-eval` runs the same pipeline (incl. the graph channel). Recall (50 cases): 60/66/72/76 @ k=5/10/20/100 (two-channel: 60/66/72/74; raw cosine: 54/70/76/90).

## [0.18.0] - 2026-10-06

### Changed — Semantic search: real retrieval embedder (BGE)
- **Embedding backend switched** from the raw ModernBERT masked-LM
  checkpoint (`answerdotai/ModernBERT-base`, mean pooling) to
  **BAAI/bge-base-en-v1.5** (CLS pooling + L2 normalization), because
  the new recall@k harness (`ai recall-eval`, evals/sense_recall.json)
  measured **recall@5 = 0%** on the MLM features — anisotropic space,
  all cosines ~0.9. After the switch (BGE, GPU-indexed, SQLite-stored):
  **recall@1 12.5%, recall@5 54.2%, MRR 0.30**. The legacy path is kept
  as `EmbeddingBackbone::ModernBertMlm` so old model dirs still load.
- **Query/passage split**: `get_query_embedding` prepends the BGE v1.5
  retrieval prefix ("Represent this sentence for searching relevant
  passages:") on the query side only; `get_embedding`/`get_embeddings`
  embed documents raw. `gnaw_sense`, `recall-eval` use the query path;
  `ai index` embeds documents.
- **512-token truncation** on the BGE tokenizer (was unbounded encode
  on ModernBERT); long node bodies are silently truncated at embed time.
- `AiModel::Bge` added; models cached under
  `.gnawtreewriter_ai/models/bge-base/`; `ai setup` downloads it.
  **Re-index after upgrading** — vectors from the MLM model are useless:
  delete the old index and run `gnawtreewriter ai index` again.
- Known limitation: BGE documents are embedded one forward per node on
  CPU (no batching yet) — indexing is noticeably slower than the old
  batched MLM path, but produces embeddings that actually work. GPU
  builds (`scripts/build-gpu.sh` + `indexing.device: auto`) index a
  full src/ tree in ~20 s.

### BREAKING (lib)
- `AiModel` (`src/llm/ai_manager.rs`) gained a variant `Bge` —
  exhaustive matches break; `AiModel::ModernBert` remains as the legacy
  MLM path. `ModernBertModel`'s field `model: ModernBert` was replaced
  by `backbone: EmbeddingBackbone` (new pub enum:
  `ModernBertMlm(ModernBert)` | `Bge(BertModel)`); embed via
  `get_embedding` (documents) / `get_query_embedding` (queries), never
  on the inner model.
- `IntegrityReport` (`src/core/guardian.rs`) gained a field
  `deltas: Vec<EditDelta>` (`#[serde(skip)]`). Struct-literal
  construction of `IntegrityReport` outside the crate breaks — build
  reports via `GuardianEngine::audit_edit[_with_language]` instead.
  (Guardian v2, docs/GUARDIAN_V2_PLAN.md Fas 4.)
- `Batch` (`src/core/batch.rs`) gained fields `last_verdict:
  RefCell<Option<Value>>` and `impacts: RefCell<Vec<Value>>`
  (`#[serde(skip)]`). Struct-literal construction breaks; prefer
  `Batch::new`/`from_json`/`with_file`. (Fas 4.1 batch parity.)

### Added — recall@k harness
- `ai recall-eval <eval-set> [--k N] [--json]`: runs an eval set
  (JSON array of `{query, expect_file, expect_preview?}`) against the
  current index and reports recall@1 / recall@k / MRR plus embed/search
  timings. `evals/sense_recall.json` ships 24 cases as the project
  baseline. Measurement only — the basis for model and storage
  decisions (found the 0% recall of the MLM checkpoint).
  Requires the `modernbert` feature.

### Added — SQLite-backed index storage
- The vector index is now a single `embeddings.db` (SQLite, WAL,
  bundled rusqlite) instead of one pretty-JSON shard per file: ~8×
  smaller on disk (93 MB → 11 MB on this repo) and fast to load
  (raw BLOB reads replace JSON parsing of every shard).
- **Orphan fix**: `save_index` now deletes a file's previous rows in
  the same transaction before inserting — re-indexed files can no
  longer leave stale entries behind (the JSON shards had no delete at
  all, so renamed/moved node sets accumulated forever).
- **Automatic one-time migration**: the first `load_project_index`
  imports legacy `<file_hash>.json` shards + `model_info.json` into
  the DB and renames the sources `*.migrated` (idempotent — never
  re-imports). `model_info` moved to the DB with a legacy-JSON
  fallback read. Public API unchanged; verified A/B: recall identical
  pre/post migration.

### Added — Guardian v2 (docs/GUARDIAN_V2_PLAN.md, Fas 0–4 + 5.1–5.3)
- **Structural deltas (Fas 1)**: `EditDelta` diff of old/new node trees
  detects operator inversion, dropped conditions, lost error handling
  and signature changes; context-aware severity (lone operator change
  stays a warning, contract-broken context escalates to error).
- **Invariant contracts (Fas 2)**: text-level contract of the old node
  (guards, error handling, asserts, unwrap-freedom, doc lines) must not
  be lost entirely — catches agent-style full regenerations.
- **No-op guard + EditReceipt (Fas 3)**: identical-content edits
  rejected loudly; post-write disk verification; `verified` +
  `bytes_changed` in MCP edit/insert responses and CLI output.
- **Rule fixes + healer hints in rejections (5.2)**: blocks name
  `[rule_id:line]` and the rule's `fix:` suggestion; healer failures
  named explicitly.
- **Structured `edit_verdict` (5.1+5.3)**: all four rejection paths
  record `{level, score, findings, suggestions}`; MCP
  `edit_node`/`insert_node` (and `semantic_edit` via delegation) attach
  it to `E_EDIT_REJECTED` responses. Contract test included.
- **Impact report (Fas 4)**: signature-changing edits carry
  `impact: {symbol, callers, sites}` in MCP responses and a `📊 Impact`
  CLI line, sourced from the knowledge graph
  (`.gnawtreewriter_ai/graph/`). Missing index ⇒ field omitted, never
  an error. `ImpactAnalyzer::load_all_graphs` implemented (was a
  placeholder) and `impact_analyzer` registered in `llm` (was
  unregistered).
- **Limitation fixes (Fas 4.1)**: honest `to_file` resolution in the
  relational index (Some(file) only when unambiguous — no more
  first-match guess); `analyze_impact` uses its `defined_in` parameter
  to exclude callers of same-name symbols defined elsewhere; batch
  runs the full single-edit validation (NO-OP guard, Guardian, rules)
  before any write and carries the same `edit_verdict` on
  `E_BATCH_ROLLED_BACK` plus `impacts` on successful applies.
- **Resolution observability + same-file rule (Fas 4.2, ROADMAP
  "Symbol resolution toward LSP-grade" steps 1–2)**:
  `RelationalIndexer::resolution_stats()` reports unique vs ambiguous
  symbol names, call-resolution rate and top-N ambiguous names;
  `doctor` gained a `knowledge_graph.resolution` check; impact
  responses carry `unresolved: N` (MCP) and the CLI prints
  `📊 Impact … (N unresolved)` so caller counts can be trusted
  appropriately. A definition in the current file now wins for
  bare-name calls (same-module precedence), resolving most same-name
  ambiguity without path parsing.
- **Import-aware call narrowing (Fas 4.2 step 3)**: `FileGraph` now
  stores the identifier tokens of each file's use/import statements
  (`#[serde(default)]` — old graph JSON stays loadable). When a bare
  call's name is defined in several files, exactly one candidate whose
  file stem appears in the caller's import tokens resolves the site
  (`use crate::utils::parse`, `from utils import parse`,
  `import { parse } from "./utils"`); zero or several stem matches
  stay honestly ambiguous.
  **BREAKING (lib)**: `FileGraph` gained a field — struct literals
  must add `imports` (or `..Default::default()`).

## [0.17.0] - 2026-10-05

### Added — Phase 10: Agent Confidence & Motivation (trygghet + maning)
- **`validate <file>` (CLI + MCP `{file_path}`)**: strict pre-edit syntax
  gate — node count on success; on failure `E_STRICT_PARSE` + the error
  position + pointer to the partial read paths, nonzero exit on the CLI.
  (AGENTS.md advertised this command in three places while it did not
  exist — following our own docs gave "unrecognized subcommand".)
  Shared core: `parser::validate_file`.
- **MCP `diff`**: independent post-write verification — `{old_file,
  new_file}` (text/json) or `{file_path, old_path, node_path}`, same core
  as CLI `gnaw-diff`, reads only.
- **`undo {preview: true}`**: lists what WOULD be reverted
  (id/file/operation/description, newest first) with zero mutation —
  `UndoRedoManager::peek_undo`; e2e-verified by history counts before and
  after.
- **Write receipts**: `transaction_id` on edit_node/insert/semantic_insert
  responses, `transaction_ids [src, tgt]` on move, one id per written file
  on batch apply — correlates with `history`/`undo` (`RefCell`,
  source-compatible for lib consumers).
- **Idempotent retries**: `edit_node` whose target already equals the
  requested content answers `already_applied: true` (trimmed compare, zero
  bytes written) instead of a scary `E_EDIT_REJECTED` — closes the
  historical timeout → retry → rejection spiral.
- **MCP `guide {situation?}`**: machine-readable mirror of the skill
  situations table (21 when/tool/example rows) for hosts that never load
  SKILL.md (Motor2); token-match filtering, no-match fails loudly;
  test pins every referenced tool to the live registry.
- **Cold-start notes** in the five model-tool descriptions ("first call
  may take 20-30 s — raise the host timeout") — untold latencies were the
  source of false "Not connected" exits.

### BREAKING (lib) — policy established
- Release checklist (AGENTS.md) now requires a `### BREAKING (lib)`
  section before release for enum/struct field additions and new
  required params; the retroactive 0.16.0 note (SenseResponse::quality)
  stands as the first entry. Integrators that path-depend (Motor2) rely
  on being warned.

### Fixed
- **`index_relations`: "calls" relations never carry an empty `from`**
  (`a655cd2`, Motor2 dashboard report): call sites inside impl blocks
  resolved to `gtw:{file}:function:{enclosing}`; scope-less calls get the
  synthetic `gtw:{file}:callsite:{line}`. Verified against Motor2's
  exact report file: 91 relations, 0 empty — their "(intern)" UI patch
  can go when they bump the path-dep. Rustdoc documents the contract;
  invariants tested on fixture and on a real 3000-line file.
- **`--no-default-features` builds** (Motor2 bug report, fixed in
  `18962b2`): `sense_with` lacked a feature gate while its body uses
  modernbert-gated code — 8 compile errors for path-dependents like
  motor2-gtw. Now gated with an honest not-modernbert stub; regression
  threshold added: `validate.yml` runs
  `cargo check --no-default-features --all-targets` (default AND mamba
  both carry modernbert, only this config catches gate holes).
  `examples/debug_loading.rs` got `required-features` (pre-existing
  candle hole).

### Docs
- **Doc-drift pass**: GTW_INSTRUCTIONS.md said "30 tools" while 34 were
  registered (doctor/history/stats/index_project had landed without
  updating it) — now complete, and `integration_mcp_instructions_listed`
  pins EVERY registered tool name to the file so the drift cannot
  recur. SKILL.md gained situations-rows/MCP-ref entries for
  index_project/history/stats plus `expand`/`search_quality` guidance
  and the `E_STRICT_PARSE` row; README lists updated (incl. a
  nonexistent `diff_to_batch` MCP tool claim corrected to CLI-only
  `diff-to-batch`); v0.16.0 notes test count corrected 232 → 233.

## [0.16.0] - 2026-10-05

### Added
- **Query expansion for satellite `sense`** (`expand: true`, opt-in):
  LFM2.5 expands the question into content terms (same prompt as
  investigate step 1); the expanded text runs as a SECOND semantic
  channel fused with the original by max cosine
  (`fuse_by_max`), and feeds the reranker's lexical side. Costs one LLM
  call + one extra embedding — default false. Non-mamba builds respond
  with an explicit note instead of failing. Live-verified in a
  release+mamba build (7.4 s end-to-end, terms like
  `['transaction log', 'persistence', 'entry storage', …]`).
- **Search feedback loop** (roadmap AUTO-koppling):
  - `search_quality {prior_failures, suspect_reason, expansion}` on
    every satellite answer; suspect reasons: `empty`, `low_cosine`
    (top raw cosine < 0.5), `no_lex_overlap` (top hit shares no query
    word — catches the flat-cosine saturation where unrelated code
    scores 0.85+).
  - Failures append to `.gnawtreewriter_search_log.jsonl`
    (gitignored, rotated at 1 MB → newest 500 lines); `prior_failures`
    counts earlier failures of the SAME normalized query and is shown
    in both the success note and the no-matches error — an agent sees
    "this query keeps failing" instead of trusting a flat field.
  - Live-verified: nonsense query flagged `no_lex_overlap`, second call
    showed `prior_failures: 1`, JSONL entries well-formed.

### Fixed
### BREAKING (lib)
- **`SenseResponse::Satelite` gained the `quality` field** (search feedback loop): downstream code that pattern-matches the variant with an exact field list must add `quality: _` (or `..`). MCP clients are unaffected (wire format only gained optional fields). Policy from now on: every lib-API breaking change gets a `### BREAKING (lib)` section here BEFORE release — integrators like Motor2 path-dep GTW and rely on being warned (see the AGENTS.md release checklist).

### Fixed
- `edit --ask "…" --all --preview` ignored the preview flag (writes
  were gated only by `!force`) — preview now always wins, even
  combined with `--force`; also clears the mamba-only unused-parameter
  warning.

### Docs
- `sense` schema: `expand` property + description documents
  `search_quality` semantics; `.gitignore` covers the search log.

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
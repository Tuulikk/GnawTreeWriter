# GnawTreeWriter Roadmap

## Overview

GnawTreeWriter is a tree-based code editor optimized for LLM-assisted editing. This roadmap outlines the evolution from a precise CLI tool to an intelligent agent-integrated platform.

The roadmap is divided into two sections:
- **Open Source** - Core functionality, community-driven features, available to everyone
- **Premium/Enterprise** - Commercial features, team collaboration, enterprise integrations

---

## Current Status: v0.9.8 (Released 2026-10-01)

### ✅ Completed Since v0.8.5

- **Performance**: Parallelized file processing (rayon) — pack 50% faster, explore 59% faster, indexing 40-52% faster. Explore command (4 zoom levels). Parse cache.
- **AI Context Tools**: `explore`, `pack` (+`--compress-threshold`), `curate`, `compress`, `diff-to-batch`, `stats`.
- **Local LLM command extension (LFM2.5)**: `explain`, `summarize`, `investigate` — fixed-step pipelines on a small local model (Q4, ~1 GB, CPU).
- **Token & time transparency**: Every LLM command reports a budget (expected/actual tokens, calls, estimated/actual seconds, truncation warning).
- **Self-calibrating timing**: `ai calibrate` measures this machine's inference speed and persists a `TimingProfile` for accurate time estimates.

---

# 🌍 Open Source Roadmap

All features in this section are and will remain **free and open source** under the project license.

---

## Symbol resolution toward LSP-grade (no external dependencies)

Staged improvement of the knowledge graph's symbol resolution plus the
observability to see whether each step actually helped. Principle:
tree-sitter + our own indexer only — nothing external. Every step ships
independently; step 1 makes each later win measurable.

| # | Step | Status |
|---|------|--------|
| 1 | **Observability**: `RelationalIndexer::resolution_stats()` (unique vs ambiguous symbol names, call-resolution rate, top-N ambiguous names with their files); surfaced in `doctor` (`knowledge_graph.resolution` check) and as `unresolved: N` in impact responses (MCP + CLI `📊 Impact (N unresolved)`) | ✅ 2026-10-06 |
| 2 | **Same-file rule**: a definition in the current file wins for bare-name calls, even when other files define the same name (Rust same-module scoping) — resolves most ambiguity with zero path parsing | ✅ 2026-10-06 |
| 3 | **Import awareness**: read `use` / `from x import y` / `import {y}` per file; a bare call is matched against imported candidates — one candidate = resolved, several = stays ambiguous | ✅ 2026-10-06 |
| 4 | **Qualified paths**: `mod::fn(...)` calls matched against definition module paths instead of last segment only | ⬜ |
| 5 | **Module map (`mod` tree)**: full Rust scope chain for exact resolution; removes the same-name problem in principle | ⬜ |

Known trade-off (kept intentionally): unknown/ambiguous call sites stay
counted in impact reports but are surfaced as `unresolved`, so agents
can scale their confidence. Long-term, optional LSP add-ons (see
AGENTS.md "Add-ons (LSP & MCP)") can provide true scope analysis via
language servers — the two tracks complement each other.

---

## Phase 8: Local LLM Command Extension 🔄 IN PROGRESS
**Target: v0.9.x → v0.10**

### What shipped (v0.9.7)

A small local LLM (**LFM2.5-1.2B, Q4 GGUF, ~1 GB**) extends the CLI with natural-language commands, built as **fixed-step pipelines** — Rust controls the flow, the model does focused understanding. No chat, no agentic loop, no API keys.

- [x] **`explain <file> [--node <path>]`** — plain-language explanation of an AST node
- [x] **`summarize <dir>`** — hierarchical AST-skeleton map-reduce summary
- [x] **`investigate "question"`** — query expansion → index search → ranked answer with file references
- [x] **`edit --ask` + `--all`** — LLM-proposed AST edits (line-based), multi-edit across occurrences
- [x] **MCP tools**: `explain`, `summarize`, `investigate`, `add_rule`
- [x] **`--resolution` flag** (fast / balanced / thorough) — trades speed vs detail
- [x] **Token & time budget reporting** on every command
- [x] **`ai calibrate`** — self-measured timing profile per machine
- [x] **Rules engine (spec: docs/RULES_ENGINE_SPEC.md)** — semgrep-inspired pattern linting, all 5 steps:
  - [x] Step 1 — rule format + structural AST matcher + `lint --rules`
  - [x] Step 2 — edit guardian (error blocks, warning allows)
  - [x] Step 3 — rule annotations injected into `edit --ask` prompts
  - [x] Step 4 — `lint --discover` (LLM-written rules) + `rules add` / MCP `add_rule` (agent-written)
  - [x] Step 5 — `edit --ask --all` multi-edit (consistent changes across occurrences)

### Architecture

```
src/llm/
├── ai_manager.rs   — LFM2.5 via candle quantized_lfm2 (GGUF), Mutex-guarded generate()
├── pipeline.rs     — fixed-step pipelines: explain, summarize (map-reduce), investigate, edit-ask
├── prompts.rs      — deterministic prompt templates per step
└── (TimingProfile) — self-calibrated prefill/decode times, persisted to config
src/core/
└── rules.rs        — pattern rules: compile ($X placeholders), structural match, lint, guardian
```
```

### Design notes (learned the hard way)

- **Model choice**: evaluated Mamba3 — ecosystem immature, no q4, `mamba-rs` CPU path is F32-only with hard arch limits (headdim ≤ 32, d_state ≤ 64). Pivoted to LFM2.5: mature GGUF/q4 (QAD Q4_0 ≈ 97% of BF16), candle support, active development.
- **Prefill is ~O(seq²)** in candle's LFM2 — a 2048-token prefill takes ~140s. Every step must stay small (~300 tokens), and `summarize` feeds **compact AST skeletons** (~98% smaller than raw source) instead of raw code. Summarize went from >3 min/file to ~15s/file.
- **Let the model stop itself**: EOS bias on `<|im_end|>` so generation ends naturally; truncation is always reported, never silent.

### LLM missions — status

1. ✅ **`edit --ask "change X to Y"`** — the loop-closer: LLM proposes an AST edit, the Duplex Loop validates syntax, Guardian checks structure, preview before apply. The AST is why a small model is viable for edits — it never needs mechanical precision; it states intent and the tree places. Error mode shifts from "touched the wrong place" to "proposed wrong content" (caught by validation).
2. ✅ **Rules engine (all 5 steps)** — see `docs/RULES_ENGINE_SPEC.md`. Pattern linting → edit guardian → prompt annotations → agent/LLM-written rules → multi-edit.
3. ⬜ **`curate` as digest** — run a suite of deterministic GTW tools (explore, index_entities, gnaw_find), feed condensed excerpts to the LLM, return a synthesized investigation with tool citations. A meta-pipeline: the LLM orchestrates GTW's reliable tools and distills.
4. ⬜ **`review` + `commit-msg`** — review an uncommitted diff (`diff_since`) for bugs/risks; generate commit messages from diff + ALF intent.
5. ⬜ **`impact "what breaks if I change X?"`** — relations graph (`index_relations`, `gnaw_graph`) provides candidates, LLM ranks and explains.
6. ⬜ **`query "find all places we handle auth"`** — natural language → AST search with structured results (node paths).
7. ⬜ **`complexity` / `test-gen` / `impl-stub`** — structural analysis, test generation, implementation stubs, inserted as validated AST nodes.

### Open questions

- **Model upgrade path**: LFM2.5-2.6B exists — same arch, ~2× size, better quality. Stronger small models are emerging; the pipeline format makes the model swappable behind `generate()`.
- **KV/state reset between calls**: verify candle's LFM2 ModelWeights fully resets between sequential generations (context leak vs measured prefill cost).
- **Chunk quality**: AST skeletons are fast but lossy — for `thorough` resolution, consider hybrid (skeleton + targeted raw excerpts).
- **Rules engine depth**: no `pattern-inside`/`pattern-not`/`fix` yet (v0.1 scope) — the structural matcher is designed to extend to them.

---

## Phase 9: Agent Adoption & MCP Parity ✅ COMPLETE 2026-10-05 (9.1–9.6 samtliga klara)
**Target: v0.10 | Source: GAP-REPORT-2026-10-04.md (uppdaterad in i roadmap 2026-10-04)**

*Diagnos: Motor2-agenter använder GTW nästan aldrig (diagnos/sökning/edit) trots "GTW först"-policyn,
medan gnawdriver används dagligen. sex konkreta luckor, prioriterade i ordning nedan.
Detaljerad bevisning: `GAP-REPORT-2026-10-04.md`.*

### 9.1 — `gtw_lint` saknas i MCP-registret (högst prio) ✅ COMPLETE 2026-10-04
- [x] Lägg `lint`-tool-post i `src/mcp/mod.rs`-registret + handler
  (`paths: string[]`, `recursive: bool`, `rules_file`, `severity`, `rule_id`,
  **+ ad-hoc `pattern`/`language`** så agenter kan göra ast-grep `-p`-sökning utan förhandsregel)
- [x] Delad funktionskärna: `run_lint(paths, &LintOptions) -> LintResult` i `src/core/rules.rs`
  som CLI (`handle_lint`) och MCP (`handle_lint_mcp`) båda anropar (crate, inte subprocess — MCP och CLI divergerar inte)
- [x] Strukturerat svar: `findings: [{file, rule_id, severity, message, line, column, node_path, captures}]`
  + `files_checked`/`truncated`/`file_errors` på tools/call-resultatet
- [x] Tester: 6 enhetstester för `run_lint` (fixture, ad-hoc, recursive-krav, truncation, severity-filter,
  oläsbar fil redovisas) + 2 MCP E2E (schema-kontrakt i tools/list; tools/call ad-hoc-sök på fixture, report-only-bevis)
- [x] CLI-paritet: `lint --pattern "$X.unwrap()" --language rust` (ad-hoc-sökning även från CLI,
  samma `run_lint`-kärna) — rökverifierad 2026-10-04 (text + json-format + builtin-regler)
- Sidovinster samma session: **MCP `undo`/`batch`-stubbar ersatta med riktiga implementationer**
  (`handle_undo_mcp` via `UndoRedoManager`, `handle_batch_mcp` via `Batch::from_file`/preview/apply)
  + `tool_definitions()` extraherad ur `json!`-blobben så registret blivit nod-adresserbart.
  Batch-E2E-testet uppgraderat från stub-kontrakt till äkta preview-kontrakt (missing `file` → 400/-32602).

### 9.2 — `fix:`-stöd i regler (ast-grep-paritet) ✅ COMPLETE 2026-10-04
- [x] `Rule.fix: Option<String>` i `src/core/rules.rs` (+ YAML-nyckel `fix:`; serde-skip när None)
- [x] `rule add --fix "…"` — validerar att pattern kompilerar OCH fixen parse:ar som giltig kod
  (`validate_fix`: `$NAME`→placeholder-identifierare, scaffold-fallback `fn probe() { … }`
  för uttrycks-former som `?`-kedjor; ogiltig fix avvisas, aldrig sparad)
- [x] `lint --fix` i `handle_lint`: default report-only; `--fix` bygger `fix_batch(findings, rules)`
  → atomisk Batch (Edit-op per träff, nod-innehåll byts på plats så paths inte skiftar); `--fix --preview`
  = diff utan skrivning. ALDRIG skriv utan explicit flagga. Samma för MCP: `lint`-tool fick `fix`/`preview`-args
  (`fix_applied` i strukturerat svar), `add_rule` fick `fix`-arg
- [x] Tester: 6 nya (expansion med multipla `$X`-träffar; obunden metavar = fel; validate accept/reject;
  fix_batch-op + skip-räkning; dedup per nod; **E2E: run_lint → fix_batch → apply → verifierat innehåll**)
  + rökprov: `--fix --preview` md5-oförändrad, `--fix` skriver, `undo --steps 2` återställer
- Kärnfunktioner: `expand_fix`, `validate_fix`, `fix_batch` (src/core/rules.rs);
  `LintResult.rules` bär effektiv regeluppsättning så anropare slipper läsa om regelfiler

### 9.3 — MCP-schemas + beskrivningar som lär ut (adoption-luckan) ✅ KLAR 2026-10-04
- [x] Fullt `inputSchema` (properties + required + exempel) på **alla 30** verktyg — inte bara
  prioriteringssexen. Schema-lögner rättade mot dispatchen: `list_nodes` fick `filter`/`max_depth`,
  `explain`/`edit_ask`/`investigate` fick `required`, `index_entities`/`index_relations` fick
  `anyOf` (file_path | file_paths), `save_state` är ärligt noll-args (`properties:{}, required:[]`)
- [x] Beskrivningsmall per tool: **VAD** + **NÄR** (jämförelse mot Read/Grep/Edit) + **RETURNERAR** + **EXEMPEL**
- [x] Kontrakts-test `integration_mcp_tools_adoption_contract`: unika namn, beskrivning ≥ 100 tecken,
  `inputSchema.type == object`, `required ⊆ properties` (schema ljuger inte), ärligt noll-args,
  ≥ 30 verktyg, prioriteringsverktygen (`batch`, `edit_node`, `insert_node`, `search_nodes`, `sense`,
  `semantic_edit`) med ≥ 1 property. Avvikelse från planen medveten: noll-args-verktyg tillåts tomma
  properties (att hitta på en fejk-property vore värre)
- [x] `GTW_INSTRUCTIONS.md` regenererad: 17 → 30 verktyg (förra försöket hade driftat) +
  diagnostisk kedja `analyze`/`get_skeleton` → `sense`/`search_nodes` → `read_node` →
  `edit_node`/`semantic_edit` → `gnaw-diff`/`undo`
- [x] **Bisetapp — resurrection-mysteriet löst** (GTW_MCP_ISSUE_LOG.md fynd 12/13): "resurrection" ×8
  var inga cache-buggar — `integration_mcp_tools_call_undo` revertade repo-rötens RIKTIGA
  transaktionslogg vid varje `cargo test`. Ny `serve_with_shutdown_root` tvingar tester till
  isolerad projektrot; regression bevisad med 2 gröna fullkörningar, rules.rs-md5 oförändrad.
  Batch-testets spec hade fel fält (`replace` → `path`+`content`) sedan skapandet

### 9.4 — Källcitat i LLM-svar (`investigate`/`get_semantic_report`) — steg 1-3 klara 2026-10-04
- [x] `sources: [{file, node_path}]` på båda LLM-svaren: `get_semantic_report` mappar findings →
  `{file, node_path}` (dedup); `investigate` mappar **evidence** (filerna svaret faktiskt
  syntesiserades från, ärligare än alla kandidater) → `{file}`. Ren builder
  `sources_from_evidence` + `Source` bor i `ai_manager.rs` (ogated) — pipeline är mamba-gated och
  testerna skulle aldrig körts i default-gate annars. Handlers fäster `sources` på toppnivå via
  rena payload-funktioner (`semantic_report_payload`, `investigate_payload` under mamba-gate)
  så kontraktet testas utan modell
- [x] Kontrakts-test: `integration_sources_semantic_report_payload` (fixture
  `src/core/batch.rs`/nod `1.2`, dedup-verifiering) i default-gate +
  `integration_sources_investigate_payload` i mamba-gate — båda anropar exakt de funktioner
  handlerna anropar. 6 ytterligare enhetstester (builder + report.sources)
- [x] `FileMatch.content_preview`: satellite-svar bär nu `preview` från `NodeEmbedding`
  (sparar extra `read_node`-rundtur); kompileringsforcerad enda konstruktionsplats
- [x] Bisurfad: `--features mamba` gick inte alls att kompilera (cli.rs:4889 saknade `fix`-fält
  i Rule-init — 9.2-beroende bakom gate); fixad med fix-paritet (`fix` parsas ur YAML/JSON)
- [x] Live-verifiering av v0.9.8:s stabilitetsfix (2026-10-04, loggad i `GTW_MCP_ISSUE_LOG.md`):
  `cargo install --path .` + live `sense` via stdio-MCP med känd fråga → rätt fil
  (`src/core/undo_redo.rs`) och rätt nod (topp `8` = `impl UndoRedoManager`, 0.79).
  `tools/list` bekräftar dessutom 9.3 live (30 verktyg, 23 WHEN-beskrivningar).
  Satellitläge (filupptäckt utan given file_path) kvarstår pga ofullständigt projektindex
  (63/102, inkrementellt `ai index` kan köras senare i bakgrunden); 4 äldre MCP-daemoner
  kör fortfarande före-install-binären tills deras sessioner startas om

### 9.5 — Stäng öppna poster ur GTW_MCP_ISSUE_LOG.md ✅ KLAR 2026-10-04
- [x] MCP `undo`/`batch`-stubbar (hardkodat "Undo executed"/"Batch executed") — ersatta med riktiga
  implementationer + E2E-tester 2026-10-04 (se 9.1-sidovinster)
- [x] `get_skeleton` tomt svar på stora filer: text-kanalen bar bara rubriken
  (skelettet låg på result-nivå) → skelettet finns nu i `content.text`,
  `truncated`-flagga explicit vid 500-nod-taket, tom skelett = guident fel
  (aldrig tyst). Kontraktstest `integration_mcp_get_skeleton_never_bare_header`
- [x] Stäng 2026-09-30:s halvfärdiga triage-checklistor — alla tre posterna
  (omgång 1, omgång 3, rotsak) avbockade med domar; "FIX-presentationen"
  för sense-zoom/satelit-textkanalen utförd (träffar i text + guidad tomhet)
- [x] Lägg till lint-regeln `rust_string_byte_slice` (`$X[..$Y]` med fix-mall
  `$X.get(..$Y).unwrap_or($X)`) i rules/builtin.yaml + rökprov: träffar på
  riktiga byte-slices i xml.rs/qml.rs, `--fix --preview` visar korrekt
  transform. Sidovinster: meddelanden interpolerar nu `$X`-bindings
  (`interpolate_message`), okänt rule-id = högljutt fel med nästa steg
  (inte tyst "No issues" — samma princip som fynd #14)
- [x] `doctor` som MCP-post: delad kärna `run_full_doctor` (CLI + MCP kan inte
  divergera, mönstret från run_lint), {healthy, passed, failed, warnings,
  checks[]} + guidad text. Sidovynt: `check_backups` ljusskannar nu (räknar
  per filnamn + validerar 3 nyaste) istället för att fullt-parsa 2 GB/128
  backups → doctor 80 s → 0,5 s; parser-smokes parallella (12 ms)

### 9.6 — GTW ska vara mer guidande och redovisande ✅ KLAR 2026-10-04
- [x] Skill registrerad i opencode (`SKILL.md` symlinkad till `~/.config/opencode/skills/gnawtreewriter/`) — 2026-10-04
- [x] SKILL.md omskriven: situations-tabell (16 rader, situation → MCP-verktyg →
  exempelanrop), fallback-regeln (timeout → EN omväg → logg i issue-loggen →
  verifiera bytes, aldrig git-checkout), korrekt batch-format (`{file, path,
  content}` — den gamla använde det felaktiga `search/replace`-formatet),
  `gnawtreewolf`-typer borta, status-delen pekar på `doctor` istället för ett
  inhackat versionsnummer
- [x] Alla `tool_error`-vägar guidar: 0 råa `e.to_string()`-passthrough, alla
  IO/feature-gate/modell-fel pekar på nästa steg (`ai setup`/`ai status`,
  `search_nodes`/`explore`, issue-loggen). Driftsäkrat av
  `integration_error_strings_carry_guidance` (bannade mönster får inte återkomma)
- [x] AGENTS.md: "För agenter som ANVÄNDAR GTW"-sektion — diagnostisk kedja i
  sex steg, noteringstaxonomin (7 kategorier, varje stängd post = en riktig fix),
  eskalationsregeln (EN omväg, verifiera bytes misstänkt "framgång", aldrig
  tysta feladrättelser)

**Definition of done (per item):** `cargo build` + `cargo clippy -- -D warnings` rent; tester gröna inkl. nya
(beteendetest — verifiera att datan flödar MCP-in → resultat ut); `GTW_INSTRUCTIONS.md` + `GTW_MCP_ISSUE_LOG.md` uppdaterade.
Motor2-sidans §35-utrullning sköts separat — hör inte hit.

---

## Phase 10: Agent Confidence & Motivation (tryghet + maning) 🔥 PRIORITERAD 2026-10-05
**Syfte:** LLM-agenter (framförallt OpenCode + Motor2) ska *känna sig trygga*
och *våga* använda GTW. Trygghet = kunna verifiera, förutsäga och återställa
i vartegrepp; maning = veta vad verktyget kan utan att ha läst vår SKILL.
Alla poster är **publika fixar** (syns i release notes).

### 10.1 — Säkerhetsluckor (verifiering + förutsägbarhet) ✅ KLAR 2026-10-05
- [x] **`validate <file>`** — AGENTS.md lär ut kommandot på TRE ställen
  (workflow, fel-tips, testexempel) men det **finns inte** → agenter som
  följer våra egna instruktioner får "unrecognized subcommand". Implementera
  (strict parse → OK/rapport + exit-kod) + som MCP-läsverktyg
  (`validate {file_path}` → E_STRICT_PARSE vid fel, guidad)
- [x] **MCP `diff`** — oberoende verifiering efter skrivning (CLI `gnaw-diff`
  finns, MCP saknas): `diff {old_file, new_file}` — "trust but verify" utan
  att lita på GTW:s egen svarstext
- [x] **`undo {preview: true}`** — visa vad som *skulle* återställas
  (transaktioner/filer) innan man återställer — samma preview-filosofi som
  för skrivningar, åt andra hållet; förutsätter att agenter vågar trycka
- [x] **Skrivkvitton** — `transaction_id`/`backup_id` i alla skrivsvar
  (edit_node, insert, move, batch, semantic_insert) så agenten kan
  korrelera med `history` och peka på exakt transaktion
- [x] **Idempotens vid retry** — `edit_node` när målets innehåll redan ==
  begärt → succé med `already_applied: true` i stället för
  `E_EDIT_REJECTED` (Motor2:s historiska dödsfall: timeout → retry →
  skrämmande avslag trots att målet redan nåtts)

### 10.2 — Maning (veta vad GTW kan) ✅ KLAR 2026-10-05
- [x] **Kallstart-latens i beskrivningarna** — `sense`/`doctor` beskrivningar
  säger "första anrop efter serverstart kan ta 20–30 s (modelllastning) —
  höja värdens timeout" → färre falska "Not connected"-utgångar som tär på
  tilliten
- [x] **`guide {situation?}` MCP-verktyg** — SKILL:s situations-tabell som
  maskinläsbar coaching för värdar som inte läser vår SKILL.md (Motor2):
  situation → verktyg → exempelanrop. Driftsäkrat: test att alla citerade
  verktygsnamn finns i registret
- [x] **BREAKING-disciplin för lib-konsumenter** — v0.16 lade till fält i
  `SenseResponse`-varianten (brytande för exhaustiva matchare) utan
  utropstecken i CHANGELOG. Lägg `BREAKING`-sektion + semver-policy för
  lib-ytan i AGENTS release-checklista (integratorer som Motor2 path-dep
  litar på repo som ropar ut brytningar)

**Definition of done (per item):** `cargo build` + `cargo clippy -- -D warnings` + `cargo check --no-default-features --all-targets` rent; tester gröna inkl. nya beteendetester (MCP-in → resultat ut); `GTW_INSTRUCTIONS.md`/`SKILL.md` uppdaterade där verktygsytan ändras; release notes nämjer posten.

---

## Phase 1: Reliability & Safety ✅ COMPLETE
**Status: DONE**

- [x] **Transaction Log System**: JSON-based log tracking all operations with timestamps.
- [x] **Multi-File Time Restoration**: Project-wide and session-based rollback.
- [x] **Undo & Redo Commands**: Navigation without Git dependency.
- [x] **Interactive Help System**: `examples` and `wizard` commands.
- [x] **Temporal Demo Project**: Step-by-step evolution guide with history snapshots.

---

## Phase 2: MCP Integration & Extensions ✅ COMPLETE
**Status: DONE**

- [x] **Stdio & HTTP Transports**: Native support for modern AI clients.
- [x] **Registry & Discovery**: Seamless tool listing for Gemini CLI and Zed.
- [x] **Surgical Edit Tools**: Precise node-based manipulation via MCP.
- [x] **Standardized Extensions**: Centralized `/extensions` directory for all integrations.

---

## Phase 3: GnawSense & Semantic Infrastructure 🔄 IN PROGRESS
**Target: Q1–Q3 2026**

### Architecture

```
src/llm/
├── ai_manager.rs          — Model management (load ModernBERT, device selection)
├── gnaw_sense.rs          — Broker: Satelite/Zoom/ProposeEdit
├── semantic_index.rs      — Vector storage + cosine similarity search
├── project_indexer.rs     — Project crawler → per-node embeddings
└── relational_index.rs    — Call graph (Call/Definition/Reference relations)
```

**Two modes**: Satelite (project-wide search) and Zoom (file-level search with impact analysis).
**Model**: ModernBERT-base (149M params, 768-dim, 571 MB) via Candle — fully local, no external calls.

### Performance Baseline (Measured 2026-04-26)

| Step | Time | Bottleneck? |
|------|------|-------------|
| Load ModernBERT (571 MB) | ~2–3 sec | 🔴 YES — 90% of total time |
| Generate 1 embedding | ~50–200 ms | 🟡 Medium |
| Brute-force cosine sim (468 nodes) | ~0.15 ms | 🟢 Negligible |
| Load 23 JSON index files | ~5–10 ms | 🟢 Negligible |

**Key insight**: Vector DB is NOT needed at current scale. The bottleneck is model loading, not search.

---

### Tier 1: Quick Fixes (Hours)

- [x] **Confidence threshold**: Only return results with score > 0.3. Warn if best result is > 0.5 (low confidence). Prevents false-positive "matches" that agents trust.
- [x] **JSON output for sense**: `--json` flag for machine-readable results. Agents currently parse ANSI/emoji stdout.
- [x] **Extract name — all languages**: Add patterns for `def `, `class `, `func `, `pub fn`, `async fn`, `impl`, `trait`, methods, etc. Currently only handles `fn ` and `struct `.
- [x] **Better error when model not loaded**: `not(feature = "modernbert")` should return JSON-formatted error for agents.

### Tier 2: Agent-Friendly (1–2 Days) ✅ COMPLETE

- [x] **Auto-index without prompt**: `--auto-index` flag that indexes without interactive y/N. Current prompt blocks agents. *(Verified in code 2026-10-04: `src/cli.rs` `auto_index: bool`)*
- [x] **Sense-insert with all intents**: Support `before`, `inside`, `replace` — not just `after`. *(Verified in code 2026-10-04: `src/llm/gnaw_sense.rs` stödjer after/before/inside/replace)*
- [x] **Fix propose_edit position logic**: `get_next_index()` used `idx + 3 + 1` hack. *(Verified in code 2026-10-04: nu `idx + 3` med dokumenterad positionskodning "insert after child at index idx")*

### Tier 3: Performance (2–3 Days) ✅ COMPLETE

- [x] **Model caching**: Load models once and reuse across calls. *(Verified in code 2026-10-04: `OnceLock`-cache för ModernBERT + LFM2.5 i `src/llm/ai_manager.rs`, plus shared sense broker med per-request panic isolation sedan v0.9.8)*
- [x] **JIT index cache**: Cache embeddings per file+content-hash. *(Verified in code 2026-10-04: `calculate_content_hash` i `semantic_index.rs` + `project_indexer.rs`)*
- [ ] **Incremental project indexing**: Only re-index changed files at `ai index`. Per-file hash-check finns, men global inkrementell indexering saknas fortfarande.

### Tier 4: Intelligence (1–2 Weeks)

- [x] **Query expansion**: Expand vague queries into multiple embedding searches for better recall. *(Verified in code 2026-10-04: `investigate` gör query expansion → index search → ranking i `src/llm/pipeline.rs`)*
- [ ] **Hierarchical context in embeddings**: Include parent node type/name when generating embeddings. A `login` function in `tests/` differs from one in `auth/`.
- [ ] **Fix RelationalIndex integration**: `index_directory()` is only called inside `sense()` JIT, never during `ai index`. Call graph is never properly built at index time.
- [ ] **Feedback loop**: Log failed searches (no results, low confidence) and adjust scoring. Spec calls this "AUTO-koppling".

### Tier 5: Scale & Vision (Future)

- [ ] **HNSW index**: Pure Rust `hnsw` crate for approximate nearest neighbor. Only needed at 10k+ nodes. Current brute-force takes 0.15 ms at 468 nodes.
- [ ] **Smaller/code-specialized model**: Evaluate swapping ModernBERT-base (571 MB) for a smaller code-tuned model to reduce memory footprint.
- [ ] **Side-effect prediction**: Use relational graph to warn about downstream impact before edits (HRM Vision).
- [ ] **Structural style transfer**: Learn user's coding style and normalize agent-generated code.
- [ ] **Semantic diffing**: Show changes as tree operations instead of line diffs.

---

### Completed (Foundation)

- [x] **Zoom Mode**: Semantic search within a single file.
- [x] **Skeletal Mapping**: High-level definition overview for token efficiency.
- [x] **Node Discovery**: Search for nodes by name or content.
- [x] **Semantic Selection**: Target nodes using `@fn:name` shorthand.
- [x] **Project-wide Cache**: Background crawler that indexes the entire project into a local vector store (v0.7.7).
- [x] **Semantic Anchors**: Basic `after` insertion based on semantic landmarks.
- [x] **Relative Placement Expansion**: Support for `INSIDE`, `BEFORE`, `BEGINNING`, and `END` using AST context.
- [x] **The Duplex Loop (Foundation)**: Self-correcting edits using structured syntax errors (v0.7.4).

---

## Phase 4: Language & Parser Expansion ✅ COMPLETE
**Status: DONE**

- [x] **New Languages**: Kotlin, Swift (v0.7.6).
- [ ] **Template Support**: Jinja2 / HTML mixed-mode parsing.
- [ ] **Multi-Parser Files**: Seamlessly switching parsers within a single file.
- [ ] **Structural Anomaly Detection**: AI-linter that warns about unsafe patterns or semantic duplication before edits.

---

## Phase 5: Intelligence & Autonomy 🔄 IN PROGRESS
**Target: Q3 2026**

- [x] **ALF (Agentic Logging Framework)**: Standardized temporal journaling for AI agents (v0.7.5).
- [x] **Structural Scaffolding**: Create new files by defining a tree schema (Moved from Phase 5 to v0.7.1).
- [ ] **HRM 2.0 Integration**: Implementation of Hierarchical Reasoning Models for side-effect prediction and structural style transfer. See [docs/HRM_VISION.md](docs/HRM_VISION.md).
- [🔄] **"Fix-my-Fix" Loop**: If an edit causes a parse error, use the AST to suggest or auto-apply the syntax fix (Initiated in v0.7.4).
- [ ] **Semantic Diffing**: Show changes as tree operations instead of line diffs.

---

## Phase 6: Universal Tree Platform 🔄 PLANNED
**Target: Q4 2026 / v1.0**

- [ ] **Gnaw Daemon**: Background process holding the project AST in memory for instant responses.
- [ ] **Cross-File Refactoring**: Symbol renaming with cross-file guarantees.
- [ ] **File Watcher**: Real-time updates to the AST when files are changed.
- [ ] **Infrastructure as Code**: Terraform, K8s YAML manipulation.

---

## Recent Progress

### v0.9.2 (2026-04-25) — THE AGENT TOOLBELT UPDATE 🔧
- ✅ **Quick Insert**: Bulk insert content after regex-matched lines with `--filter`, `--unique`, preview support.
- ✅ **5 New Languages**: JavaScript, C#, Dart, Svelte, SQL → 26 languages total.
- ✅ **Global `--dry-run`**: All edit commands respect global dry-run flag for safer agent workflows.
- ✅ **Diagnostics Module**: `doctor` command, `GNAW_JSON=1`, `GNAW_VERBOSE=1`, post-edit AST diff.
- ✅ **Better Error Context**: Offending code line, language name, nearby nodes on parse failure.
- ✅ **`commands` endpoint**: `gtw commands --json` lists all tools with metadata for dynamic integration.

### v0.7.1 (2026-01-22) — THE SCAFFOLDING UPDATE 🏗️
- ✅ **Structural Scaffolding**: Command `scaffold` for creating AST-templated files.
- ✅ **Improved Indexing**: Support for arbitrary child positions in core `insert`.
- ✅ **Help System**: Added semantic search and scaffolding examples to `examples`.

### v0.7.0 (2026-01-22) — THE SEMANTIC RELEASE 🚀
- ✅ **GnawSense**: Semantic search (`sense`) and action (`sense-insert`) using ModernBERT.
- ✅ **TCARV 1.0**: Core methodology for AI-assisted engineering (+ TAC & AUTO modules).
- ✅ **Anchor System**: Ported from Comparative-Writer to handle `// ...` anchors.
- ✅ **Agent Intelligence**: Added `GEMINI.md` and updated `AGENTS.md` for safer AI collaboration.
- ✅ **Safety Policies**: Implemented Anti-Lobotomy and Git-Surgery (No-Nuke) rules.

### v0.6.11 (2026-01-12)
- ✅ **Help System Cleanup**: Fully updated `examples` and `wizard` commands to match current functionality (Contributed by OpenCode).
- ✅ **Command Documentation**: Added missing examples for `search` and `skeleton`.
- ✅ **Quick-Replace Fix**: Corrected outdated references to the `quick` command.

### v0.6.10 (2026-01-12)
- ✅ **Full CLI Parity**: Added `search`, `skeleton`, and `semantic-report` commands to match MCP capabilities.
- ✅ **Docs Cleanup**: Fixed `examples --topic ai` to accurately reflect available commands.
- ✅ **Linting**: Silenced unused field warnings in `AiManager`.

### v0.6.9 (2026-01-12)
- ✅ **Semantic Selection**: Target nodes using `@fn:name`, `@struct:name`, etc., instead of numeric paths.
- ✅ **Enhanced CLI**: Added `read` command and improved `list` output with node names.
- ✅ **Clean Core**: Moved name-extraction logic to `TreeNode` for universal use.

### v0.6.8 (2026-01-11)
- ✅ **Agent Safety Guide**: Added "The Gnaw Mental Model" to AGENTS.md to prevent AI mistakes.
- ✅ **Zed Flatpak Support**: Added dedicated documentation and `flatpak-spawn` instructions for Zed users.
- ✅ **Robust Extensions**: Improved Zed extension source code for better reliability.

### v0.6.7 (2026-01-11)
- ✅ **Contextual Usage Hints**: Added a "Just-in-Time" learning system that prints helpful tips to stderr.
- ✅ **Double-Brace Shield**: Hardened CLI and MCP outputs against common shell escaping issues.

### v0.6.6 (2026-01-11)
- ✅ **Colored Diff Preview**: Added ANSI color support for CLI previews.
- ✅ **MCP Diff Feedback**: Edit and Insert tools now return context-aware unified diffs.
- ✅ **Preview Tool**: Added `preview_edit` to MCP for "dry run" capabilities.

### v0.6.5 (2026-01-11)
- ✅ **Intelligence Loop**: Integrated LabelManager and Semantic Reporting.
- ✅ **Robust MCP**: Fixed JSON-RPC syntax and added stdio/http stability.
- ✅ **Clean Imports**: Optimized dependency usage in core modules.

### v0.6.4 (2026-01-11)
- ✅ **Skeletal Mapping**: Added `get_skeleton` for high-level definition overviews.
- ✅ **Smart Search**: Added `search_nodes` to find targets by name/text.
- ✅ **Token Efficiency**: Depth-limited listing and punctuation filtering.

### v0.6.2 (2026-01-10)
- ✅ **Full MCP Stdio Support**: Integration with Gemini CLI and Zed.
- ✅ **License Guardian**: Added `scripts/check-license.sh` to ensure MPL-2.0 purity.
- ✅ **Temporal Demo**: Added `examples/temporal-demo` micro-project.

### v0.6.0 (2025-01-05)
- ✅ Fixed GitHub Actions CI/CD for ModernBERT.
- ✅ Extensive dogfooding - fixes made using GnawTreeWriter!

---

*This roadmap is a living document. Inspired by Comparative-Thinker and Comparative-Writer.*

---

## Phase 7: Debuggability & Agent Diagnostics ✅ COMPLETE
**Target: v0.10.0 | Q2 2026**

*Investigation date: 2026-04-24 — comprehensive audit of existing debugging infrastructure.*

### Existing Infrastructure (Already Complete)
- [x] **Transaction Log System** — JSON-based operation log with timestamps, hashes, session IDs
- [x] **Backup System** — Per-edit JSON backups with content hashes
- [x] **Undo/Redo** — Stack-based navigation through transaction history
- [x] **Restoration Engine** — Project-wide and session-based rollback
- [x] **Guardian Engine** — Edit impact analysis (volume, complexity, comment preservation)
- [x] **Healer (Duplex Loop)** — Auto-heal for simple syntax errors (missing braces/colons)
- [x] **Post-edit Validation** — Reparse code after edit, block invalid syntax
- [x] **TreeVisualizer** — Visual diff with focus-nodes and sparklines
- [x] **Health Check (`status`)** — Environment, Git, AI, MCP, undo-state
- [x] **ALF (Agentic Logging)** — Intent/risk/outcome journaling
- [x] **Report Engine** — Markdown structural evolution reports
- [x] **SyntaxError** — Parsing errors with line/col/message
- [x] **Lint Command** — Find issues in files

### Prio 1 — Agent-Critical (Making GTW Easy to Debug for AI Agents)
- [x] **`--dry-run` on edit/insert/delete** — Return what *would* happen without writing to disk (global flag working on all commands)
- [x] **Structured JSON errors** — `GNAW_JSON=1` env var gives machine-readable errors with error_type, suggestion, context
- [x] **`--verbose` flag** — `GNAW_VERBOSE=1` visar parser-val, node-uppslagning, guardian-score, AST-validering, structural changes

### Prio 2 — Robustness
- [x] **`doctor` command** — Test all parsers, check backup integrity, validate transaction log
- [x] **Better error context** — Include offending code line at parse errors, language name, nearby nodes

### Prio 3 — Advanced
- [x] **Post-edit AST diff** — Compare tree before/after and warn if structure breaks (e.g. function_declaration becomes something else)
- [ ] **Reference analysis** — Warn if deletion affects referenced code (future: cross-file)

### Identified Gaps (Why These Features Matter)
1. **No post-edit AST verification** — Validation checks if code *parses*, not that tree structure is consistent
2. **No `--dry-run` on single edits** — Only batch and quick-replace have preview. Agents can't test-drive edits
3. **Limited error messages** — `SyntaxError` has line/col but lacks code context, fix suggestions, and language info
4. **No structured error format** — All errors formatted for humans (emojis, ANSI). Agents need JSON
5. **No debug/verbose flag** — No way to see what GTW does step-by-step (parser choice, node lookup, etc.)
6. **No `doctor` command** — `Status` shows env but doesn't validate parsers, backups, or log consistency
7. **No consequence analysis** — Guardian checks volume/complexity but not reference integrity or scope
8. **No diff command** — Can't see before/after detail of an edit (only visualizer with focus)

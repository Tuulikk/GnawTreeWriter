# Handover — 2026-11-10 (III): v0.24.0 släppt · fas 1 KLAR · CT-plan P1 klar (wiring) · fas 2 ökning 1 klar

## Bekräftat läge

- **v0.24.0 SLÄPPT** (`eaa1209`, tagg + GitHub Latest) + uppföljande commits
  (alla pushade): kuvert-fix `21dc368`, fas 2-ökning 1 `5db65e4` (schema v4),
  prosa-lagret `83cd3d7`, CT-planen `b860747` + rev II `a14b245`, harmony
  steg 1 `14b4e0d`, **P1 wiring `834d390`**.
- **Fas 1 (code-index) KLAR:** delmål A (coverage-yta: CLI `--coverage` +
  MCP `mode=coverage`), B (hermetisk fixtursvit + facit-projekt
  `testprojekt/code-index-fixture/`), C (extraktor v2: metodanrop H1,
  makron H2, scoped-path H3, implements kind=trait H4).
  Före/efter GTW-repot: relations 4 650 → 22 095; resolved call-kanter
  890 → 3 848 (4,3×); käll-matched 49,4 % → 70,7 %; implements 31/31
  korrekt-målade (de gamla 88,5 % innehöll fel-mål).
- **CT-planen P1 KLART enligt S-protokollet** (S0–S5: källan läst, byggd +
  KÖRD — 6 mätta cases pinnade; DERIVE-klassad): `src/llm/harmony.rs`
  (ren, funktions-oberoende modul) + **P1 wiring KLART**: `RerankedHit`
  bär råa pelare (`lex`, `graph: Option<f32>`) + `harmony` (verdict +
  suggestion); FileMatch exponerar harmony; MCP-prosan surface:ar
  top-träffens verdict (suspect_note-mönstret).
  **291 lib-tester + fixtur gröna · clippy `-D warnings` ren ·
  no-default-features 0 fel.** Studieranteckning 09.1 ifylld i planen §3.6.
- **Prosa-lagret i DB (användarens design):** `prose_annotations` i schema
  v4 — ankare (kind/key/hash/name), body, producer/model/language,
  generation-revision, state machine `fresh` → `stale` ("ej färsk") →
  `invalid` ("ej gällande", med `suggested_key` vid unik samnamnig
  efterträdare). Deterministisk sweep per revision; producenter via
  `upsert_prose`; aldrig grind. Coverage: `ProseSummary` +
  `regenerate_recommended`.
- **Migrations-helande (nytt mönster):** commit-hookens äldre binär kan
  migrera storen med äldre ADD-sätt → `migrate_connection` kör idempotent
  DDL (CREATE IF NOT EXISTS) vid VARJE öppning + additiv kolumn-reparation
  (PRAGMA table_info + ALTER ADD DEFAULT). Verifierat på live-storen
  (anchor_name reparerat).
- **Bonus-guards:** `E_DUPLICATE_ATTRIBUTE` (dubbel-`#[test]`-fällan) —
  4-falls enhetstest. Issue-loggen: post II (kuvert + daemner) och III
  (edit-stöd: search_nodes-annotering, read_node storlek, storleksschock —
  "bygg nu"-status).
- **Daemner/hygien:** 84 → 8 daemner (76 deleted-exe under cline-hub
  dödade). Nuvarande 5 daemner kör CPU-bin (0.24.0 med allt) — **MCP
  coverage LIVE** (verifierat: full rapport via wrapper). libexec (GPU)
  = 10:45-bygget, SAKNAR kuvert/schema-v4/prosa → **GPU-rebuild måste
  köras om** (föregående avbröts av omstarten); harmlöst tills dess
  (daemner använder bin). gnawdrivers db_surf var stale (egen varning) —
  bygg om gnawdriver; python3-sqlite3 mode=ro användes som sanningskälla.

## Öppna poster (ordning enligt överenskommelse)

1. **P2** (CT-planen): graf-frågor `kind=cycles|impact` + `chain` i ny
   `src/core/code_graph.rs` (SQL UNION-rekursiv reachable — UNION dedupar
   så cykler terminerar; Rust path-stack-cykler med rotationskanonisering;
   dokumenterad begränsning: syntetiska callsite-from_ids propagerar inte).
2. **Fas 2 I2:** D5(a) atomisk wrap av refresh (revision+deltan+nuläge i EN
   tx; failed-refresh-bokföring i egen tx) — benchmark på Motor2-storlek
   fixtur.
3. **I3:** entity/relation-deltas (aggregerade D1/D8-nycklar med counts) +
   `changes --from --to` (CLI/MCP).
4. **I4:** D7-källklassificering (6 009 function_scope_unmatched →
   kvalificera from-identiteten) + git_hook-spelningens live-verifiering
   (hooken spelar INTE revisioner idag — upptäckt; hook-scriptet behöver
   reinstall för `GTW_CODE_INDEX_SOURCE=git_hook`-prefixet).
5. **P3a** (complexity på riktig AST — decision_kinds-tabell per språk;
   fixtures är riskposten) och **P4** (JSON-analysartefakt `gtw-analysis/1`
   + triage-skript) per CT-planen.
6. **Prosa-ytan:** `upsert_prose` har INGEN CLI/MCP-yta än (producenter
   väntar; guard blockerar rå sqlite-skrivning) — nästa öknings-kandidat
   jämte P2.
7. **Underhåll:** hook-reinstall (`--uninstall-hook` + `--install-hook`)
   för env-prefixet; CHANGELOG för prosa-lagret + P1; `code-index.md`
   omgenererad av hook (kontrollera).

## Pekare

- Plan of record: `docs/COMPARATIVE_THINKER_ADAPTATION_PLAN.md` (rev II;
  studieanteckning 09.1 ifylld i §3.6) +
  `docs/CODE_INDEX_RELATION_HISTORY_PROPOSAL.md` ("Fas 2-designnoter",
  låsta D1–D10 + design-noter från sessionen).
- Checkpoints: `phase1-coverage-harness`, `phase1-extractor-v2`.
- Minne: `gnawtreewriter:decision:phase2-locks`,
  `gnawtreewriter:decision:code-index-prose-layer`,
  `gnawtreewriter:reference:comparative-thinker-plan`,
  `gnawtreewriter:lesson:opencode2x-mcp-binary-pickup`.
- Facit-projekt: `testprojekt/code-index-fixture/` (FACTS.md — före/efter
  C mätta 2026-11-10).
- Issue-log: post II (kuvert + daemn-hygien) och III (edit-stöd, "bygg nu").

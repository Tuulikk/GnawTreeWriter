# Handover — 2026-11-10 (IV): språk + edit-korrekthet (md/css/yaml/toml → tree-sitter)

## Denna session (IV) — KLART, ej släppt

**Fyra nya grammatiker** (probe-verifierade mot tree-sitter **0.26.13** innan de
togs in; ingen drar en andra runtime):

- **PowerShell** (`.ps1`, `.psm1`, `.psd1`) — 20 filer i arbetsytan, ingen
  parser tidigare. Grammatikegenheter: **tom fil ger ERROR-noder** (så
  `validate` säger "invalid" på en 0-byte `.ps1`) och en funktionskropp med
  flera statements på EN rad ger ERROR vid `param`. Flervadiga kroppar parsar rent.
- **CMake** (`.cmake` + `CMakeLists.txt`), **Makefile**
  (`Makefile`/`makefile`/`GNUmakefile`), **Ruby** (`.rb`).
- **Filnamnsdetektering** införd: `parser_for_special_filename` körs **före**
  ändelsematchningen. Behövdes för tre klasser: `CMakeLists.txt` (ändelse
  `.txt` → blev `TextParser`), `Makefile` (`extension()` = `None`) och
  `.bashrc`/`.zshrc`/`.profile`/… (också `None`). De senare fick bash-parsern.
- **Tillägg på befintliga språk:** `.pyi` → Python, `.mts`/`.cts` → TypeScript.
- **Ruby var en tyst lögn:** `get_parser_for_language` mappade `ruby`/`rb` →
  `"rb"` men `get_parser` hade ingen `"rb"`-arm → `GenericParser` (en nod), så
  `language: ruby`-regler kompilerade mot ett tomt träd och matchade aldrig.

**Fyra parsrar bytta till tree-sitter** (byte-exakta spann i stället för
position-blinda noder): **markdown, css, yaml, toml**. Detta var inte
kosmetika — `edit_node` förstörde filerna:

| Format | Före (mätt mot 0.24.0-binären) | Efter |
|---|---|---|
| markdown | rubrik-edit **raderade `#`**; stycke-edit **åt upp blankraden** | `# Ny rubrik` intakt, blankrader kvar |
| css | värde-edit **ersatte hela regelraden** | `body { color: green; }` kirurgiskt |
| yaml | edits **nekades** av Duplex ("broke the AST") → read-only | `namn: demo2`, `version: 1` kvar |
| toml | som yaml | `namn = "demo2"`, nyckeln orörd |

Rotorsak: `start_col`/`end_col` var `0`, `content` ofta rekonstruerad text i
stället för källslice ⇒ splitten föll tillbaka på **helrads-ersättning**.
Delad konverterare `parser::tree_from_ts_node` + `parse_with_tree_sitter` bär
spann-logiken på ett ställe (var 8 kopior). Netto **−1249/+835 rader**
(markdown-parsern 569 → 33).

**Bonusfynd (pre-existing, inte från språkarbetet):**

- **`edit_node` tog bort filens avslutande newline i ALLA format** — även
  Rust: `}\n` → `}`. `str::lines()` + `join("\n")` sätter aldrig tillbaka den.
  Fixat i alla fyra splice-ställen (`join_preserving_newline`, LF/CRLF-medveten).
  Betyder att varje GTW-edit historiskt gett "no newline at end of file"-brus
  och att `bytes_changed` varit off-by-one.
- **Panik vid leaf-edit:** `TreeVisualizer::generate_sparkline` delade med noll
  när den renderade en ensam nod (max-djup 0 = normalfallet när en **leaf**
  redigeras). Krashen kom **efter** att editen skrivits och verifierats på
  disk ⇒ anroparen såg fel på en lyckad operation. Vakt `max(1)` i
  `visualizer.rs` + `visualizer_test.rs`, 2 enhetstester.
- **CLI:t hade en egen `--extensions`-default** (samma 15 stale lista), så
  `CodeIndexOptions::default()`-fixen var verkningslös för CLI-vägen. Nu tom
  default → faller tillbaka på den inbyggda listan (**en** källa, inte två).
- **`code_index`-default: 15 → 32 ändelser**, bara parser-backade. Nio grupper
  (`kt/kts`, `swift`, `cs`, `dart`, `svelte`, `sql`, `qml`, `slint`,
  `mjs/cjs`) hade parser men indexerades aldrig.

**Verifiering:** ny `tests/legacy_parser_surgical_edit.rs` (8 tester) som
assertar **hela filtexten** efter edit — inte "innehåller värdet", vilket var
precis vad som släppte igenom korruptionen. **385 tester / 19 testbinärer /
0 fel**. `cargo clippy -- -D warnings` (lib+bin) **ren**. `rustfmt --check` ren
på samtliga rörda filer. `code-index.md` omgenererad (2275 rader, titel
`GnawTreeWriter`, de fyra nya parserfilerna med).

**Probe-artefakter:** `/tmp/gtw-lang-probe/` (isolerade crates, egen
`CARGO_TARGET_DIR`; `REPORT.md` + `LEGACY_TS.txt` + `USAGE.txt`).
`tree-sitter-dockerfile 0.2.0` **avvisad**: exporterar ingen `LANGUAGE` och
drar en andra runtime (0.20.10). `hcl 1.1.0` och `lua 0.5.0` verifierade OK
men **inte inkopplade** (0 filer i ytan).


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
   för env-prefixet; CHANGELOG för prosa-lagret + P1. `code-index.md` är
   **omgenererad manuellt** i session IV (ny binär, 2275 rader) — hooken
   skriver den inte längre automatiskt, kontrollera vid nästa commit.

### Nya öppna poster från session IV

8. **`cargo clippy --all-targets -- -D warnings` är RÖD — men inte på denna
   sessions kod.** Två lints, båda i testkod i filer som inte rördes:
   - `src/core/index_entities.rs:975` → `clippy::doc_lazy_continuation`
   - `src/llm/semantic_index.rs:843` → `clippy::field_reassign_with_default`
   `cargo clippy -- -D warnings` (lib+bin) är **grön**. Beslut behövs: fixa de
   två lints, eller medvetandegör att CI-grinden är lib-only.
9. **`cargo fmt --check` avviker på 52 filer redan på HEAD** — varav 40+ orörda
   av session IV (bl.a. `src/cli.rs`, `tests/code_index_fixtures.rs` och
   `src/llm/harmony.rs`). En repo-wide `cargo fmt` är ett eget beslut; sessionen
   formaterade bara egna filer för att inte begrava ändringen i brus.
10. **`.h` → C++-heuristik** ej gjord. `.h` mappas alltid till C-parsern, även
    för C++-headers (`class`/`template`/`namespace`), och ytan har 49 `.h` mot
    50 `.cpp`. Beteendeförändring på befintliga filer ⇒ egna tester först.
11. **JSON kvar på interim (J1).** `json_semantic_edit` navigerar per
    **nyckelnamn**, inte numeriskt index — ett byte till `tree-sitter-json`
    ändrar varje JSON-path och måste göras ihop med funktionen (pinnat av
    `json_edit_stays_valid_and_goes_through_the_semantic_path`).
    **Separat fynd (fanns före session IV):** `--preview` visar den råa
    splitten medan skarp edit tar den semantiska vägen — preview och apply är
    oense för JSON.
12. **PowerShell-grammatikens egenheter** (dokumenterade i `powershell.rs`):
    tom fil ⇒ ERROR-noder (så `validate` säger "invalid" på en 0-byte `.ps1`),
    och en funktionskropp med flera statements på EN rad ⇒ ERROR vid `param`.
    Ingen åtgärd tagen — kräver antingen en `empty_ok`-väg i `validate` eller en
    lokalt patchad grammatik.
13. **Dockerfile saknas fortfarande.** `tree-sitter-dockerfile 0.2.0`
    exporterar ingen `LANGUAGE` och drar en andra runtime (0.20.10) ⇒ oanvändbar
    utan omskrivning. `hcl 1.1.0` och `lua 0.5.0` är probe-verifierade men
    **inte inkopplade** (0 filer i ytan).
14. **MCP-ytan:** `blast`, `graph` och `cycles` är fortfarande **CLI-only**.
    P2 i CT-planen adresserar det via `code_index kind=cycles|impact`.

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

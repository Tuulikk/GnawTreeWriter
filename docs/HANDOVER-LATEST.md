# Handover — 2026-11-10: code-index fas 1, delmål A (coverage) levererat; B+C nästa

## Bekräftat läge (UPPDATERAT efter B+C, samma dag)

- **Delmål A klart:** `code_index_coverage.rs` (measure + 3 tester), CLI
  `--coverage`, MCP `mode=coverage`, instruktions-paritet. Checkpoint:
  **phase1-coverage-harness**.
- **Delmål B klart:** hermetisk fixtursvit `tests/code_index_fixtures.rs`
  (tempdir + `.git`-markör) + synligt facit-projekt
  `testprojekt/code-index-fixture/` (FACTS.md med [F1]-[F10]-markörer).
  Fixturen HITTADE två oplanerade hål: **H3** scoped-path-anrop
  (`super::`/`crate::` — 0 rader) och **H4** implements-upplösning död av
  namnkollision (`impl:Speaker` vs `trait:Speaker`).
- **Delmål C klart (extraktor v2, `EXTRACTOR_VERSION = "2"`):** metodanrop
  (H1), makron med bang-suffix (H2), scoped-path-anrop (H3),
  relationstyp-medveten upplösning — implements matchar bara `kind=trait`
  (H4). **282/282 lib-tester + fixture-test grönt.** Före/efter GTW-repot:
  relations 4 650 → 22 095; calls resolved 890 → **3 848 (4,3×)**;
  käll-matched 49,4 % → **70,7 %**; implements 88,5 % → 39,7 % men
  **100 % korrekt-målade** (31/31 kind=trait, SQL-join-verifierat — de
  gamla 69 innehöll fel-mål-resolutions). Rate-tolkning: nämnaren växte
  med ärligt olösligt (stdlib + makron) — per-konstruktion-mätning gäller.
- **Bonus: `E_DUPLICATE_ATTRIBUTE`-guard** i edit-funneln
  (`check_duplicate_attribute_above` + hook i `edit_node_at_path`) —
  högljudd rejection när replacement-content åter-included attributet
  ovanför målnoden (dubbel `#[test]`-fällan; dogfood-fynd, fixad i
  samma session). Enhetstest: 4 fall. Loggad i `GTW_MCP_ISSUE_LOG.md`.
- **Daemn-hygien:** 84 → 8 daemner (76 deleted-exe under cline-hub
  dödade; kvar: 6 OpenCode per katalog + 1 Codex). OBS: gnawdrivers
  db_surf gav motstridiga svar (stale binär, egen _build_varning) —
  bygg om gnawdriver före nästa db-surf-diagnos; python3-sqlite3
  (mode=ro) användes som sanningskälla.
- **Ocommitat arbetsläge:** allt ovan + `AGENTS.md` (OpenCode 2.x-sektion),
  `GTW_INSTRUCTIONS.md` (coverage + 38-räkning), proposal (D1–D10),
  `GTW_MCP_ISSUE_LOG.md` (2026-11-10-posten), `docs/HANDOVER-LATEST.md`.
  Checkpoint: **phase1-extractor-v2** (12 filer).

## Nästa (i ordning)

1. **Install + service-restart:** `cargo install --path . --features mcp`
   (eller `scripts/build-gpu.sh`) → `mode=coverage` live i MCP; verkliga
   verifiering: anropa coverage-mode och lyssna på feltexten.
2. **CHANGELOG + commit:** verktygsytan (coverage + extractor v2 + guard)
   ropas ut; commit enligt konvention (feat(core): ...).
3. **Fas 2-förutsättningar:** D1, D2, D5, D7–D9 låses innan
   revisions-tabellerna (schema v4) skrivs — se proposalens
   "Granskningsbeslut"-tabell.
4. **Reparationsmål (största klasserna):** function_scope_unmatched 6 009
   (from-identitetens kvalificering), unresolved_macro 3 419 (makro-
   expanderare/domänkontrakt), db_surf-verifiering efter gnawdriver-ombygge.

## Pekare

- Plan och beslutsspår: `docs/CODE_INDEX_RELATION_HISTORY_PROPOSAL.md`
  (sektionen "Granskningsbeslut 2026-11-09": D1–D10, T1–T5; fas 2 får
  först börja när D1, D2, D5, D7–D9 är låsta — D3/D4/D10 styr impact-
  fasen).
- Checkpoint: `phase1-coverage-harness` (återställbar via checkpoint-
  systemet).
- Minne: `gnawtreewriter:decision:relation-history-review-d1-d6`
  (båda granskningsomgångarna), `gnawtreewriter:lesson:opencode2x-mcp-
  binary-pickup` (grundad MCP/binär-modell).
- Motor2-mall för detta format: `Motor2/docs/HANDOVER-LATEST.md`.

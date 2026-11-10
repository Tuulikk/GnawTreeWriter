# GnawTreeWriter — Release Notes (v0.24.0)

**Datum:** 2026-11-10
**Typ:** Minor Release (nya funktioner, bakåtkompatibla)

## Sammanfattning

Code-index fas 1 levererad: **coverage-ytan** (CLI + MCP), **extraktor v2**
(metodanrop, scoped-path-anrop, makron, ärlig implements-upplösning) och
**E_DUPLICATE_ATTRIBUTE-guarden** i edit-funneln. Hjälp-manifestet synkat
(var 28 av 54 kommandon). Resolved call-kanter på GTW-repot: 890 → 3 848.

## Changes

### Added

- `code-index --coverage` (CLI) och `code_index mode=coverage` (MCP):
  grafkvalitet ur `.gnawtreewriter_ai/code_index.db` — upplösningsgrad per
  relationstyp, D7-källklasser (matched / synthetic_callsite /
  function_scope_unmatched / file_scope_unmatched / other), target-klasser
  (resolved / unresolved_macro / unresolved_qualified_path /
  unresolved_plain_name), symbols-sammanfattning. Read-only — öppnar aldrig
  en store, loud error med refresh-hint när den saknas.
- Extraktor v2 (`EXTRACTOR_VERSION = "2"`; nästa refresh bygger om helt):
  metodanrop (`obj.method()`), scoped-path-anrop (`super::`/`crate::`),
  makron med bang-suffix (`unresolved_macro`-klassen) och
  relationstyp-medveten implements-upplösning (endast `kind=trait`).
- `E_DUPLICATE_ATTRIBUTE`-guard: replacement-content som åter-inkluderar
  attributet ovanför målnoden avvisas högljuddt — dubbel-`#[test]`-fällan
  (giltig syntax, testet körs två gånger) uppstår inte längre tyst.
- Hermetisk fixtursvit `tests/code_index_fixtures.rs` + handmärkt
  facit-projekt (`testprojekt/code-index-fixture/`).
- Dokument: revisionsgrafs-proposal (D1–D10 + T1–T5), HANDOVER-LATEST,
  AGENTS.md OpenCode 2.x-sektion (klient–server, per-katalog-daemner,
  binär-pickup-kedjan).

### Changed

- Relation-inventariet växer kraftigt (GTW-repot: 4 650 → 22 095 rader;
  calls 2 555 → 19 996). Konsumenter av relations-tabellen ska förvänta
  metodanrop-, makro- (bang-suffix) och scoped-path-rader.
- Upplösningsgrad per typ SANK (calls 34,8 → 19,2 %) — nämnaren växte med
  ärligt olösligt (stdlib + makron). KVALITETEN höjdes: implements
  88,5 → 39,7 % med 100 % korrekt-målade targets (31/31 kind=trait,
  SQL-verifierat); de gamla 69 innehöll fel-mål-resolutions.
- `gnawtreewriter commands` synkad: 28 → 54 kommandon (code-index,
  validate, gnaw-* -familjen och AI-familjen var borta sedan tidigare
  push). Varaktig fix (clap-genererad introspektion) är backlog.

### Fixed

- `migrate_connection` delad mellan fil-backad öppning och in-memory
  test-schema — en DDL-sanning.

## Uppgradering

```sh
cargo install --path . --features modernbert,mcp
# GPU-bygget (MCP-daemnens binär): scripts/build-gpu.sh
```

Nästa `code-index --db`-refresh bygger om storen (extraktor v2 —
generation-nyckeln ändras). Läsare av relations-tabellen: förvänta nya
radformer (metodanrop/makron/bang-suffix) och se coverage-rapporten för
ditt repos egna före/efter-tal.

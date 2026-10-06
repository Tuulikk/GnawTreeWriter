# Guardian v2 — Designplan: detektera "bad edits" även när AST:n är korrekt

Version: 0.1 (design) | Datum: 2026-10-06 | Status: utkast för granskning

---

## Bakgrund

GTW validerar edits i tre steg i `GnawTreeWriter::edit()` (`src/core/mod.rs:207`):

1. **Guardian** (`src/core/guardian.rs`) — heuristik för volym/nyckelord/kommentarer
2. **Syntaxvalidering** — parse före skrivning + Healer (`src/core/healer.rs`)
3. **Rules guardian** — 70 AST-mönsterregler (`rules/builtin.yaml`), error blockerar

Gapet: en edit kan vara **syntaktiskt perfekt och ändå semantiskt förstörande**.
Inverterad jämförelse (`==`→`!=`), borttagen bounds check, tyst bortfallen
felhantering — allt parar rent och slinker igenom.

### Bugg som redan är fixad (2026-10-06)

Guardian var felkopplad: `audit_edit` fick *(gammal nod, HELA filen efter edit)*
istället för *(gammal nod, ny nod)*. Eftersom `edit_node_at_path` returnerar
hela den modifierade filen (`src/core/mod.rs:216`) gjorde jämförelsen nod-mot-
hel-fil att volym- och komplexitetscheckarna var död kod (hel fil är nästan
alltid större och nyckelordsrikare än en enskild nod).

- Fix: `audit_edit(resolved, content)` — jämför nod mot nod (core/mod.rs:258)
- Regressionstest: `core::tests::guardian_blocks_drastic_node_reduction`
  (har en andra funktion `helper` som håller hel-filens volym/komplexitet hög —
  utan den diskriminerar inte testet den gamla felkopplingen)
- Även: Insert/Delete granskas fortfarande INTE av Guardian (se Fas 1.4)

---

## Mål

En agent ska inte kunna göra ett edit som:
1. Tyst ändrar semantik (operatörer, villkor, felhantering)
2. Tappar invariantar som fanns i gamla koden (vakter, asserts, doc)
3. Rapporteras som lyckad utan att en byte ändrades (fynd #14-klassen)
4. Ändrar en signatur utan att få veta vilka call sites som påverkas

...utan få ett **strukterat, självförklarande avvisande med konkreta förslag**.

---

## Fas 1 — Structural Guardian v2 (jämför träd, inte text)

**Kärnidén:** Parsa gammalt nodinnehåll och nytt nodinnehåll var för sig
(tree-sitter finns redan) och klassificera deltas.

### 1.1 EditDelta-modell

Ny modul `src/core/edit_delta.rs` (eller utöka `guardian.rs`):

```rust
pub enum DeltaKind {
    OperatorChange { old: String, new: String },   // == → !=, && → ||, ! borttagen
    ConditionRemoved,                               // bounds check försvinner
    ErrorHandlingRemoved { what: String },          // ?, unwrap→tom retur, match→_
    SignatureChange { what: String },               // param, returtyp, pub
    LiteralChange { old: String, new: String },     // magic number / sträng
    CallTargetChange { old: String, new: String },  // foo() → foo2()
    StatementRemoved { count: usize },
    CommentRemoved { lines: usize },
    NoOp,                                           // identiskt innehåll
}

pub struct EditDelta {
    pub kind: DeltaKind,
    pub severity: Severity,        // Error / Warning / Info
    pub location: Option<String>,  // relativ beskrivning i noden
    pub suggestion: Option<String>,
}
```

### 1.2 Jämförelseloop

Rekursiv träd-jämförelse (matcha barn på typ + position):

**Prestanda (Gemini-feedback 2026-10-06):** jämför full djup ENDAST för
"logiskt kritiska" nodtyper — `binary_expression`, `unary_expression`,
`if_expression`/`if_statement`, `return_statement`, funktions-signaturer.
Övriga noder går igenom billiga checkar (typ- och volymjämförelse) så att
exekveringstiden hålls låg även för stora noder.

- `binary_expression` med olika operator-token → `OperatorChange`
- Barn som finns i gammalt träd men saknas i nytt → klassificera per typ
  (villkor/felhantering/statement)
- `identifier`-byte där båda sidor är call-argument → `CallTargetChange`
- Funktionsparameter-/returtyp-listor differar → `SignatureChange` (Error)

**Severity för OperatorChange är kontextmedveten (Gemini-feedback):**
Warning som default (operatörsbyten kan vara legitima bugfixar). Uppgraderas
till Error endast när samtidigt kontraktsbrott föreligger enligt Fas 2
(t.ex. bounds check borttagen i samma edit) — detta motverkar att
utvecklare/agenter tvingas köra med `--force` för giltiga ändringar.

### 1.3 Integration i audit_edit

`GuardianEngine::audit_edit` får en andra fas: efter de befintliga
heuristikerna körs `diff_trees(old_ast, new_ast)` och varje `EditDelta`
bidrar med poäng + meddelande + förslag. Befintliga volym/kommentar-
checkar behålls som fallback för språk utan tree-sitter-parning.

### 1.4 Täck Insert/Delete

Idag körs Guardian endast för `EditOperation::Edit` (core/mod.rs:248).
- **Delete:** jämför noden som tas bort mot tom — hela `audit_edit`-logiken
  gäller (volym = 100 % reduktion → Critical för funktioner med logik)
- **Insert:** lägre grad — dubblettdetektion (se 3.2) och rules guardian
  räcker initialt

---

## Fas 2 — Invariant-preservation

Extrahera ett "edit-kontrakt" från gamla noden FÖRE apply:

| Kontraktspost | Detekteras som | Varning |
|---|---|---|
| Felhantering | `?`, `unwrap`, `expect`, `match Err/None`, `if x.is_none()` | Error när ny nod saknar ALL felhantering som fanns |
| Bounds/vaktsatser | `if idx < len`, `is_empty`-guards tidigt i block | Error när borttagna |
| Asserts | `assert!`, `assert_eq!`, `debug_assert` | Warning |
| Doc-kommentarer | `///`, `/** */` intill noden | Warning när borttagna utan explicit delete |
| Panik-fria garantier | gamla noden hade `unwrap`=0 → ny har `unwrap` | Warning ("införde panikväg") |

Kontraktet beräknas i `GuardianEngine::extract_contract(old_node) -> Contract`
och checkas i samma audit-fas som Fas 1.

---

## Fas 3 — No-op-detektion och byte-rapportering (fynd #14)

1. **Pre-write no-op:** hash gammalt nodinnehåll vs `content` — identiska ⇒
   avbryt med högljutt fel: `"edit was a NO-OP (0 bytes would change)"`.
   Gäller även quick-replace (redan högljudd där — samma kontrakt i edit_node).
2. **Post-write verifiering:** efter `fs::write` — re-read, hash-verifiera,
   räkna diffade bytes. Varje edit-svar (CLI + MCP) får:
   ```json
   {"applied": true, "bytes_changed": 142, "verified": true}
   ```
   `verified: false` = skrivning kunde inte bekräftas ⇒ Error, aldrig tyst OK.

---

## Fas 4 — Impact-koppling (kunskapsgrafen)

`ImpactAnalyzer::load_all_graphs` är en stubb som returnerar `Ok(Vec::new())`
(`src/llm/impact_analyzer.rs:63`).

1. Implementera läsning av sparade grafer (JSON i indexkatalogen)
2. Vid edit där noden är en symbol med `SignatureChange`-delta: bifoga
   påverkade call sites i svaret:
   ```json
   {"impact": {"callers": 3, "sites": ["src/a.rs:1.2.5", "src/b.rs:3.1"]}}
   ```
3. Kräver index (index_relations) — saknas index ⇒ utelämna fältet, inte fel.

---

## Fas 5 — Agent-förslag: självförklarande avvisanden

Mönstret finns redan i `semantic_edit` (`src/mcp/mod.rs:3015–3052`):
`semantic_match {confidence, candidates, matched_path}`. Generalisera:

### 5.1 Strukturerat `edit_verdict` på ALLA avvisanden

```json
{
  "edit_verdict": {
    "level": "critical",
    "score": 0.1,
    "findings": [
      {"rule": "structural_delta", "severity": "error",
       "message": "bounds check 'if idx < len' removed",
       "fix": "återställ vakten före indexeringen"}
    ],
    "suggestions": [
      "Använd insert_node med parent 1.2 om du menade lägga till kod",
      "Liknande nod: 1.2.5 (confidence 0.81)"
    ]
  }
}
```

### 5.2 Lågt hängande frukter (kan ske före Fas 1)

- **Regelfix i avvisanden:** `rules/builtin.yaml`-regler har redan `fix:`-
  mönster men edit-blocket (core/mod.rs:339) skriver bara ut `message`.
  Skicka med `fix`-förslaget per finding.
- **Healer-förslag i valideringsfel:** Healer kan redan föreslå
  `}`/`:` — bifoga förslaget i felmeddelandet som `suggestion` istället
  för att bara auto-heala tyst.
- **Kandidatnoder:** vid Guardian-Warning på `@name`-upplösning — lista
  andra noder med samma namn (flera träffar = fel nod vald är vanligt).

### 5.3 MCP-kontrakt

Alla edit-verktyg (`edit_node`, `semantic_edit`, `insert_node`, `batch`)
lägger `edit_verdict` i svaret vid block. CLI skriver ut samma innehåll
läsbart. Kontraktstest i `tests/mcp_integration.rs` (mönstret finns i
`integration_mcp_instructions_listed`).

---

## Prioritering

| # | Fas | Vinst | Kostnad | Ordning |
|---|-----|-------|---------|---------|
| 0 | Buggfix nod-vs-nod (KLAR) | Guardian fungerar alls | — | ✅ 2026-10-06 |
| 1 | 5.2 (regelfix i block, healer-förslag) | Direkt agentnytta | Liten | ✅ 2026-10-06 |
| 2 | 3 (no-op + bytes_changed) | Fynd #14-klassen dör | Liten | ✅ 2026-10-06 |
| 3 | 1 (EditDelta) | Största detektionsvinsten | Medel | ✅ 2026-10-06 (feedback inarbetad: kritiska noder + kontextmedveten severity) |
| 4 | 2 (kontrakt) | Fångar agent-regenerering | Medel | ✅ 2026-10-06 |
| 5 | 5.1+5.3 (enat verdict-kontrakt) | Agent-UX | Medel | ✅ 2026-10-06 |
| 6 | 4 (impact) | Signatur-medvetenhet | Medel-stor | ✅ 2026-10-06 |

### Fas 3 — implementerad 2026-10-06

- **No-op-guard:** `GnawTreeWriter::edit()` avvisar edits vars innehåll är
  identiskt med nodens nuvarande innehåll ("NO-OP: … Nothing was written").
  Kompletterar MCP:ns idempotenta retry-guard på trimmed-nivå
  (`src/mcp/mod.rs`, `already_applied`).
- **Post-write verification:** efter varje `fs::write` läses filen om och
  jämförs med avsett innehåll; `verified == false` ⇒ Error, aldrig tyst OK.
- **`EditReceipt`** (`src/core/mod.rs`): `{transaction_id, verified, changed,
  bytes_changed}` via `last_edit_receipt()`. CLI `edit` skriver ut
  "✓ applied & verified on disk (bytes changed: N)"; MCP `edit_node`/
  `insert_node` bifogar `verified` + `bytes_changed` i svaret.
- Tester: `edit_noop_is_rejected`, `edit_receipt_reports_verified_bytes`
  (+ 2 Guardian-regressionstester) — alla hermetiska via `tempfile`.

### 5.2 — implementerad 2026-10-06

- **Regelfix i avvisanden:** ny helper `rules::fix_for_finding(finding)`
  (`src/core/rules.rs`) expanderar den träffade builtin-regelns `fix:`-
  mall med findingens `$X`-bindningar. Både RULES GUARDIAN BLOCK-meddelandet
  och stderr-varningarna skriver nu ut `[regel-id:rad]` samt
  `— suggested fix: \`…\`` när en fix finns, plus pekaren
  `lint --fix` (preview först).
- **Healer-förslag i valideringsfel:** när en healing misslyckas anger
  felet nu vilken åtgärd som provats ("Automatic healing attempt (…)")
  istället för att bara rapportera syntaxfelet.
- Tester: `fix_for_finding_expands_builtin_fix` (rules.rs),
  `rules_block_names_rule_and_line` (core/mod.rs).

**Lärdom (session 2026-10-06):** `quick-replace` unescapar `\n`-sekvenser i
argument — backslash-escapes i kodtext (t.ex. `'\n'`) korrumperas. Använd
escape-fria konstruktioner (`split_whitespace()`) eller STDIN (`-`).

### Fas 1 — implementerad 2026-10-06

- **Ny modul `src/core/edit_delta.rs`:** `DeltaKind` (OperatorChange,
  ConditionRemoved, ErrorHandlingRemoved, SignatureChange, LiteralChange,
  StatementRemoved) + `EditDelta {kind, severity, message, suggestion}`.
- **Träddiff med kritiska-noder-fokus (Gemini):** full djupjämförelse endast
  för binary/unary/if/condition/return/match/try-noder; övriga beskärs via
  textlikhet eller bunden vandring (`MAX_DEPTH 24`, `MAX_DELTAS 16`).
  Operatörer jämförs som direkta barn-tokens (GTW:s `build_tree` inkluderar
  anonyma tokens) plus innehållsnivå-logik (`&&`/`||`/`and`/`or`-antal)
  som fångar borttagna villkorsoperander även vid omstrukturering.
- **Kontextmedveten severity (Gemini):** `audit_edit_with_language` i
  `guardian.rs` höjer OperatorChange från Warning till Error när samma edit
  också tog bort villkor/felhantering. Poäng: Error −0.35, Warning −0.15,
  Info −0.05, clamp [0, 1].
- **Wiring:** core `edit()` skickar filändelsen; `audit_edit` bevarad som
  wrapper (bakåtkompatibel, hoppar strukturell diff utan språk).
- Tester: 3 i `edit_delta` (inversion, identiskt → inga deltas, villkor-
  operand-borttag) + 2 e2e i `core::tests` (guard-borttag + operatörsbyte
  blockeras; enstaka operatörsbyte tillåts utan block).

### Fas 2 — implementerad 2026-10-06

- **`Contract` + `extract_contract`/`check_contract`** i
  `src/core/edit_delta.rs`: felhantering (unwrap/expect/try/catch/except),
  vakt-rader (`if`-rader med jämförelser/`.len()`/`is_empty`), asserts
  (`assert!`-familjen), unwrap-frihet, doc-rader (`///`).
- **Eldar endast på total förlust** — komplementärt till Fas 1:s per-token-
  deltas; funkar även när en full regenerering ändrat trädformen totalt
  (textkontrakt från gamla noden, oberoende av trädalignering).
- Nya `DeltaKind`-varianter: `AssertRemoved`, `PanicPathIntroduced`,
  `DocRemoved`. Panik-introduktion (gammal kod var unwrap-fri, ny har
  unwrap/expect) varnas som Warning — Fas 1 såg bara bortfall.
- **Wiring:** guardian.audit_edit_with_language extendar deltlistan med
  kontrakts-deltas före poängsättningen; kontraktets ConditionRemoved/
  ErrorHandlingRemoved triggar även kontextuppgraderingen.
- Tester: `contract_detects_total_invariant_loss`,
  `contract_flags_panic_path_introduction` (edit_delta) +
  `guardian_contract_blocks_regeneration_losing_guard` (e2e).
- Känd begränsning: `?`-operatören ingår ej i felhanterings-uppsättningen
  (för bullrig i textnivå); vaktdetektering är rad-baserad (flerradiga
  villkor kan missas).

### Fas 5.1+5.3 — implementerad 2026-10-06

- **`GnawTreeWriter::last_edit_verdict()`** (`src/core/mod.rs`): fält
  `last_verdict: Option<serde_json::Value>` — nollställs i början av
  `edit()`, sätts vid ALLA fyra avvisandena:
  1. NO-OP-guard → level `notice`, rule `no_op`, förslag: verifiera nod-
     sökväg / `preview_edit`
  2. Guardian Critical → level/score från Guardian-rapporten, findings
     rule `structural_delta` (severity `error` för Critical, annars
     `warning` — `matches!` eftersom `IntegrityLevel` saknar
     `PartialEq`), förslag: `read_node` / `--force`
  3. Syntaxvalidering (healed-fail och no-healer-grenarna) → level
     `critical`, rule `syntax`, förslag från Healer:ens
     `action.description` respektive språk-tip:et
  4. RULES GUARDIAN BLOCK → findings med `rule`/`severity`/`message`/
     `fix` (via `fix_for_finding`), förslag: `lint --fix`
- **MCP-kontrakt (5.3):** ny `tool_error_with_data(msg, code, data)`
  (`src/mcp/mod.rs`); `edit_node` + `insert_node` (och därmed
  `semantic_edit`, som delegerar till `edit_node`-hantlaren) bifogar
  `edit_verdict` i felresultatet vid `E_EDIT_REJECTED`. `batch` använder
  `Batch::apply()` (går ej via core `edit()`) — känd begränsning,
  dokumenterad i planen.
- **Kontraktstest:** `integration_mcp_edit_rejection_carries_edit_verdict`
  (`tests/mcp_integration.rs`) — hermetisk server + tempdir, edit som
  tappar en bounds-vakt must svara `isError` + `E_EDIT_REJECTED` +
  `edit_verdict {level: critical, findings med guard/condition, suggestions}`
  och filen måste vara orörd på disk.
- Lärdom: den tidigare `\n`-korrumperingen var bash-citatterminering
  (enkelfnutt-sträng med `'` inuti), INTE GTW-unescape — ankare med
  `\n`-sekvenser matchar filens literala backslash-n korrekt.

### Fas 4 — implementerad 2026-10-06

- **`ImpactAnalyzer` återupplivad**: modulen var inte ens registrerad i
  `src/llm/mod.rs` (oreferenserad stub). Nu `pub mod impact_analyzer` +
  re-export; `load_all_graphs` delegerar till
  `RelationalIndexer::load_all_graphs` (läser
  `.gnawtreewriter_ai/graph/*.json`). Tomt index ⇒ tom rapport, aldrig
  fel. Ny `new_with_root(project_root)`-konstruktor + enhetstester.
- **`IntegrityReport.deltas`**: rapporten bär nu med sig Fas 1-deltorna
  (`#[serde(skip)]` — EditDelta saknar Deserialize). **BREAKING (lib)**
  — noterad i CHANGELOG [Unreleased].
- **Core-wiring**: i `edit()` (före verdict-blocket — `resolved` lånar
  self, får inte användas efter `set_verdict`; E0502-lärdom) — när
  deltorna innehåller `SignatureChange` och noden har ett namn:
  `find_project_root` → `ImpactAnalyzer::new_with_root` →
  `analyze_impact(namn, fil)`; `{"symbol", "callers", "sites"}` sätts i
  `last_impact` endast när callers > 0. `last_edit_impact()`-getter,
  nollställs vid edit-start.
- **Svar:** `attach_impact` i MCP bifogar `impact` på edit_node/
  insert_node-success (semantic_edit ärver via delegation); fältet
  UTELÄMNAS när None (aldrig null-brus). CLI skriver `📊 Impact: N
  caller(s)` + `↳ fil:nod-sökväg` per site.
- Tester: `signature_change_reports_impact_via_knowledge_graph` +
  `no_signature_change_no_impact_field` (core, hermetiska tempdir-
  projekt) och `integration_mcp_edit_reports_impact_on_signature_change`
  (MCP-kontrakt: impact på signaturändring, frånvaro vid body-only).
- Kända begränsningar: `to_file`-upplösning tar första matchen
  (relational_index), symbols-namn tas från nodens `get_name()` (nya
  symboler utan index ger ingen impact), batch går fortfarande via
  `Batch::apply()` utan verdict/impact.

## Acceptanskriterier (Fas 1)

- Edit som byter `==` mot `!=` i ett villkor blockeras (Error) utan `--force`
- Edit som tar bort en funktion med logik (Insert/Delete-täckning) blockeras
- Benigna edits (rename, kommentarsbyte, små tillägg) passerar utan varning
- Alla deltas har `suggestion`-fält ifyllt i ≥80 % av fallen
- `cargo test --lib core::tests` + `tests/mcp_integration.rs` gröna

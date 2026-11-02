# GTW MCP-rapport från Motor2 — buggar och feature-önskningar

**Datum:** 2026-11-01
**Från:** Motor2-session (roll-först-registry-refaktorn, `model_runtime.rs` ~4 700 rader)
**Kontext:** ~40 GTW-anrop under en större refactor: sök, skeleton, read_node,
edit_node (semantiska funktion-edits + test-omskrivningar). 8/9 semantiska
edits gick via GTW; omvägar och friktion dokumenteras nedan (§35.2-andan —
inget tyst omväge).

---

## Buggar

### B1. edit_node landade på FEL nod efter att ett tidigare edit skiftat paths
**Symptom:** Efter att en testfunktion omskrivits (blev längre) pekade en
inspelad nod-path (`287.2.171`) på fel funktion. `edit_node` med nytt innehåll
skapade en **duplicerad funktionsdefinition** i stället för att ersätta målet —
`E0428 duplicate definition` vid bygge.

**Rot:** Nod-paths är positionsbaserade och ogiltiga efter varje strukturell
edit i samma fil. Varken edit_node-svaret eller något verktyg varnade för att
tidigare utdelade paths blivit stale.

**Önskning:** (a) edit_node-svaret ska returnera nodens NYA path + en
innehålls-hash, (b) en `validate_path`-kontroll: om nodens identifierare
(funktionsnamn) inte matchar förväntat → neka med "path stale, sök igen" i
stället för att skriva fel nod.

### B2. edit_node lämnade remnant av gammalt innehåll (partiell ersättning)
**Symptom:** En edit ersatte ett block men raderade inte alla gamla rader —
resten av den GAMLA koden låg kvar som dubblett strax under den nya (upptäcktes
av kompilatorn, `or_else` på Future). Manuell städning krävdes.

**Önskning:** Efter-skriv-verifiering i edit_node: AST-diff ska visa att
noden ersatts 1:1 (gammal nod borta), inte "nytt innehåll insatt + gamla
fragment kvar".

### B3. get_skeleton på stora filer ger ingen orientering
**Symptom:** `max_depth: 1` på ~4 700 rader → platt lista med 289 noder,
övervägande namnlösa `line_comment`/`use_declaration`-rader. Ingen hierarki,
inget att navigera efter. ("skeleton är large my ass" — användaren, samma
session.)

**Önskning:** Outline-läge: endast namngivna noder (fn/struct/impl/mod/test),
kollapsade kommentarer, ett nivådjup per anrop. Alternativt `kind`-filter.

### B4. search_nodes: multi-mönster ger 0 träffar, enstaka mönster träffar
**Symptom:** `"declared_shadow|multi_role|test_insert_running"` → 0 träffar.
`"test_insert_running"` ensamt → 133 träffar. Alternations-syntaxen är omskriven
eller odokumenterad.

**Önskning:** Dokumentera pattern-grammatiken i tools-schemat; stöd alternation
(eller neka med tydligt fel vid okänd syntax).

### B5. Guardian nekar legitima kontrakt-omskrivningar; ingen force via MCP
**Symptom:** Omskrivning av ett regressionstest (nytt kontrakt: namnbyte +
färre unwraps + omskrivna asserts) blockerades: "removes critical logic…
Use --force to override". `--force` finns ENDAST i CLI:n — MCP-`edit_node`
har ingen force-parameter. Medveten §42-kontrakt-uppdatering av ett test är
därmed omöjlig via MCP-vägen.

**Önskning:** `force: true` (eller `expect: {removed_asserts: N}`-deklaration)
i MCP-schemat — medveten överträdelse deklareras av anroparen, Guardian loggar
den i stället för att blockera till döds.

---

## Features

### F1. Batch med färsk path-upplösning per edit
Mekaniska multi-site-ändringar (samma swap på 10–15 ställen) kräver per site:
search_nodes → read_node → edit_node — tre anrop och much kontext per site.
**Önskning:** `batch_resolve { file, pattern, edit_template }` — sök en gång,
resolva VARJE träff färskt före sin edit, rapportera per site (med B1:s
stale-koll). Detta är skillnaden mellan "GTW för mekanisk omstrukturering" i
teorin och praktiken.

### F2. edit_ask utan modell-laddningstak för skriptade serier
Dokumenterat: första edit_ask-anropet kan ta 20–30 s (modell-laddning). För
långa serier av små edits är deterministisk edit_node att föredra — se F1.

### F3. `list_nodes` med filter + limit per nod-typ
`list_nodes` utan djup-begränsning returnerade 3 133 noder (limit nådd) på
fel nod — oanvändbart sväll. Filter (`kind: function_item`, `name_contains`)
med rimlig default-limit skulle göra nod-navigering billig.

---

## Vad fungerade BRA

- `edit_node` för hel-funktioners-ersättningar med oförändrad signatur: 8/9
  semantiska edits rena och syntax-validerade (note_router ×2, mark_failed,
  residents_snapshot-closure, is_shadow_type, claim_need_gb, loaded_models,
  båda test-hjäparna, multi-role-testet).
- `search_nodes` med enstaka mönster: snabb och träffsäker (133 träffar,
  korrekta dot-paths).
- `read_node` på små noder: exakt och billig — rätt pre-flight före edit.
- Transaktions-/undo-löftet (skadlig skrift avvisas i stället för att
  korrumpera) höll där Guardian slag till — problemet är B5:s döda-lås, inte
  vakten i sig.

## Underlag

- Motor2-commit med refactor där GTW användes: `4e2d1a9` (roll-först-registrering).
- Friktionsfallen är också noterade i Motor2:s `AGENTS.md` §35 (verktygsstatus).

# Handover — 2026-11-18 (IX): code-index exponeringslager byggt, M13, tre återvunna orphan-vågor, claim_scan

**Läget just nu:** allt committat (`cbf744b`), träd rent, installerad binär
0.26.1 = rev `2fb01e2` (claim_scan). Hela sviten grön genom
`scripts/commit-build.sh --skip-commit` → **`RESULT ok`** (fem gånger denna
session, varje arbetsorder en). Ingen release taggad (0.26.1 ute, 0.27.0 ej
påbörjad). **Daemner kör fortfarande den gamla binären** — nya grepp når dem
först vid nästa daemon-omstart (mellan sessioner, `scripts/daemon-hygiene.sh`
enligt vana; kör aldrig `GTW_REAP=yes` mitt i en session).

Läsordning för den som tar över: den här filen → `uppslag.md` (nyast först) →
`docs/CODE_INDEX_AGENT_ADOPTION.md` (sessionens nya doc) →
`docs/CODE_INDEX_SCAN_TOOLS_PROPOSAL.md` (T4-designen) →
`docs/CODE_INDEX_INFORMATIONSMOTOR_UTVECKLINGSPLAN.md` (facit-kravet §9).

---

## 1. Denna session — KLART (9 leveranser, alla live-bevisade)

Utgångspunkt: en utredning ("vad kan Code Index ge LLM-agenter — särskilt
små modeller — eller stora modeller färre tokens?") som mynnade i 10
problemklasser (P1–P10), tilläggen T1–T8/N1–N8 och tre godkända
designbeslut. Därefter byggdes kön av i denna ordning:

**a) Antagandeplanen** (`docs/CODE_INDEX_AGENT_ADOPTION.md`): 3
antagningsfelklasser (upptäckt/förtroende/träningsprior), 6 hävstängar —
starkast är **in-band what-now** (varje svar bär sina nästa steg; varje
vägran ett körbart kommando) + kontraktstest-paritet.

**b) N1 — `kind=chain` + `kind=file_cycles` i MCP** (`41a696a`): koden fanns
implementerad och testad i `code_graph.rs` men nådde inte verktygstabellen.
Bevis: JSON-RPC in i färsk debug-binär, `chain` found:true hops:1.

**c) M13 — medägare-regeln** (`d2bec57`, AGENTS.md 2.4): agenten är
medägare av repots hela tillstånd. Okommittrat arbete som påträffas
**utreds → bevisas → committas med ärlig härkomst** — "någon annans WIP" är
ingen disposition. Tillfälle i fel-listan: denna sessionen själv (855
rader klassades som främmande och lät dirty-grinden blockera release).

**d) Tre orphan-vågor återvunna** (`3f8f576`, `e54efcb`, `9f95cfc`, ~1 700
rader): (1) 2026-11-14: **omprövningsfixen** — `resolve_unresolved_in_db`
omvärderar nu ALLA rader (gamla: `WHERE to_id IS NULL`); stänger
INFORMATIONSMOTOR-planen §1.2. Plus `CallForm`/`SourceAnchor`/
`CallObservation`-klassificering + facit +533 rader. (2) 2026-11-17:
**binding_observations** — fil-lokal Rust/JS bindnings-/operationsanalys,
library-API (5 tester). (3) **GUIE-visionsdokumentet** (279 rader).

**e) T2 — `query_status` på varje query-svar** (`004a7d5`): maskinläsbart
block `{completeness, denominator, index_stale, next_checks,
resolved_edges_only, scope}` — INLINE alltid (beslut: opt-in-ärlighet =
P3-felklassen inbyggd). `index_stale` har EN definition (store rev vs
worktree rev, samma som CLI graph meta). `next_checks` är antagnings-
mekanismen: `index_stale=true` → `mode=refresh` först.

**f) N2+N4 — server-side-filter** (`abc6107`): relations: `relation_type`,
`symbol`-inblandning, klient-`limit`; symbols: `name_prefix` + `external`.
**Kritisk rättelse mitt i beviset:** LIMIT i SQL gjorde `matched` till
post-cap-talet ("matched:5, truncated:false" när 22 matchade — nämnaren
blev tyst topp-N, M7). Fix: matched räknas FÖRE cap
(`matched:22, returned:5, truncated:true` bevisat live).

**g) T5-rest — per-kant-provenans + scopad coverage** (`df26209`):
relations-rader bär `resolution` = `resolved | unresolved_macro |
unresolved_qualified_path | unresolved_plain_name` — coverage-modulens
exakta predikat (en taxonomi, två ytor). "resolved" = GTW:s regel valde
målet, **inte** kompilatorverifierat. `impact` tog `prefix`: coverage-
nämnaren scopas (`path_prefix=src/cli` → 189/2890 mot globalt 3399/26785);
walken scopas inte. Omprövningen själv (T5:a) visades vara redan byggd —
återvunnen i orphan-våg (1) ovan.

**h) T1 — `kind=map`** (`f543342`): tokenbudgeterad, grafrankad
orienteringsyta. In-degree-rankning över resolva mål, tie-break namn+fil,
greedy prefix-fit med `estimate_code_tokens` (återanvänd). **Store-
krävande** per beslut (lexikal fallback = curator-dublett). Determinism
bevisad: två körningar byte-identiska (budget 600 → 582 tokens, 10/3 761
symboler, topp: `write` 166 refs). `next_checks` bär topp-symbolen.
`candidates_total` är **budgetoberoende** (eget test fatade att första
varianten gjorde nämnaren budgetberoende).

**i) T4 #1 — `claim_scan`** (`2fb01e2`, verktyg #39 + CLI `claim-scan`):
doc-claims mot store-fakta. Backticks + parentes-uppräkningar (≥2 komman)
ur `///`/`//!` joinas mot fem sanningsmängder. Fynd = **lie-KANDIDATER**
med fil:rad-ankare + severity (HIGH = kvalificerad väg), aldrig domar
(§8.4). **Dag-1-fynd på GTW självt:** `index_relations.rs:33` lovar
relationstyperna `defines`/`contains` — ingen av dem finns i extraktionen
(23/25 claims verifierade). Samma klass som Motor2-fyndet som födde hela
scan-familjen. Paritet: verktyg 38→39 i `GTW_INSTRUCTIONS.md` i samma
ändring; kontraktstestet `integration_mcp_instructions_listed` grön.

---

## 2. Process — ÄGARENS KORREKTION + två lärdomar med tänder

**M13 praktiserades:** tre orphan-vågor som en tidigare agent klassat som
"någon annans WIP" återvunna samma timme som korrektionen.

**GTW-first-drift (ägar-kritik, lagrad som lektion
`gnawtreewriter:lesson:gtw-first-decay-m11`):** hand-redigering blev norm
för cli.rs/mcp-edits; post-52-falskt-postiv generaliserades tyst till allt.
Fem handredigerings-fel denna session, **alla fångade av grindar eller
probe-utdata, inget tyst** — men drift i sig är felet. Motgård: frågan
"är detta GTW-able?" vid VARJE edit; omväg endast när GTW faktiskt faller,
och omvägen loggas (post 52-familjen). GTW:s vakter räddade sessionen två
gånger (match-arm-detektorn på Ok-mönster; stale-path-detektorn som
stoppade överskrivning av `resolve_symbols` efter structural insert).

**Tekniska regler som faller ut (nästa agent: lär dem):**
1. `insert_node` på **filrot**: position är INTE search-path-index — landade
   i modul-dokumenten (E0753). Content ska vara HELA kompletta items; en
   avslutande nästa-fn-signaturrad swallowar följande funktion (T2-fallet).
   Insättningar i flata moduler: `edit_node` på nod-ankare eller editor.
2. Om-locate nodvägar ALLTID efter strukturella inserts (index skiftar).
3. matched/returned-nämnare: räkna FÖRE cap — aldrig SQL-LIMIT i den
   loader som redovisar matched (M7).
4. String-sortering av severity ("MEDIUM" > "HIGH" lexikografiskt) — explicit
   rank-funktion.
5. Testimports i `#[cfg(test)]`-moduler: importera i modulen, inte på
   toppnivå (unused-imports med `-D warnings`).

---

## 3. Provisoriskt / olöst (ärligt — inget av detta är tyst)

| Post | Läge |
|---|---|
| T1: `signature` är best-effort | djupt nästlade noder saknar items-rad → `signature:null`; node_path kedjar ändå till read_node. Fix: items-extractor-utökning eller annan join |
| T1: rankning = in-degree v1 | PageRank-liknande viktning är möjlig v2 — bara om facit-batteriet motiverar |
| `chain` saknar CLI-flagga | medvetet (N1 var MCP-scope); hör hemma i en kommande CLI-våg |
| N3: reverse reachability | blast radius är halvplan (impact framåt, touches 1-hop bakåt) — osvarbart "vem beror indirekt på X" |
| T7-rest | `entity_changes`/`file_versions` skrivs men läses av ingen action; rena upplösningsändringar fångas inte i historiken (D8) |
| T3 triage-fält | väntar på T4-fynd att fästa på (design SCAN_TOOLS §7) |
| §8.4 falsk-positiv-policy | när eskaleras en claim-kandidat till lint? Ägarbeslut efter användningsdata |
| **Facit-batteriet** (planen §9) | fortfarande olöst — substitutionen ("agenter får det lättare") är arkitektur, inte bevis, tills uppgiftsbatteriet mäter korrekt slutförande + kontrollarbete + tokens per modellklass |
| Adoption-spåret | session-prep-pekare ("nytt repo → kind=map först") + Motor2 §§ enligt adoption-planen §4 — agent-lagers jobb, inte repo-kod |
| claim_scan falsk-positiva | kända klasser: param-namn, kommando-namn, ID-format-prefix (ex. `gtw:` på index_relations.rs:28) — rankade, capped, grep-utbildade; hållbarhetsdata behövs |

---

## 4. Nästa steg (exakta, i ordning)

1. **T4 #2 `island_scan`** (SCAN_TOOLS §3.2): population = pub-symboler ur
   storen; kopplingsbevis = referenser utanför definierande fil med
   test-filer EXKLUDERADE; rankning efter metod-yta; output-standard §4.
   Färsk modul (`src/core/island_scan.rs`), samma mönster som claim_scan:
   core + CLI + MCP + paritet i en ändring + hermetiskt test.
2. **§8.4-beslutet** (policy: kandidat → lint) — ägaren, med
   användningsdata från claim_scan-körningar.
3. **Facit-batteriet** (INFORMATIONSMOTOR-planen §9): ett litet batteri
   verkliga uppgifter ("hitta alla berörda kodställen före en
   signaturändring") mätt med/utan förmågorna per modellklass. Detta är
   villkoret för att "agenter får det lättare" ska vara bevis, inte känsla.
4. **N3 reverse reachability** (M) och **T7-rest** (S–M) — efter facit.

## 5. Verifieringskommandon (kör dessa först — M11: minnet är inte en källa)

```bash
git status --porcelain && git log --oneline -12   # rent, 9 leveranser + artifact-commits
gnawtreewriter code-index . --check               # grön
gnawtreewriter claim-scan . --files src/core/index_relations.rs | grep identifier
#   → "defines" på rad 33 = dag-1-fyndet replikerat
printf '%s\n' \
 '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"probe","version":"0"}}}' \
 '{"jsonrpc":"2.0","method":"notifications/initialized"}' \
 '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"code_index","arguments":{"mode":"query","kind":"map","budget":400,"root":"."}}}' \
 | ~/.cargo/bin/gnawtreewriter mcp stdio 2>/dev/null | grep -oE 'symbols_shown.{0,40}'
#   → installerad binär svarar med kartan (daemner: vid omstart)
scripts/daemon-hygiene.sh                          # mellan sessioner
```

*Skapad 2026-11-18 av sessionens agent (GLM 5.3 Flash); ägare: Tuulikk.
Allt ovanstående är verifierat med kommandon denna session — inga
påståenden utan facit (M1–M13).*

# GTW-fynd från Motor2 — 2026-11-12 (uppföljning till FEEDBACK_FRAN_MOTOR2 + SVAR-TILL-MOTOR2)

Två fynd från Motor2-sidans F2d-adaptation mot 0.26 (GTW-agentens svar
`docs/SVAR-TILL-MOTOR2-2026-11-12.md` mottaget och behandlat — punkt 1:s
grammatikregression bekräftad fixad i vårt test vi där vi byggde mot
0.26.1-träet). Dessa två är **nya**, upptäckta under samma adaptation.
Full repro-kontext: Motor2 `docs/VERIFIERING.md` XC (Codex-handovern
2026-11-12) och Motor2 `crates/motor2-gtw/src/lib.rs` (F2d, kommentar-
radering före funktionsradering).

## 1. `Delete` på ett radspann kan sluka en funktionsrad

Scenario: fil med doc-kommentar (rad 6-7) + funktion (rad 7-9) i en
10-rads fil. Motor2:s wrapper raderar doc-kommentarnoden först, sedan
funktionens nod. **Observerat:** raderingen av kommentarens nod kan
ta med funktionens första rad — "sluka" den — så filen förlorar kod,
inte bara kommentar.

Misstänkt område: range-validering/-semantik när raderingens radspann
gränsar till nästa items start (off-by-one i exklusiv/inklusiv slut-
hantering), eller om-parsning mellan två raderingar i samma edit-
sekvens där raden räknas om mot en mellan-läsning.

Motor2:s sida adaptade (vi raderar kommentarerna FÖRE funktionsnoden,
varning vid varje misslyckande) — men **andra konsumenter** som raderar
"kommentar + nod"-par i 0.26 kan sluka kod tyst. Värt en core-granskning
av delete-range-gränserna.

## 2. `insert_node` med position utanför modulen rapporterar framgång

`insert_node(position=999)` (position > nodantal) rapporterar **framgång**
men placerar innehållet **efter modulens slut** — utanför det parsade
trädets giltiga region. Ingen validering att position ≤ antal topp-nivå-
noder/antal barn.

Motor2 fann det via F2d-prov (Codex-probet position=999). Förslag:
validera position mot faktisk barn-antal och neka/klampa med ärlig
notis — eller dokumentera klamp-semantiken uttryckligen i verktygets
beskrivning.

## Kontext

- Båda fynden är i GTW core/MCP — inte Motor2:s wrapper-logik (vår sida
  adaptad och grön: motor2-llama 173/173, workspace Clippy, release-bygge).
- Full evidens: Motor2 `docs/VERIFIERING.md` XC + Motor2-gtw-testet
  `delete_node_removes_function_with_full_protection` (röd mot 0.26.0,
  grön mot vår adaptering, se Motor2 commit `16875dd`).

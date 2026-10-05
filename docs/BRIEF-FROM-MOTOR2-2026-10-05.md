# Briefing till GTW-agenten — från Motor2-session 2026-10-05

**Från:** Motor2-kodning-session (verifiering av GTW 9.5/9.6 mot live-daemon)
**Mottagare:** agenten som arbetar på GTW-problemen
**Fullständiga detaljer:** `GTW_MCP_ISSUE_LOG.md` → posten "2026-10-05 — Motor2-verifiering" (repro-tabell + bedömning enligt mall)

---

## 1. Körda mot: aktuell 0.13.0 (inte stale)

MCP-daemon PID 3259463 (Motor2-sidan, startad 2026-10-05 17:59), exe = live
`~/.cargo/libexec/gnawtreewriter`, version 0.13.0 = repo-HEAD e4030bf.
Fynden nedan gäller alltså SENASTE release — inte en gammal binär.

## 2. Bugg 1 (allvarligast): parse-avvisning av GILTIG Rust

- `get_skeleton` + `analyze` på `crates/motor2-session/src/database.rs`
  (Motor2-repot, 11 476 rader) → "Syntax error at line 6097, col 72".
- Raden ÄR giltig Rust (cargo build --release RC=0):
  `serde_json::from_str::<serde_json::Value>(&raw).unwrap_or_else(|_| serde_json::json!({ "raw": raw }))`
  — turbofish-generic i metodkedja med closure-fallback är primär misstanke.
- **Inte storleksrelaterat:** bridge/tests.rs (6 371 rader) parse:ar fint
  (500 noder).
- **Verkan:** HELA kedjan (analyze → sense → read_node → edit_node) är död
  för drabbade filer — ingen GTW-verktyg kan redigera dem.
- **Önskade riktningar:** (a) repro-test för konstruktionen, (b) partial-
  grace-parse — giltig kod får aldrig avvisas hel-fil; posta felet, returnera
  det som gick.

## 3. Bugg 2: satellit-sense och MCP-index är två skilda index

- `sense` utan file_path → "no matches ... build it with `ai index`" —
  även EFTER MCP `index_entities` (56 entiteter indexerade, 0 errors).
- `sense` MED file_path fungerar bra (5/5 relevanta träffar).
- **Gapet:** agenten som indexerar via MCP förväntar sig att sense sedan
  svarar. Antingen gemensamt index, eller att MCP-indexeringen explicit
  heter "strukturindex only" i beskrivningen.

## 4. 9.5/9.6-KLAR-markeringen (ROADMAP ✅ 2026-10-04) är halvsann

- 9.6 "guidande" — **levererad**: sense ger remediation-hint, skeleton ger
  explicit fel i stället för tyst tomt. Detta syns live.
- 9.5 — symptomet ("tomt svar") borttaget, men parse-robustheten saknas
  (bugg 1 ovan). Rekommendation: öppna posten igen som "parse partial-grace".

## 5. Meny från Motor2:s awareness-plan (plan-system-awareness-och-
   transparens-2026-10-03.md) — GTW-raderna, oprörda:

| Plan-# | Åtgärd |
|---|---|
| #19 | Semantic-transparens: top-k noder + confidence vid semantic_edit träff/miss |
| #23 | Duplex Loop-metriker (propose/validated/applied, persistade) + retry-med-feedback (mata tillbaka AST-felet för EN reparationsrund — små modeller gynnas mest) |
| #24 | `history`/`stats` MCP-verktyg mot TransactionLog (datan finns redan) |
| #25 | Felklass-register: stabila koder på edit-fel → Motor2 aggregerar i `tools.editing` |

Motor2-sidiga P0 (för din info, redan lösta där): show_node-verktyget finns
nu i motor2-gtw med tvåvägs prompt-kontraktstest; duration_ms mäts redan i
ToolRegistry::execute (planens #3 var falskt larm).

---
*Skapad av Motor2-sessionen 2026-10-05. Uppdatera issue-loggen, inte denna
fil, när buggarna landar.*

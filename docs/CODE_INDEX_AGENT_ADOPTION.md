# CODE INDEX — antagandeplan för agentförmågor

**Status:** Plan (design, ej byggd) · **Datum:** 2026-11-18
**Proveniens:** GTW-session 2026-11-18 — utredningen "Code Index som kognitiv stödyta för
LLM-agenter" (10 problemklasser P1–P10, tilläggen T1–T8, N1–N8). Designbesluten godkända
av ägaren samma dag; antagandeplanen fördes sedan in i repot på ägarens begäran.
**Underlag:** [CODE_INDEX_SCAN_TOOLS_PROPOSAL.md](CODE_INDEX_SCAN_TOOLS_PROPOSAL.md)
(scan-familjen + triage §7), [CODE_INDEX_INFORMATIONSMOTOR_UTVECKLINGSPLAN.md](CODE_INDEX_INFORMATIONSMOTOR_UTVECKLINGSPLAN.md)
(status-kontraktet §3, exponering §5, facit-batteriet §9), [CODE_INDEX_SQLITE_DESIGN.md](CODE_INDEX_SQLITE_DESIGN.md)
(store-regler, ägarskapsregel decision #8). Minnesservern:
`GnawTreeWriter:analysis:code-index-llm-support-utredning`,
`GnawTreeWriter:decision:code-index-exposure-design`, `GnawTreeWriter:analysis:agent-adoption-plan`.
**Omfattning:** hur agenter (OpenCode först, därefter Motor2) faktiskt börjar använda
förmågorna — `kind=map` (T1), undersökningsstatus (T2), triage-fält (T3), scan-familjen
(T4) — plus de redan befintliga ytorna (impact/touches/history) som i dag underanvänds av
samma skäl som planen behandlar.

---

## 1. Problem: förmåga utan antagande är dekoration

Repots egen läxa (uppslag #17): "`code_index --check` fanns men kördes inte i `validate.yml`
— en grind ingen går igenom är dekoration." Samma klass hotar varje ny code_index-förmåga:
dataskiktet är moget, men agenternas beteende styrs inte av vad som finns i verktygstabellen
utan av tre felklasser — alla bevissatta i repots historia:

| Felklass | Sättet den visar sig | Bevis |
|---|---|---|
| **Upptäckt** — agenten vet inte att verktyget finns | Nya verktyg tillkommer men instruktionsytan är från förra versionen | Crush körde v0.9.2-instruktioner mot 0.25.x (drift-fyndet); `commands`-driften 28→54 (utvecklingsplan §1.4) |
| **Förtroende** — agenten vet inte vad svaret *kan fastställa* | Anropar, får ett svar den inte litar på, återgår till grep | create_goal-fallet: tomma `incoming` krävde att Motor2-agenten *läste sig till* varför (uppslag 2026-11-18) |
| **Träningsprior** — grep/Read är djupt inlärd | Väljer alltid den kända vägen när den fungerar | Alla agent-ytor; inte dumhet utan statistik |

**Nyckelinsikt:** adoption vins inte genom tvång utan genom att göra nya vägen till den
billigaste — och låta verktygen själva lära ut resten.

---

## 2. Hävstängar (ordnade efter robusthet)

### 2.1 In-band what-now — den starkaste hävstången

Varje svar bär sina egna nästa steg (`next_checks`-fältet i T2-statusblocket). Detta är den
enda mekanismen som fungerar **efter första anropet oavsett vad agenten har läst** — svaret
självt lär ut verktyget. Mönstret är bevisat i drift: `undo` → "använd `undo --files`",
`heal` → `--apply`, rotvakten → verktygsspecifikt råd (`af3fa63`). Regeln är testbar:
**varje vägran måste innehålla ett körbart nästa steg.** Kostnad: noll extra turer.

```json
"query_status": {
  "scope": "src/",
  "completeness": "bounded",
  "denominator": "2312 matched, 5000 returned, 0 truncated",
  "resolved_edges_only": true,
  "index_stale": false,
  "next_checks": ["kind=touches symbol=X", "mode=coverage"]
}
```

### 2.2 Beskrivning som fråge-mappning, inte funktionslista

Agenten väljer verktyg utifrån "vilken fråga passar beskrivningen". Nya ytor beskrives i
befintlig WHAT/WHEN/RETURN/EXEMPEL-form — men med **gränsen som förstklassigt fält**
(utvecklingsplan §5): "upplösta kanter enligt GTW:s regel", "endast inom vald sökgräns",
"saknat lokalt mål är inte bevisad extern import". För `kind=map`:
*"billigaste sättet att orientera i ett okänt repo — ett anrop ~1k tokens i stället för
N filläsningar; börja här, inte med grep."*

### 2.3 Intent-triggers i OpenCode — läs-intent är luckan

Edit-intent täcks redan (gnawtreewriter-skillen auto-injiceras vid edit-anrop). Men
**läs/undersök-intent saknar trigger** — och code_index-frågorna är just läs-intent.
Fixen: `session-prep`-skillen (som redan firear på "undersök/planera/jobba med") får
en pekare: *"nytt repo eller ny modul? → `code_index kind=map` först (1 anrop), sedan
`touches`/`impact` — grep är sista utvägen, inte första."* En rad, ingen ny skill.

### 2.4 Kontraktstest-paritet — instruktionen ljuger annars

Etablerat mönster, utökas mekaniskt per ny förmåga:

- `GTW_INSTRUCTIONS.md` uppdateras **i samma ändring** som verktyget
  (`integration_mcp_instructions_listed`-mönstret).
- Verktygstabellen i `GTW_INSTRUCTIONS.md` får raderna (38-verktygskontraktet).
- `pack-skill.sh --check` skyddar Crush-projektionen.
- Scan-förslaget §5 hade rätt: en instruktions-tabell som ljuger är *samma hål-klass som
  verktygen själva hittar* — och `claim_scan` kan en dag köra den kollen på egen hand.

### 2.5 Session-defaults — mjuk salience

`doctor` rapporterar redan store-status och kan nämna "store fresh, `kind=map` available".
`memory_context` pekar vid session-start. Mjuk salience, inte tvång — inget av detta
blockerar något.

### 2.6 Mät antagandet, inte känslan

Duplex-räknarna och sök-loggen finns redan. Nya mått: andel orienteringsfrågor besvarade
med `map` vs grep-kedjor; Pass^N-batteriet (utvecklingsplan §9) med/utan förmågan, per
modellklass. Regeln står: *ett gränssnitt som bara kortar svaret utan att förbättra korrekt
slutförande har oprövad nytta.*

---

## 3. Host-matris: en källa, flera projektioner

Antagande-instruktionen skrivs **en gång** (i `guide`-katalogens maskinläsbara källa) och
projiceras till varje host-yta — aldrig handskriven dubbelt (Crush-drift-läxan):

| Host | Yta | Mekanism |
|---|---|---|
| OpenCode | MCP-beskrivningar | §2.2 — fråge-mappning med gränsfält |
| OpenCode | `session-prep`-skillen | §2.3 — läs-intent-pekaren |
| OpenCode | `GTW_INSTRUCTIONS.md` | §2.4 — kontraktstest-paritet |
| Crush | `gnawtreewriter-crush.skill` | genererad av `pack-skill.sh`; `--check` i CI |
| Cline / Codex / VSCode | host-egna instruktionsytor | samma källa, host-formattering |
| Motor2 | Motor2:s AGENTS.md §§ + gnawdriver-skillen | sektion 4 nedan |
| Alla | verktygens svar själva | §2.1 — in-band what-now |

---

## 4. Motor2-specifikt — ut över OpenCode

Motor2-agenter styrs av **Motor2:s AGENTS.md §§ och gnawdriver-skillen** — inte av GTW:s
dokument. Adoption där följer samma lag som fick gnawdriver-flödena använda: verktygen
namnges i de operativa dokumenten för de vanliga situationerna, till exempel:

- *"handover skrivs → `code_index kind=changed` + `kind=history` ger delta-grunden,
  inte minnet."*
- *"triage av fynd → scan-verktygens triage-fält (`priority_score`, `fix_order`,
  `moot_after`) — granska, motivera, flagga osäkerhet; eskalera ('ovanför min löneklass')
  är ett giltigt verdict."*
- *"vad beror på X före refaktor → `kind=impact` + `kind=touches`; grep är
  facit-kontrollen, inte sökaren."*
- *"orientering i ny modul → `kind=map` (1 anrop, ~1k tokens) före läsning."*

Motor2 var konsumenten som födde flera av förmågorna (extensions-hålet, create_goal,
16 MiB-incidenten) — dess §§ är därför den naturliga antagandeytan för scan-familjen (T4).

---

## 5. Vad som uttryckligen inte ska göras

1. **Inget permission-tvång** (blockera grep för strukturfrågor) — klientlagrets beslut;
   skapar motstånd, inte adoption; straffar de fall grep är rätt verktyg.
2. **Ingen instruktions-bloat** — AGENTS.md är redan stor; trötta modeller ignorerar prosa
   de inte behöver just nu. In-band what-now (§2.1) vinner över mer skriven vägledning.
3. **Inga andra sanningskällor** — beskrivningar/instruktioner genereras ur registreringarna,
   omformuleras aldrig per host.

---

## 6. Sekvens med byggplanen

Antagande-arbetet är nästan gratis **när** det byggs samtidigt med förmågan (en
beskrivningsrad, en skill-rad, en kontraktstest-rad) och dyrt att retrofitta:

| Ordning | Bygge | Antagande-del i samma ändring |
|---|---|---|
| 1 | T2 undersökningsstatus | Antagningsmekanism #1 själv — `next_checks` gör varje senare verktyg självantagande |
| 2 | N1 exponera chain/file_cycles + N2/N4 filter | beskrivningar + `GTW_INSTRUCTIONS.md`-paritet |
| 3 | T1 `kind=map` | session-prep-pekaren + orienterings-rådet (det blir först då sant) |
| 4 | T4 scan-familjen | Motor2-§§ (Motor2 är konsumenten som bad om dem) + triage-ekot |
| 5 | löpande | antagande-räknare i metrics; granskas vid varje facit-batterikörning |

---

## 7. Öppna frågor

1. **`guide` som generatorkälla:** kan situation→verktyg-katalogen (redan MCP) bli den
   maskinläsbara källan för §3-projektionerna, eller behövs ett eget register
   (utvecklingsplan §7:s förmågekarta)? Beslutas när T2 landar.
2. **Antagande-räknarnas plats:** duplex-räknarna (`.gnawtreewriter_metrics.json`) eller
   egen räknare per yta — och vad som är baslinjen för "andel orienteringsfrågor via map".
3. **Motor2-§§-författandet:** skrivs av GTW-repot som förslag (denna fil) och godkänns i
   Motor2 — tvärgående flöde som behöver en ägare på varje sida.
4. **session-prep-pekarens formulering:** den ska eka sin tolkning (uppslag-läxan från
   2026-11-11) — peka på förmågan, aldrig gissa intentet tyst.

---

*Levande dokument — uppdateras när förmågorna byggs och antagande-måtten börjar komma in.
Varifrån: GTW-session 2026-11-18 (utredning + beslut), minnesservern: tre poster under
`GnawTreeWriter:analysis/decision:code-index-*` och `:agent-adoption-plan`.*

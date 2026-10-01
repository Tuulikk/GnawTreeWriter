# GTW MCP — fel- och avvikelselogg

> Syfte (användaren 2026-09-30): när GTW-strular i riktig användning ska
> det PÅPEKAS här, så att vi senare kan avgöra om det är (a) verktyget
> använt fel, (b) bugg, (c) saknad funktion, eller (d) verktyget lever
> under vad det borde. GTW är en central del av Motor2-kedjan och vi är
> ansvariga för projektet — dolda strul = dött organ.
>
> Format: datum | anrop (parametrar) | förväntat | faktiskt | kontext |
> omväg som användes | bedömning (fylls vid triage).

---

## 2026-09-30 — alla tre GTW-anropen timeade under U2-arbete (Motor2)

**Kontext:** U2 (vakt-report) i Motor2-sessionen; jag sökte var
`/api/v1/logs`-endpointen ligger och system.rs-struktur — GTW:s
kärnförebild (skeleton + semantisk sökning istället för grep).

| # | Anrop | Parametrar | Resultat |
|---|---|---|---|
| 1 | `sense` | query="where is the /api/v1/logs endpoint implemented…" (utan file_path — projekt-scope) | **MCP error -32001: Request timed out** |
| 2 | `sense` | samma query + file_path=system.rs (zoom-läge) | **MCP error -32001: Request timed out** |
| 3 | `get_skeleton` | file_path=system.rs, max_depth=2 | **MCP error -32001: Request timed out** |

3/3 anrop misslyckades inom ~en minutes fönster. Föregående session
(används ej GTW där — grepp + Edit var mönstret, vilket användaren
påpekade).

**Omväg som användes i stället:** grep (hittade `system::logs_handler` +
routen i main.rs) och Read (logs_handler-kroppen). Fungerade direkt —
men grep:en är exakt det mönstret som missade coach-gaten tidigare
samma dag (falskt negativ: `\.coach\b`-mönstret matchade inte
`self.config.semantic.coach`-användningen i en annan fil), vilket var
skälet att gå till GTW från början.

**Bedömning (fylls vid triage):**
- [ ] Använt fel? (t.ex. MCP-servern ej startad/i dåligt läge i hosten,
      stor fil, nätverk)
- [ ] Bugg? (timeout-tröskel för låg för skeleton/sense på ~860-raders
      fil / projekt-scope-sökning)
- [ ] Saknad funktion? (ingen timeout-feedback/kpartiell respons; ingen
      retry-semantik)
- [ ] Under förmåga? (sense borde ha hittat coach-användningen tidigare
      samma dag — opreparerat eftersom anropen då inte gjordes)

**Notera även (positivt/OBS):** inga GTW-anrop lyckades denna session
än så länge — jämför med tidigare sessioner där GTW använts med framgång
enligt GTW_INSTRUCTIONS.md. När GTW svarar: testa om sense hittar
`semantic.coach`-kopplingen i bridge/mod.rs (den grep-mönstret missade).

**Uppföljning samma dag (omgång 2, ~20 min senare):**

| # | Anrop | Parametrar | Resultat |
|---|---|---|---|
| 4 | `get_skeleton` | paths.rs (motor2-core, ~530 rader), max_depth=1 | **OK** (svarade; skelett-innehåll ytterst kort i svaret — se not nedan) |
| 5 | `list_nodes` | system.rs (motor2-server, ~1170 rader efter dagens edits) | **OK** — "Found 1121 nodes (limit reached)" |

Tolkning hittills: timeouter var transienta eller filstorhets-/last-
beroende (samma fil som timeade med get_skeleton svarade med list_nodes;
mindre fil skeleton:ade fint). 1121 noder i EN fil är samtidigt en signal:
node-gränsen nås i normalstora Rust-filer → skeleton/sense på sådana filer
kan vara tyngre än verktyget gynnsamt klarar inom timeout. Nya frågor för
triage: (a) vad är timeout-tröskeln och är den konfigurerbar? (b) bör
sense/skeleton paginera vid node-tak i stället för att riskera timeout?
(c) skeleton-svaret innehöll knappt synbart innehåll — är det förväntat
för max_depth=1?

---

<!-- Ny post: kopiera mallen nedan
## ÅÅÅÅ-MM-DD — kort rubrik
**Kontext:**
| # | Anrop | Parametrar | Resultat |
**Omväg:**
**Bedömning:** [ ] användarfel [ ] bugg [ ] saknad funktion [ ] under förmåga
-->

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

## 2026-10-01 — reproduktion: timeout → "Not connected"; sense projekt-scope degenererad

**Kontext:** Användaren förvånad ("verktyget har fungerat tidigare"). Systematiskt
reproduktionsprov ~14:46–14:51, load 5–7.5. Innan provet städades 2 föräldralösa
GTW MCP-processer (pts/2 och pts/3 saknades i `who`); pts/1, pts/4, pts/5 tillhör
levande sessioner och lämnades.

| # | Anrop | Parametrar | Resultat |
|---|---|---|---|
| 1 | `get_skeleton` | cli.rs (GnawTreeWriter-repo, liten fil), max_depth=1 | **OK** — svarade direkt, men svaret var bara rubrikraden; skelett-innehåll tomt (samma not som omgång 2) |
| 2 | `sense` | query="where is the /api/v1/logs endpoint implemented" (projekt-scope, exakt samma query som 2026-09-30) | **"Satelite search results"** — svar men tomt/degenererat. Tyst misslyckande: ingen timeout, inga träffar, inget fel |
| 3 | `sense` | samma query + file_path=system.rs (zoom-läge) | **MCP error -32001: Request timed out** (REPRODUCERAT) |
| 4 | `get_skeleton` | system.rs (nu 1116 rader), max_depth=2 | **MCP error -32001: Request timed out** (REPRODUCERAT) |
| 5 | `list_nodes` | system.rs | **"Not connected"** (fungerade i omgång 2 på samma fil!) |
| 6 | `get_skeleton` | system.rs, max_depth=1 | **"Not connected"** |

**Nyckelfynd:**
1. **Inte transient.** Samma anrop som timeade 2026-09-30 timeade igen en dag
   senare under liknande last — exakt reproducerbart.
2. **Timeout → "Not connected".** Efter två timeouter tappar hosten anslutningen.
   Processen (PID 797441, pts/5) lever kvar: alla trådar i `futex_do_wait`
   (blockerad, icke-spinning). Hängt dead-MCP-läge utan återanslutning inom
   provfönstret.
3. **Mönstret är process-tillstånd, inte filstorlek ensamt.** list_nodes klarade
   samma fil i omgång 2 men ger nu "Not connected"; små-fil-anrop fungerar.
   Hänget smittar hela servern, inte bara det anrop som triggar det.
4. **Resursackumulering.** 511 MB RSS + 9:27 CPU-tid på ~5,5 h (startad 09:20,
   U2-arbete). RSS låg konstant mellan två mätningar under häng — läckan
   byggdes tidigare, inte av själva hänget.
5. **Degenererat sense-svar.** Projekt-scope sense returnerar
   "Satelite search results" (stavfel = intern etikett som läckt till UI?)
   i stället för träffar eller korrekt "inga träffar". Farligare än timeout:
   agenten kan tro att sökningen faktiskt gjordes.

**Omväg som användes i stället:** grep/Read för system.rs-fynd (som vanligt),
`ps -L`/wchan för tråddiagnos. Fungerade direkt.

**Bedömning (fylls vid triage):**
- [ ] Använt fel? — mindre troligt nu: identiska anrop fungerat tidigare,
      reproducerbara över två dagar
- [ ] Bugg? — troligast, tre separata symptom: (a) häng i sense/get_skeleton
      på stora filer/projekt-scope → host-timeout → anslutning dör utan
      återanslutning; (b) minnesackumulering 511 MB/5,5 h; (c) tomt
      "Satelite search results"-svar från projekt-scope sense
- [ ] Saknad funktion? — delvis: ingen timeout-feedback/partiell respons,
      ingen återanslutning eller hälsoproba i MCP-läge
- [ ] Under förmåga? — projekt-scope sense borde hitta eller rapportera
      "inga träffar", aldrig en tom etikettrad

**Nästa steg:** (1) döda hängd process (797441) → host spawnar ny, validera
små-fil-anrop; (2) bevaka RSS på färsk process under normalt arbete;
(3) replikera hänget fristående med scriptad JSON-RPC mot
`gnawtreewriter mcp stdio` + RUST_BACKTRACE=1; (4) kolla timeout-tröskeln
i opencode-config mot verklig körtid för projekt-scope sense.

---

## 2026-10-01 — ROTSAK FYNNAD + ÅTGÄRDAD: panik, per-anrop-model reload, obegränsad JIT-cache

**Kontext:** Uppföljning av omgång 3 — binär uppdaterad 0.9.5 → 0.9.7 enligt
användarens direktiv ("binär behöver vara senaste"). Reproduktion fristående
(utan MCP-host) avslöjade hela kedjan. Obs: under arbetet upptäcktes även att
/home var 100 % full (SIGBUS i länkaren) — 28 GB regenererbar bygg-cache
städdades; diskfullhet kan ha bidragit till tidigare strul.

**Rotorsaker (alla bekräftade i kod):**
1. **Panik på UTF-8-char-boundary** — `gnaw_sense.rs:307` (`&node.content[..97]`,
   byte-slice) kraschade på `—` i Motor2 system.rs. Paniken avslutade
   stdio-läsloopen → "Not connected" (omgång 3 fynd 2).
2. **Ny broker per MCP-anrop** — `handle_sense` skapade `GnawSenseBroker::new`
   varje call: ModernBERT modellen + JIT-cachen laddades/skippades om varje
   gång → 60 s+ per anrop, aldrig cache-träff (förklarar varför omgång 2:s
   "fungerar ibland" bara gällde små filer).
3. **Obegränsad JIT-cache** — `HashMap` utan eviction; 511 MB RSS på 5,5 h
   (omgång 3 fynd 4).
4. **Obegränsad nod-embedding** — varje definitions-nod embeddades, upp till
   hela modellkontexten: jättenoder (>8192 tokens) kraschade modellen hårt
   (`inconsistent last dim size in rope`, reproducerat på GTW:s egen cli.rs)
   och CPU-attention är kvadratisk → minuter på stora filer.
5. **Degenererat svar** — "Satelite search results" utan antal/tomhets-ledtråd
   (omgång 3 fynd 5).

**Åtgärder (committa separat, se git status):**
- `gnaw_sense.rs`: char-safe `truncate_preview()` (± enhetstester), 
  `MAX_EMBED_NODES=24`, `MAX_EMBED_CHARS=1200`, JIT-cache-cap 32 filer med
  eviction.
- `mcp/mod.rs`: delad broker i `AppState` via `OnceCell` (modell + cache
  lever genom serverns livstid), panik-isolering per request (`tokio::spawn`
  i både stdio- och HTTP-loopen → panik blir JSON-RPC internal error i stället
  för död anslutning), informativt Satelite-svar med antal + tomhets-ledtråd.
- `parser/xml.rs`: 2 pre-existerande clippy-varningar (question_mark) städade.

**Validering (2026-10-01, belastad maskin, load 5–7):**
| Prov | Före (0.9.5) | Efter (0.9.7) |
|---|---|---|
| sense zoom system.rs (1116 rader) | timeout 60 s → panik → Not connected | **OK på 23,6 s** (CLI) / **30,4 s** (MCP e2e) |
| sense cli.rs (jättenoder, rope-krasch) | modellfel vid 34 s | **OK på 27,5 s** |
| get_skeleton system.rs direkt efter sense | timeout (könad bakom sense) | **OK på 0,1 s** |
| små filer (14 rader) | OK | OK 0,96 s |
| cargo test | — | 8/8 sviter gröna; clippy -D warnings ren; cfg-paritet OK |

**Bedömning:**
- [x] Bugg — alla fem rotorsaker kodifierade och åtgärdade
- [x] Saknad funktion — panik-isolering + delad broker tillför saknad robusthet
- [ ] Använt fel — nej; anropen var legitima
- [ ] Under förmåga — löses delvis (bättre felmeddelanden); sense-kvalitet
      på cap:ad index är "lokalisering", inte fullständig — dokumenterat i
      konstanternas doc-kommentarer

**Kvar att bevaka:** (a) första sense-anropet i en färsk MCP-process betalar
modellladdning (~10–30 s) — hostens timeout måste vara ≥60 s; (b) RSS på
långlivad process med cache-cap 32 (bör ligga ~<150 MB); (c) projekt-scope
sense kräver fortfarande `ai index` för träffar — felmeddelandet ledar nu
dit.

---

## 2026-10-01 (II) — get_skeleton tomt svar på 856-raders JS-fil

**Kontext:** Motor2 dashboard-polering. Behövde strukturen på
`pkg/gede/project-map.js` (856 rader JS) före edit.

| # | Anrop | Parametrar | Resultat |
|---|-------|------------|----------|
| 1 | get_skeleton | file_path=project-map.js, max_depth=2 | Endast rubrikrad i svaret — tomt skelett, inget fel |
| 2 | grep (omväg) | mönster direkt i filen | Fungerade — hittade selektorer/radnummer |

**Omväg:** grep + read med offset.

**Bedömning:** [ ] användarfel [x] bugg [ ] saknad funktion [ ] under förmåga
Samma fil-storleks-mönster som första posten (Motor2-U2: 1121-noders
system.rs). JS ger troligen extremt bred trädtopp (IIFE + objektliteral
→ funktioner på depth 6–7), men TOMT svar utan felindikering är ett
verktygsfel: should returnera fel eller begränsat skelett, aldrig tyst
tomt. Förstärker triage-frågan (b): paginering/degnereringssignal vid
stora träd.

---

<!-- Ny post: kopiera mallen nedan
## ÅÅÅÅ-MM-DD — kort rubrik
**Kontext:**
| # | Anrop | Parametrar | Resultat |
**Omväg:**
**Bedömning:** [ ] användarfel [ ] bugg [ ] saknad funktion [ ] under förmåga
-->

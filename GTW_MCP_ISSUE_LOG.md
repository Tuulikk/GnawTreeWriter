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

**Bedömning (triage stängd 2026-10-04, ROADMAP 9.5d):**
- [x] Använt fel? — NEJ: server startad, anrop legitima, symptomen
      reproducerbara över flera dagar
- [x] Bugg? — JA: timeout-tröskel/filstorlek på stora filer → åtgärdad i
      0.9.7/0.9.8 (panik-isolering, storlekscaps, delad broker — se
      valideringstabellen i 2026-10-01-omgången nedan)
- [x] Saknad funktion? — JA (dels) → åtgärdad: panik-isolering ger
      JSON-RPC-fel istället för död anslutning (0.9.7); `get_skeleton`
      returnerar nu explicit `truncated`-flagga och kan aldrig svara tyst
      tomt (9.5a); `doctor` svarar "är GTW vid liv" i ett anrop (9.5c)
- [x] Under förmåga? — DELVIS, kvarstår öppet: sense-kvalitet på cap:ad
      index = "lokalisering", inte fullständig sökning (dokumenterat i
      konstanternas doc-kommentarer; satellit-ranking toppas av
      moduldeklarationer — se 2026-10-01-noten)

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

**Bedömning (triage stängd 2026-10-04, ROADMAP 9.5d):**
- [x] Använt fel? — NEJ: reproducerbart över två dagar, anropen legitima
- [x] Bugg? — JA: alla tre symptomen bekräftade → rotorsaker fanns och
      åtgärdades direkt efter denna triage (nästa post: panik på
      char-boundary, ny broker per anrop, obegränsad JIT-cache — alla fem
      kodifierade i 0.9.7, stabiliserade i 0.9.8)
- [x] Saknad funktion? — JA (dels) → åtgärdad: panik-isolering per
      request + återanslutningsvänliga fel (0.9.7); idag även
      `doctor`-diagnostik och `truncated`-signal (9.5)
- [x] Under förmåga? — JA på punkten "tom etikettrad": satelit-svaret
      rapporterar numera antal + tomhets-ledtråd, aldrig en naken etikett
      (0.9.7). Kvarstår öppet: själva träffkvaliteten (se2026-10-01)

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
- [x] Använt fel — nej; anropen var legitima (triage stängd 2026-10-04)
- [x] Under förmåga — delvis: felmeddelanden bättre; sense-kvalitet på
      cap:ad index är "lokalisering", inte fullständig — dokumenterat i
      konstanternas doc-kommentarer. Kvarstår öppet som känd brist
      (adoption: satellit-ranking prioriterar moduldeklarationer)

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

## 2026-10-01 — UTF-8-audit efter omgång 4: 4 fler byte-slice-bomber funna + åtgärdade

**Kontext:** Användaren: "UTF-8-fixar är redan gjorda — finns fler gömda?"
Systematisk audit (`rg` efter `&x[..N]`, `[x.len()-N..]`, byte-baserad
chunking) bekräftade misstanken. Dagens panik var alltså inte en enstaka
slarveri utan en klass.

| # | Fil:rad | Mönster | Risk | Åtgärd |
|---|---|---|---|---|
| 1 | `llm/pipeline.rs:460` | `capped[..3000]` på godtycklig fil-content | panik (evidens-läsning i AI-pipeline) | chars().take(3000) |
| 2 | `llm/project_indexer.rs:118` | `&content[..97]` — exakt kopia av dagens bugg | panik (project index-preview) | delad `truncate_preview()` |
| 3 | `llm/project_indexer.rs:108` | `&chunk[..len().min(100)]` | panik | truncate_preview |
| 4 | `llm/project_indexer.rs:146-148` | `chunk_text` BYTE-baserad (`text[start..end]`) | panik på CJK/emoji >8000 tecken | char-baserad (Vec<char>) ± tester |
| 5 | `llm/project_indexer.rs:96` | chunk-tröskel 15000 tecken vs rope 8192 tokens | modellkrasch (CJK ≈ 1 token/tecken) | tröskel 8000, chunks 4000/500 |
| 6 | `cli.rs:4061` | `&first_line[..22]` | panik (removed-preview) | chars().take(22) |
| 7 | `core/secrets.rs:292` | `&secret[len-4..]` | panik (defensiv) | Vec<char>-redact |

**Granskade men OK:** `mcp/mod.rs:1398` (`git status --porcelain` — ASCII-prefix
per formatet), `cli.rs:3026` + `core/report.rs:46` (hex-txn-ID:n), 
tree-sitter-offset-slicear (giltiga UTF-8-gränser per parserkontrakt).

**Validering:** clippy -D warnings ren, 155+ lib-tester gröna (inkl. nya
chunk_text-regressionstester), binär 0.9.7 ominstallerad, sanity-prov OK.

**Disciplin-notering:** byte-slicear på användarkontroll strings ska aldrig
skrivas — `truncate_preview()` (gnaw_sense) eller `chars().take(n)` är
mönstret. Kvar i registry: att lägga till en lint-regel (gnawtreewriter
add_rule) som flaggar `&X[..N]` på strängar i denna kodbas.

---

## 2026-10-04 — driftprov av sense (e.1): stabilitet OK, men MCP-ytan
dropper träffarna

**Kontext:** Motor2-agenten körde det planerade driftprovet av 10-01-fixen
(3afa725) innan punkt 6 kunde betraktas som stängd. Fråga med känt svar:
"logs endpoint handler" → `crates/motor2-server/src/system.rs:45`.

| # | Anrop | Parametrar | Resultat |
|---|-------|------------|----------|
| 1 | `sense` (MCP) | satellite, "where is the logs endpoint handler…" | Guidande svar ✓: "no matches … build it with `gnawtreewriter ai index`, or pass file_path" — men MCP har INGET `ai index`-verktyg (agenten måste bash:a) |
| 2 | `sense` (MCP) | zoom, file=system.rs, två olika queries | **Bara rubrikraden** "Zoom search results for <file>" — inga noder, inga poäng, inget innehåll |
| 3 | `sense` (CLI, `GNAW_JSON=1`) | identisk query + fil | Fullt JSON: 5 noder `{path, preview, score}` (14, 121.2.11, 121.2.9, 106, 79) — den strukturerade datan FINNS |

**Omväg:** CLI med GNAW_JSON=1 (för att överhuvudtaget se träffarna).

**Bedömning:**
- [ ] användarfel [ ] bugg [ ] saknad funktion
- [x] **svår att nå/rätta användning** — MCP-handlern skickar JSON:n som
      strukturerad bilaga (`tool_success(text, Some(json))`), men agentens
      MCP-yta visar bara `text`-delen. Verktyget är under funktionalitet i
      sitt huvudläge: agenten som anropar `sense` via MCP får noll
      information. **FIX:** lägg träfflistan ÄVEN i text-delen
      (t.ex. "5 träffar — 106 system_health_handler (0.81), …") så att
      texten är tillräcklig i sig.
- [x] **sub-funktion saknas för situationen** — satellitläget saknar ett
      sätt att bygga indexet från MCP (meddelandet pekar på ett CLI-kommando
      agenten inte har som verktyg).
- **Relevans-notering:** `logs_handler` (system.rs:45) fanns i filen men
  kom ej med i topp-5; `VersionInfo`-strukturen toppade (0.84). Rangordning/
  preview-fördjupning bör bedömas (hör till sub-funktion-raden ovan om det
  återkommer).

**Vägledning träffad?** delvis — satellitens no-matches-medar exakt nästa
steg (riktig stil); zoom-rubriken guiding? nej (rubrik utan innehåll).

**10-01-fixen:** zoom-läget svarade på sekunder, ingen timeout, ingen panik
— stabiliteten bekräftad åtminstone för zoom på 800+ raders Rust-fil.
Satellit utan index = guidad avvisning, inte crash. Punkten betraktas som
stängd för stabilitet, öppen för MCP-ytan ovan.

**FIX-presentationen utförd 2026-10-04 (9.5):** både zoom och satelit bär
nu träfflistan I text-delen (`"Zoom …: 5 node(s) — 106 (0.81) …"`,
`"Satellite …: 10 match(es) — file:node (0.91) …"`); tom zoom svarar med
guident fel, aldrig en naken rubrik. Kvarstår öppet: satelitens
index-bygge saknar ett MCP-verktyg (pekar på CLI-kommandot) — noterat som
känd lucka, ej i 9.5-omfånget.

---

## 2026-10-04 — MCP `undo`-stub ljög "Undo executed"; semantic_insert felplacering; valideringsgap (9.1-sessionen, GTW:s eget repo)

**Kontext:** 9.1-arbetet (gtw_lint i MCP) i GTW-repet självt. OpenCode-agent
(GLM) körde `semantic_insert` mot `src/core/rules.rs` med ankaret
"format findings as compact prompt annotations…", följt av MCP `undo`.

| # | Anrop | Parametrar | Resultat |
|---|-------|------------|----------|
| 1 | `semantic_insert` (MCP) | rules.rs, anchor "format findings as compact prompt annotations…", intent before | "Successfully inserted near anchor '25' (confidence 0.84)" — men rätt mål var node **70** (`format_findings_for_prompt`). Inserten landade på parent 0/pos 27: **mitt i modul-doc-kommentaren**, som delade `//!`-blocket på mitten |
| 2 | `undo` (MCP) | — | **"Undo executed"** — men `git diff` visade oförändrad +219-raders förvrängning. Rotorsak: dispatchen `src/mcp/mod.rs:743-745` är en hårdkodad stub som returnerar `"Undo executed"` utan att göra **något**. Samma mönster på `"batch"` (:740-742) → "Batch executed" |
| 3 | `cargo check` efteråt | — | E0753: sönder skuren `//!` inner-doc. **tree-sitter-valideringen släppte igenom den** (error-tolerant parser) — GTW:s "validation before write"-löfte har ett hål för doc-kommentarplacering |

**Omväg:** agenten föll tillbaka på `git checkout` (förbjudet per policy —
borde varit CLI `gnawtreewriter undo`, som har riktig implementering via
`UndoRedoManager`, eller `restore-project --preview`). Lärdom: när MCP-undo
ljuger framgång är git nästa steg — exakt det adoptionssyndrom Lucka 3/6
beskriver, nu bevisat i GTW:s eget repo.

**Bedömning:**
- [ ] användarfel [x] **bugg** (MCP `undo`/`batch`-stubbar: rapporterar
      framgång utan effekt — värre än saknad funktion, verktyget ljuger)
- [x] **under förmåga** (`semantic_insert`: fel ankare med 0.84 confidence,
      ingen tröskelkontroll, meddelandet visar naken nod-siffra utan typ/namn/
      innehåll — agenten kan inte upptäcka felet från svaret)
- [x] **bugg (valideringsgap)** — parse-OK ≠ giltig kod: tree-sitter fångar
      inte E0753-klassens fel (felplacerade `//!`). "Validation is mandatory"
      har därmed ett ljudlöst undantag.

**Åtgärder (i prioritetsordning):**
1. [ ] **FIX (trivial, P1):** koppla MCP `undo` till `UndoRedoManager` (som CLI
     `handle_undo`, cli.rs:1577) och MCP `batch` till batch-motoriken. Ta bort
     stubbarna i `src/mcp/mod.rs:740-745`. En stub som ljunger framgång
     strider mot "fails loudly (never silently)".
2. [ ] **FIX (9.1-kopplad):** `semantic_insert`-svaret ska bära ankarets
     identitet (typ + namn + 1 rad innehåll) och confidence-tröskel avvisning
     ("best match 0.4 — för osäkert, ange node_path manuellt").
3. [ ] **FIX (validering):** efter varje edit, flagga `//!`/`/*!` inner-doc
     som inte är i filhuvudet (billig heuristik fångar E0753-klassen), eller
     dokumentera uttryckligen i valideringsmeddelandet att tree-sitter-OK
     inte garanterar rustc-OK.
4. [x] Sessionens undo-genväg dokumenterad: **MCP-agenter ska använda CLI
     `gnawtreewriter undo` / `restore-project` tills punkt 1 är fixad.**

**Vägledning träffad?** nej — "Undo executed" är motsatsen till guidande.
Rätt svar från en äkta undo: "✓ Undone: Insert rules.rs (tx-id)" — och vid
misslyckande "✗ no backup for tx …, use restore-project".

---

## 2026-10-04 — `semantic_edit` mest trasig enligt användaren (relä från
GTW-agentens session)

**Kontext:** användaren (2026-10-04, efter GTW-agentens stora bugg-rykte):
"semantic edit i GTW är en av de som är mest trasiga ... Det verkar vara
största hålen just i GTW" — jämte undo-stubben (posten ovan). Samma dag
parkerade användaren GTW helt: "opålitlig, värt att använda först efter
nästa version" — Motor2:s AGENTS.md §35 har en motsvarande statusnot.

**Bedömning:**
- [ ] användarfel [x] **bugg** (semantisk editering — verktygets kärnlöfte)
- Samma felklass som den dokumenterade `semantic_insert`-felplaceringen i
  posten ovan (valideringsgapet) — misstänkt gemensam rot i AST-matchning/
  applicering. Driftprov + konkret repro behövs från GTW-agenten som kör
  9.1-sessionen; denna post är plats-hållaren så att hålet inte glöms i
  nästa versions inköpslista.
- **Prioritet:** hög — `semantic_edit` är annonserad i Motor2 AGENTS §35.1
  beslutstabell som verktyget för "vet VAD, inte VAR"; trasig här = hela
  "GTW-först"-löftet halteras.

**Tillits-karta 2026-10-04 (samma driftperiod):**
- `insert_node` — **bekräftat fungerande** (användaren/GTW-agenten:
  "verkar funka dock")
- `semantic_edit` — trasig (denna post)
- MCP `undo`/`batch` — stubbar som ljuger framgång (posten ovan)
- Next-version DoD: fixa de tre, behåll insert_node-nivån.

**Vägledning träffad?** ej bedömt ännu (repro saknas) — men verktyget SAKNAR
idag varning om sitt tillstånd: en agent som anropar det får ingen signal
om att resultet kan vara felplacerat/tyst fel. Guidance i svaret (t.ex.
"verify with read_node after semantic edit — known unreliable in 0.9.x")
är minsta åtgärd tills roten är fixad.

---

## 2026-10-04 — 9.1-fortsättning: stub-fix rullad ut + nya dogfooding-fynd (GTW:s eget repo)
**Kontext:** Fortsatt 9.1-arbete (CLI-refaktor + tester). GTW användes maximalt
(edit_node/insert_node/quick-replace/list/show); varje friktion loggas.

**Fynd A — STUBBEN ÄR BORTTAGEN (fix verifierad):** MCP `undo`/`batch` levererar nu
riktiga implementationer (`handle_undo_mcp` → `UndoRedoManager`, `handle_batch_mcp` →
`Batch::from_file`/preview/apply). Bevis: det gamla E2E-testet som assertade
stub-kontraktet (`batch` med tomma args → 200 + "Batch executed") FALLERAR mot nya
servern (400/-32602) — testet uppgraderat till äkta kontrakt: missing `file` →
INVALID_PARAMS, preview av riktig spec → 200 + "nothing written" + fixture byte-identisk.
**Obs:** installerad MCP-binär är fortfarande gammal (`gnawtreewriter_batch` MCP-anrop
svarade "Batch executed" utan args = stubben lever i det installerade bygget tills
`cargo install --path .` körs om).

| # | Anrop | Parametrar | Resultat |
|---|-------|-----------|----------|
| 1 | mcp batch (installerad binär) | inga args | "Batch executed" — stubben, silent no-op (gammal binär, förväntad) |
| 2 | cli insert rules.rs "100.2" 12 <sökväg> | positionell content = filsökväg | **Validation rejected**: sökvägen insattes som literal text → syntaxfel fångat. Rätt API: `--source-file`. Bedömning: svår att nå/rätta användning (friktion) men valideringen gjorde sitt jobb |
| 3 | cli quick-replace rules.rs (3 st) | 2 enkrad + 1 flerrad | enkrad ✓ applicerad; **flerrad: rapporterade "✓ QuickReplace applied" men ändrade INGET** (assertionen kvar oförändrad) — **bugg: tyst no-op på multi-line search** |
| 4 | cli quick-replace cli.rs (dok-kommentar, flerrad m. blankrad) | flerrad | samma: "✓ applied" men oförändrad fil — reproducerar fynd 3 |
| 5 | cli edit cli.rs "19-equivalent" --source-file | funktionsnod | första anropet: endast GnawTip i utdatat, ingen ändring (tyst); andra anropet identiskt: applicerades (-126/+126). **Möjlig first-run-flakighet — ej reproducerad isolerat** |
| 6 | nod-path-instabilitet | insert i "100.2" pos 12 | syskonnummer skiftade (test_except_pass_python 10→?). Dokumenterat beteende men fälla för agenter som cachelagar paths |
| 7 | cli quick-replace cli.rs (opts-block, enkradssökning) | `            rule_filter,` → +2 rader | **"✓ applied" men oförändrad fil** — tyst no-op även på ENKRAD sökning (tidigare antaget flerradsbegränsat). Reproducerat 1/6 anrop i samma session |
| 8 | innehållsåteruppkomst | python-fix tog bort 3 rader (verifierat grep=0); nästa GTW-skrivning senare i sessionen | raderna PÅ TRÄF igen i filen (verifierat grep=1) — **GTW verkar göra read-modify-write från en inre cache och kan återinföra borttaget innehåll**. Observerat 2 gånger (4678→4690, +12 raders skift från mellanliggande insert). Härdning: efter externa (icke-GTW) borttag, verifiera innehåll EFTER nästa GTW-skrivning |

**Bedömning:**
- Fynd 3+4+7: **bugg (hög prio)** — quick-replace MÅSTE antingen applicera eller
  rapportera "no match"/"replaced N occurrences"; tyst ✓ är samma klass som stub-lögnen.
  Fynd 7 visar att felet inte kräver multi-line search.
- Fynd 8: **bugg (hög prio, dataförlust-klass)** — trolig stale-cache read-modify-write
  i GTW:s skrivväg; kan tyst återinföra borttagna rader och kastas bort ändringar.
  Repro: ta bort rader externt (python/text-editor) → gör en GTW-edit i samma fil →
  verifiera att de borttagna raderna inte kommit tillbaka.
- Fynd 2: förbättring — insert med filsökväg som positionellt content borde ge
  "did you mean --source-file?"-ledning i felmeddelandet.
- Fynd 6: guidance — insert/edit-svar borde nämna "paths may have shifted; re-list
  siblings after insert".

**Vägledning träffad?** nej för fynd 2/3/7 — quick-replace borde skriva ut antal
ersättningar ("replaced 2 occurrences") så att 0 = synlig no-op.

**Pre-existing skuld (ej från denna session, noteras för 9.x):** `cargo clippy
--all-targets -- -D warnings` fallerar i ovidrörda testmoduler: parse_cache.rs:94,
index_entities.rs:761, pack.rs:648/674/706, state.rs:123-124, token_count.rs (6 st).
Lib-koden är ren.

---

## 2026-10-04 — 9.2-sessionen: position-clamp, brace-valideringsgap, resurrection ×7 (GTW:s eget repo)
**Kontext:** 9.2 (`fix:`-stöd) implementerat. GTW används för all projekt-redigering
efter överenskommelse: GTW → OpenCode edit-verktyg → aldrig python-skript (agent-processöverträdelse
med python ×8 erkänd och stoppad; OpenCode edit är den sanktionerade omvägen).

| # | Anrop | Parametrar | Resultat |
|---|-------|-----------|----------|
| 9 | cli insert rules.rs "0" 68 --source-file | root-insert vid position 68 | **Tyst clamp till ~6**: funktionerna landade efter dok-kommentarerna (filens topp), inte vid index 68. Ingen varning, inget fel. Dessutom klippte insättningen MITT i `//!`-dokblocket → E0753 i rustc. Position 68 var korrekt barnindex (verifierat via lista). **Bugg: root-insert-position ignoreras/clampas tyst** |
| 10 | cli edit mcp/mod.rs <fn-nod> --source-file | funktionsersättning där mitt innehåll saknade fn:ens avslutande `}` | **Valideringen släppte igenom**: tree-sitter accepterar nästlade fn-items (laglig Rust: `fn a() { fn b() {} }`), så resten av modulen blev tyst NäSTLAT — rustc:na rapporterade därefter förvirrande E0425 (serve/status "not found"). **Valideringsgap: GTW kan inte se brace-balansskillnader som råkar vara syntaktiskt giltiga** |
| 11 | resurrection, sammanräknat | 7 offers | (1) spök-dok-kommentar cli.rs ×3 återkomster, (2) `#[allow(clippy::too_many_arguments)]` på handle_lint, (3) `ad_hoc_pattern/ad_hoc_language`-trådning i handle_lint opts, (4) `fix: None` i testhjälparen rules.rs ×2, (5) scaffold-fallback i validate_fix, (6) **hela 6-testblocket (170 rader) i rules.rs — största förlusten**. Mönstret: fil redigerad utanför GTW (python/OpenCode-edit) → därefter GTW-skrivning i samma fil → regioner från äldre cachad version återkommer. **Dataförlust-klass, nu det dominerande hotet mot agent-sessioner** |

**Bedömning:**
- Fynd 9: **bugg** — root-insert-position måste antingen respekteras eller avvisas
  med "position out of range (N children)".
- Fynd 10: **valideringsgap (medel prio)** — edit_node borde varna när en
  funktionsersättning ändrar total brace-djup ("replacement has different brace
  balance than the node it replaces").
- Fynd 11: **bugg (kritisk)** — stale-cache-write. Åtgärdsförslag: ogiltigförklara
  fil-cache när filens mtime/hash ändrats utanför GTW (check on write), och
  skriv verifiering ("N bytes written, hash X") efter varje operation.

**Processlära (agenten):** python-skript kringgår både GTW-validering OCH
shadow-checkpoints — överträdelserna ×8 stoppades av användaren. Sanktionerad
ordning: **GTW → OpenCode edit-verktyg → (ingen python)**. Och varje GTW-skrivning
följs av innehållsverifiering tills fynd 11 är fixad.

---

## 2026-10-04 — 9.3: resurrection-mysteriet löst + adoptionkontrakt

**Kontext:**

| # | Anrop | Parametrar | Resultat |
|---|-------|-----------|----------|
| — | (beviskedja) | mtime-bevakning 90 s utan testkörning | 0 omskrivningar — "resurrection" var INTE tid-beroende |
| — | `cargo test` (full svit) | ×2 före fix | rules.rs förlorade `fix: None` igen, varje körning |
| — | läs `integration_mcp_tools_call_undo` | tests/mcp_integration.rs:517 | testet anropade `undo` mot **repo-rötens riktiga transaktionslogg** |
| — | läs `serve_with_shutdown` | src/mcp/mod.rs | project_root = CWD → tester ärvade repots logg |

**Bedömning:**
- **Fynd 12 (ROTORSAK till fynd 11 — omklassificering):** bugg (kritisk, test-infrastruktur).
  `integration_mcp_tools_call_undo` rev varje `cargo test`-körning tillbaka den
  senaste GTW-transaktionen i repot. Samtliga 8 "resurrection"-offer (rules.rs
  `fix: None` ×2, cli.rs spöke-kommentar ×3 m.fl.) var **undo-replies, inte
  cache-buggar**. Fynd 11:s stale-cache-hypotes nedgraderas: ingen sådan bugg är
  bevisad; innehållsverifiering efter GTW-skrivning är fortfarande sund vana.
  **Fix:** ny `serve_with_shutdown_root(listener, token, project_root, shutdown)`
  (explicit root); undo-testet binder servern till en temp-projektrot och
  assertar deterministiskt "Nothing to undo". Regression bevisad: 2 fulla
  suite-körningar i rad, rules.rs md5 oförändrad.
- **Fynd 13:** bugg (test). `integration_mcp_tools_call_batch` skrev batch-spec
  med icke-existerande fältet `replace` — korrekt filformat är `BatchFile`/
  `BatchOp` i src/core/batch.rs: `{file, path, content}`. Testet kunde aldrig
  ha passerat mot riktig parser; felet maskerades av fynd 12:s kaos.
  Fixad till rotnods-ersättning `path:"0"` + `content`.
- **9.3-adoption (ROADMAP):** 30/30 verktyg har nu beskrivning ≥ 100 tecken
  (mall VAD/NÄR/RETURNERAR/EXEMPEL) + full inputSchema; schema-lögner rättade
  (`list_nodes` saknade `filter`/`max_depth`; `explain`/`edit_ask`/`investigate`
  saknade `required`; `index_*` fick anyOf file_path|file_paths). Nytt
  kontraktstest `integration_mcp_tools_adoption_contract` låser detta mekaniskt
  (unika namn, desc ≥ 100, required ⊆ properties, ärligt noll-args,
  prioriteringsverktyg med riktiga scheman). GTW_INSTRUCTIONS.md regenererad
  (17 → 30 verktyg + diagnostisk kedja).

**Vägledning träffad?** ja — undo-svaret pekar redan på `history`/`restore-project`;
grunden till katastrofen var att ett test OperERADE på riktiga filer, vilket
nya `serve_with_shutdown_root` gör strukturellt omöjligt för tester.

---

## 2026-10-04 — 9.4 live-verifiering: sense via MCP (ny installerad binär)

**Kontext:**

| # | Anrop | Parametrar | Resultat |
|---|-------|-----------|----------|
| 1 | `tools/list` (stdio MCP) | — | 30 verktyg, 23 `WHEN:`-beskrivningar, `lint` finns — 9.3 lever live i binären |
| 2 | `sense` (satellit) | `query="how is undo implemented in the transaction log"` | "no matches — index may be missing": projektindex ofullständigt (63/102 filer; `ai index` ≈80s/fil = ~50 min kvar) |
| 3 | `sense` (zoom) | samma query + `file_path=src/core/undo_redo.rs` | ✓ 5 noder, impact med cross-fil-referenser. Topp: nod `8` = `impl UndoRedoManager` (0.79, preview synlig), sedan `8.2.2` = `UndoRedoManager::new` |

**Bedömning:** v0.9.8-sensorn svarar korrekt genom MCP på rätt fil och rätt
nod för en känd fråga — stabilitetsfixen bekräftad live. Satellitläge
(filupptäckt utan given `file_path`) kunde ej verifieras: kräver komplett
projektindex; `gnawtreewriter ai index` är inkrementellt (hoppar över oförändrade
filer) och kan köras i bakgrunden senare utan att blockera.

**Obs:** `cargo install --path .` 2026-10-04 ersatte ~/.cargo/bin-gnawtreewriter —
de 4 redan igångna MCP-daemonerna (pts/8, /10, /9, /14, startade okt 01-03) kör
fortfarande den GAMLA binären tills deras sessioner startas om.

---

## 2026-10-04 — GPU-indekering (fynd 14: quick-replace falskt "applied")

**Kontext:**

| # | Anrop | Parametrar | Resultat |
|---|-------|-----------|----------|
| 14 | `quick-replace src/llm/ai_manager.rs` (testmodul-omskrivning) | oldString = före-`cargo fmt`-text, filen var fmt-omskriven | ✓ "applied" (txn …85977) men **bytes oförändrade** — gamla tester kvar, nya saknades; upptäcktes först av fallande tester |

**Omväg:** verifiering med grep efter varje GTW-skrivning (redan policy efter
resurrection-fynden) + ersättning med exakt fmt-matchande text → nya tester på
plats (3 träffar, 0 gamla), 177+24 tester gröna.

**Bedömning:** [x] bugg — `quick-replace` får inte rapportera "applied" när
söktexten inte matchar; bytes ska ändras, annars fel med tydlig text
("söktexten hittades inte — läs om filen efter `cargo fmt` och försök igen").
Guidande-principen: felet ska peka på nästa steg, inte bluffa framgång.
Samtliga tidigare ersättningar den här dagen verifierades med grep — denna
ena slank igenom för att verifieringen hoppades över en gång.

**Sammanhang:** GPU-arbetet (opt-in-gate `indexing.device` i
`gnawtreewriter.yaml`, `--gpu`-flagga, `scripts/build-gpu.sh`,
AGENTS.md-avsnitt) är komplett och live-verifierat i tre lägen (fil=auto →
GPU vid 28 % VRAM fri; ingen fil → konservativ CPU-default; `--gpu` → GPU).

---

## 2026-10-05 — Motor2-verifiering: parse-bugg på giltig Rust (database.rs:6097), satellit-sense kräver fortfarande CLI-index

**Kontext:** Verifiering av 9.5/9.6 mot nystartad 0.13.0-daemon (Motor2-repot,
PID 3259463, exe = live `libexec/gnawtreewriter` — aktuell binär, ej stale).

| # | Anrop | Parametrar | Resultat |
|---|-------|-----------|----------|
| 1 | `get_skeleton` | `crates/motor2-session/src/database.rs` (11 476 rader), max_depth 1–2 | ✗ "Syntax error at line 6097, col 72" — men raden ÄR giltig Rust (`serde_json::from_str::<serde_json::Value>(&raw).unwrap_or_else(\|_\| ...)`, turbofish + closure); `cargo build --release` RC=0 |
| 2 | `analyze` | samma fil | ✗ samma parse-fel — HELA diagnoskedjan (analyze→sense→read_node→edit_node) faller för filen |
| 3 | `get_skeleton` | `crates/motor2-chat/src/bridge/tests.rs` (6 371 rader) | ✓ 500 noder — INTE storleksrelaterat, konstrukt-relaterat |
| 4 | `sense` | utan file_path, projektfråga | ✗ "no matches ... build it with `ai index`" — även EFTER MCP `index_entities` (56 entiteter indexerade, 0 errors) — MCP-indexering fyller INTE satellit-indexet |
| 5 | `sense` | MED file_path (bridge/mod.rs) | ✓ 5/5 relevanta träffar (distill_completed_turns etc, 0.77–0.80) |

**Omväg:** läs fil-regionen med Read (6090–6104), verifiera giltighet via
cargo-bygge; zoom-sense med file_path i stället för satellit.

**Bedömning:** [x] bugg — två separata:
1. **Parse-robusthet:** tree-sitter-grammatikan stöter på en giltig
   Rust-konstruktion (turbofish-generic i metodkedja med
   `unwrap_or_else(\|_\| json!({...})`-fallback är primär misstanke, col 72)
   och avvisar HELA filen. Giltig kod får aldrig ge "Syntax error" —
   minst: felsäkert PARTIAL-parse (posta felet, returnera det som gick).
2. **Satellit-sense-capacitet:** MCP-`index_entities`/`index_relations`
   och satellit-`sense` använder skilda index — agenten som indexerar via
   MCP förväntar sig att sense sedan svarar; "no matches" är då ett
   integrationsfel (två index, en förväntan). Guidande-meddelandet är bra
   men capabiliteten saknas.

**Vägledning träffad?** Delvis ja — 9.6-leveransen syns (sense pekar på
`ai index`/file_path; skeleton ger explicit fel i stället för tyst tomt).
9.5-KLAR-markeringen (ROADMAP ✅ 2026-10-04) täcker symptomet "tomt svar"
men INTE underliggande parse-robusthet — rekommenderar att öppna posten
som "parse partial-grace" snarare än att låta KLAR stå som fullständig.

**Svar (GTW, 2026-10-05): ROTSAKEN HITTAD OCH FIXAD — utan delvis-parse.**
1. **Parse-buggen = `&raw` + tree-sitter-rust 0.24.0.** Minimering visade
   att varje konstrukt (`turbofish`, `json!`, multi-line-kedja, `&self`)
   var oskyldig — triggern var identen **`raw` efter `&`**: grammatsparer
   för Rust 1.82:s `&raw const/mut` (raw-referenser) kräver `const`/`mut`
   efter `&raw` och stöter på `g(&raw)` (vanligt variabelnamn!). Bevis:
   `g(&s)` OK, `g(&raw)` fail, `raw + 1` OK, `&raw const x` OK.
   **Fix: `cargo update -p tree-sitter-rust` 0.24.0 → 0.24.2** (uppströms-
   rättad ambiguëtitet) — hela `database.rs` (11 476 rader) parserar nu OK,
   inkl. era fem anrop. Regressionsspik: `parser::rust::tests::
   parses_plain_raw_borrow` (+2) gröna. Delvis-parse (punkt 1:n i
   bedömningen) kvarstår som hårdhetsbacklog AANNÅLST — men ingen
   workaround behövs längre för att filer ska fungera.
2. **Satellit-index-klyftan: BEKRÄFTAD och delvis öppen.** `index_entities`/
   `index_relations` (MCP) skriver KUNSKAPSGRAFEN; satellit-`sense` läser
   VEKTORINDEXET (`.gnawtreewriter_ai/index`) — samma namnfamilj, två
   olika databaser. "no matches efter index_entities" är ett integrations-
   misförstånd, inte en trasig sädeskälla. Åtgärd: nytt MCP-verktyg som
    kör samma indexeringspipeline som `ai index` (GPU-gaten gäller automatiskt)
    — låg som känd lucka i9.5-loggen och återfinns här; tar vi som nästa
    steg om ni vill.

**Uppföljning (samma dag):** punkt 2 är avstämd — `index_project`
(start/status i bakgrund, GPU-gaten gäller) landade i `f58989d` (v0.15.0);
`index_entities`-beskrivningarna pekar nu uttryckligen på den skillnaden
och satellitens no-matches-mädelandе nämner verktyget först.

---

## 2026-10-05 — FIXAD: v0.16.0 bröt --no-default-features-byggen (Motor2 buggrapport)

**Kontext:** Motor2-agenten rapporterade (vid ace036e): `cargo check
--no-default-features` → 8 fel i `src/llm/gnaw_sense.rs` — nya
`sense_with` (query-expansionen från 0.16.0) saknade feature-gate medan
kroppen använder modernbert-gated kod (importerna `AiModel`/`DeviceType`,
fältet `relational_indexer`, `index_file_cached`,
`extract_name_from_preview`). Vår default- ≠ mamba-matris har alltid
`modernbert` på → greppades aldrig.

**Åtgärd (direkt):**
1. `#[cfg(feature = "modernbert")]` på `sense_with`.
2. `#[cfg(not(feature = "modernbert"))]`-stub som `anyhow::bail!` med
   tydligt fel ("sense_with requires the 'modernbert' feature —
   rebuild with --features modernbert") — externa path-dependents
   (motor2-gtw) får kompilering + ärligt runtime-fel i stället för
   E0433/E0599/E0609 i vår kod.
3. **Granskning av resten av 0.16** (feedback-loop/search_quality/
   fuse_by_max/expand_query_terms): rena — hjälpna är serde/chrono/fs
   utan modernbert-typer, `expand_query_terms` bor i det mamba-gatade
   pipeline-modulen, mcp-sättningen ligger i `handle_sense`'s
   modernbert-block. Bekräftat av att `--no-default-features` går
   rent.
4. **Regressionströskel:** `validate.yml` kör nu
   `cargo check --no-default-features --all-targets` — default OCH
   mamba båda har modernbert, bara denna konfig hittar gathål.
   `examples/debug_loading.rs` fick `required-features = ["modernbert"]`
   (pre-existerande hål: candle-import utan gate) så all-targets går.

**Verifiering:** `cargo check --no-default-features --all-targets` ✓,
`cargo check --features mamba` ✓, clippy `--lib -D warnings` ✓,
232 tester gröna.

**Bedömning:** [x] bugg — min (0.16.0:s) — gate-slip vid ny publik
metod. Vägledning träffad? Ja — rapporten pekade exakt rätt fil/fix/mönster.

## 2026-10-05 — index_relations: "calls"-relationer med TOMT `from` (Motor2-dashboard, lib-konsument)

**Kontext:** Motor2:s dashboard använder `index_entities`/`index_relations`
(lib-API, motor2-server → gnawtreewriter path-dep 0.15/0.16) för en
per-fil entitetsgraf. På `crates/motor2-shadow/src/manager.rs` (91
relationer, 15 calls) har FLERA "calls"-rader tomt `from` — t.ex.
`"" -> table_cols` upprepade. Mönstret: call sites inuti impl-block där
anropssajten inte går att namnge → from blir tom sträng i stället för
någon form av id. UI fick rendera "→ table_cols"; Motor2 lappade med
"(intern)"-platshållare (pkg/gede/project-map.js).

**Omväg:** UI-platshållare — funkar men döljer information (vilken metod
ringer? raden finns i relationens `line` men from-id saknas).

**Bedömning:** [x] under förmåga (levererar inte vad namnet lovar) —
relationens `from` är del av kontraktet `gtw:{file}:{type}:{name}`; tom
sträng bryter det. Låg allvarlighetsgrad, hög enkelhet: antingen (a)
resolva till omslutande metod-namn när det finns, (b) syntetiskt id
`gtw:{file}:callsite:{line}`, eller (c) dokumentera tom-sträng-konventionen
så konsumenter vet vad de ska förvänta sig.

**Vägledning träffad?** [x] ja — lib-API:t returnerade data (inte fel),
men kontraktet var odokumenterat för detta fall.

**Åtgärd (GTW, 2026-10-05):** variant (a)+(b) som föreslogs —
`find_calls_in_scope` trådar nu det omslutande funktions-scoopen
(`is_function_like`-hjälpare, samma nodtyper som `collect_defined_functions`)
och sätter `from = gtw:{file}:function:{namn}`; scope-lösa anrop
(statiska initialisrar) får det syntetiska `gtw:{file}:callsite:{line}`.
Tom sträng är omöjlig — invarianten är testad både på fixtur
(`calls_from_is_never_empty_and_resolves_scope`: impl-metod →
`:function:run`, fritoppsfunn → `:function:main`, static → `:callsite:`)
och på riktig fil (`test_debug_mcp_relations` kör nu "inget from är
tomt" över hela mcp/mod.rs). Rustdoc på `Relation.from` dokumenterar
kontraktet. **Verifierat mot er exakta rapportfil** (manager.rs, 91
relationer / 15 calls som ni rapporterade): **0 tomma from**, calls
resolver till `function:check_admission` m.fl. — er "(intern)"-lapp i
pkg/gede/project-map.js kan tas bort när ni uppdaterar path-dep.

---

## 2026-10-06 — Till Motor2: vad som landat för er i v0.17.0 (path-dep-bump räcker)

**Till:** Motor2-kodningssessionen. **Allt nedan finns i `80c9d07` /
tagg `v0.17.0`** — ni behöver bara pulla repot (path-dep).

1. **Tomt `from` FIXAT** (`a655cd2`): era calls-relationer resolver nu
   till `gtw:{file}:function:{omslutande_metod}` eller syntetiskt
   `gtw:{file}:callsite:{rad}` — verifierat 0 tomma på er exakta
   `manager.rs` (91 relationer / 15 calls oförändrade). **Kasta
   "(intern)"-platshållaren i `pkg/gede/project-map.js`.**
2. **Idempotenta retries**: `edit_node` med innehåll som redan finns på
   målet svarar `already_applied: true` med noll skrivna byte — er
   historiska timeout→retry→avslag-spiral är stängd.
3. **Skrivkvitton**: `transaction_id` i edit/insert/semantic_insert-svar,
   `transaction_ids [src, tgt]` i move, en per fil i batch — korrelera
   med `history`/`undo`.
4. **`guide {situation}`** (21 rader) — situations-coaching utan vår
   SKILL.md; **`validate {file_path}`** (syntaxgate), **`diff {old,new}`**
   (oberoende verifiering), **`undo {preview: true}`** (se-före-återställ).
5. **BREAKING-förvarning (policy från nu)**: `SenseResponse::Satelite`
   fick fältet `quality` i 0.16.0 — matchar ni den exakt behöver ni
   `quality: _` (MCP-trådet påverkas inte). Från nu: varje lib-brytning
   får `### BREAKING (lib)` i CHANGELOG FÖRE release.
6. **Stela daemoner:** de gamla MCP-daemonerna (startade okt 1–5) kör
   föråldrade binärer och avslutas nu — era sessioner får 0.17.0 vid
   nästa MCP-anrop (eller session restart).

---

## 2026-10-06 — `gnawtreewriter undo` hängde (timeout 60 s, ingen effekt)
**Kontext:** Efter en felplacerad quick-replace i `src/core/diagnostics.rs`
(ankaret `}\n\nfn is_important_node_type` träffade impl-avslutet istället
för metodplatsen — agentfel, GTW validerade korrekt) kördes
`gnawtreewriter undo` för att återställa. Kommandot svarade inte på 60 s
(shell-timeout) och filen var inte återställd efteråt; `history` visade den
felaktiga transactionen fortfarande som sista post. Reparation skedde i
stället med exakt text-uxe (python) + korrekt GTW-insert på ett unikt
ankare (`// ---- Helpers ----`-markören).

| # | Anrop | Parametrar | Resultat |
|---|-------|------------|----------|
| 1 | `undo --preview` | — | `error: unexpected argument '--preview'` (preview finns bara på andra kommandon — oklart API) |
| 2 | `gnawtreewriter undo` | — | hängde > 60 s, ingen filändring, ingen utdata |

**Omväg:** python-exakt-text + ny GTW-insert på unikt ankare.

**Bedömning:** [x] användarfel (ankaret) — MEN punkt 2 är en verklig
fyndkandidat: undo bör antingen slutföras på <sekunder eller rapportera
varför den avstår. [ ] bugg (obekräftad — inte reproducerad isolerat)
[x] svår att nå/rätta användning (`undo --preview` saknas trots att
mönstret finns på `restore-project --preview`/`undo {preview}` i MCP)

**Vägledning träffad?** nej — en undo som hänger tyst utan utdata bryter
mot "GTW:s fel ska vara högljudda". Förslag: undo ska skriva vad den gör
(transaktions-id, filer) och avsluta Högljuddt även när den avstår.

---

## 2026-10-06 — `quick-insert` rapporterade "applied: 0 insertion(s)" utan att skriva
**Kontext:** CHANGELOG.md (markdown), ankare `## [Unreleased]` som första rad,
`--after "<ankare>" --unique`, innehåll ~23 rader. Svaret var
`✓ QuickInsert applied: 0 insertion(s)` — git diff visade **0 bytes ändrade**.
"E applied" med antal 0 är en motstridig rapport: antingen hittades ankaret
inte (då ska det vara ett FEL med radnumret som saknas) eller hittades det
men skrev inte (bugg).
| # | Anrop | Parametrar | Resultat |
|---|-------|-----------|----------|
| 1 | quick-insert | CHANGELOG.md, --after "## [Unreleased]", --unique | "applied: 0 insertion(s)", ingen filändring |
| 2 | quick-insert | samma men --before | erkänt ogiltigt flag: "unexpected argument" (finns bara --after) |
**Omväg:** OpenCode edit-verktyg (markdown är tillåtet textområde enligt
AGENTS-policy) — skrev korrekt på första försöket.
**Bedömning:** [ ] användarfel [x] bugg (tyst noll-skrivning rapporterad
som framgång — bryter mot "GTW:s fel ska vara högljudda", jmf fynd #14)
 [ ] saknad funktion [ ] sub-funktion saknas
 [x] svår att nå/rätta användning (--before saknas trots att före/efter-
insert är symmetriska behov; MCP-motsvarigheten insert_node har position)
 [ ] inte hittad vid behov [ ] under förmåga

**Vägledning träffad?** nej — 0 insertion(s) ska ALDRIG vara ✓-grönt;
ska avsluta med fel + "ankaret hittades inte på rad N" eller utföra skrivet.

---

<!-- Ny post: kopiera mallen nedan
## ÅÅÅÅ-MM-DD — kort rubrik
**Kontext:**
| # | Anrop | Parametrar | Resultat |
**Omväg:**
**Bedömning:** [ ] användarfel [ ] bugg
 [ ] saknad funktion (finns inte alls)
 [ ] sub-funktion saknas för situationen (verktyget finns, men inte
     det läge/stöd som situationen krävde)
 [ ] svår att nå/rätta användning (finns men friktion/oklar API —
     agenten använde fel eller föll tillbaka på omväg)
 [ ] inte hittad vid behov (upptäcklighet — agenten visste inte att
     verktyget fanns när situationen dök upp)
 [ ] under förmåga (levererar inte vad namnet lovar)
**Vägledning träffad?** [ ] ja [ ] nej — om nej: vad skulle ett
meddelande/skill ha sagt i just den situationen? (GTW ska vara
guidande: varje fel och varje situation ska peka på nästa steg)
-->

## 2026-10-09 — MCP edit_node: panik på char boundary (Motor2 main.rs, nod "8")
**Kontext:**
| # | Anrop | Parametrar | Resultat |
|---|-------|------------|----------|
| 1 | `edit_node` | file_path=gnawdriver/src/main.rs, node_path="8", content=<fn login-hantering, ~30 rader> | SERVER-PANIK: "byte index N is not a char boundary; it is inside 'ö' (bytes …)" — anropet dog, ingen E_STRICT_PARSE, ingen strukturerad fel-SVAR med nodinfo |
**Omväg:** OpenCode Edit-tool (sträng-ersättning med unik kontext) — fungerade.
**Bedömning:** [ ] användarfel [x] bugg
 [ ] saknad funktion [ ] sub-funktion saknas [ ] svår att nå
 [ ] inte hittad vid behov [ ] under förmåga
**Analys:** felmeddelandet är själva §28.2-klassen Motor2 redan dragit
lärdom av (5:e återkomsten i DERAS kod) — nu i GTW:s egen editväg:
någonstans slice:as edit-node-innehåll/nodnamn på byte-index i stället
för `chars()`. Två problem: (a) paniken själv, (b) den SIKTAS som rå
panik i stället för ett rent E_*-fel med position + förslag.
**Begäran (triage):**
1. Hitta byte-slicen i edit_node-vägen (troligen preview/diff eller
   nodnamnstrunkering) → `chars().take(N)` (§28.2-facit).
2. Panik i edit-vägen ska fångas → strukturerat fel (E_STRICT_PARSE-
   klass) — en agent ska kunna läsa sig till nästa steg.
**Bedömning (fylls vid triage):**
- Använt fel? — NEJ (node_path från analyze, giltigt innehåll med
  svenska tecken — 'ö' i kommentartext).
- Bugg? — JA (char-boundary-panik + opåddnad panik i edit-vägen).
- Prioritet: medel-hög (svenska kommentarer är NORMALFALLET i dessa
  repos; varje edit_node med å/ä/ö i fel position kan träffa den).

## Intake-rapport från Motor2 (2026-10-06): rusqlite 0.32 i c3a79ac bröt Motor2-workspace-resolutionen

**Klass:** sysko-intake (Motor2 AGENTS.md §36) — inte en GTW-bugg i sig, men
intake-friction som per §36-kontraktet ska postas här.

**Vad hände:** `c3a79ac feat(llm): switch retrieval embeddings to BGE` bytte
`rusqlite = "0.31"` → `"0.32"` i GTW:s Cargo.toml. Motor2-cli pinnar rusqlite
0.31 (libsqlite3-sys 0.28) → `links = "sqlite3"`-konflikt: HELA Motor2:s
workspace (cargo check/test/clippy — alla targets) vägrade resolve:a med
"failed to select a version for libsqlite3-sys". Upptäcktes först i Motor2:s
testkörning, mitt i ett pågående pass.

**Motor2-sidans lösning (tillsammans = själva intaken):** workspace-pin
0.31 → 0.32 i Motor2-roten + motor2-post + motor2-collector (rusqlite 0.32.1
/ libsqlite3-sys 0.30.1, bundled). 1710 tester gröna efter bump — ingen
API-påverkan i Motor2:s rusqlite-användning.

**Frågan till GTW:** rusqlite-major/minor-höjningar i sysko-repot påverkar
alla path-dep-konsumenter direkt (ingen release-grind emellan). Skrivs
dep-höjningar i changeloggen/RELEASE_NOTES med "BREAKING för path-deps"-
markering? (Samma mönster som 0.16.0:s ogated `sense_with`.) Önskat: en rad
i varje release-not som rör Cargo.toml-deps.

---

## 2026-10-07 — index_entities/index_relations: ID-formaterna stämmer ej med doc:en, tre sanningar i samma svar (Motor2, kartan v1)

**Kontext:** Motor2 byggde projekt-kartan v1 (beständig sambandsgraf).
`index_entities` + `index_relations` per fil är zon-motorn. Vid första
live-zonen (crates/motor2-server/src/api/map.rs) sparades kanterna med
TRE olika id-format i SAMMA svar:

| Fält | Doc (rustdoc) | Faktiskt värde |
|---|---|---|
| `Entity.id` | "Deterministic ID: `gtw:{file}:{type}:{name}`" | **bart namnet** (`"seed_structure"`) |
| `Relation.from` | "`gtw:{file}:function:{name}` …" | full id MEN med **absolut path** (`gtw:/mnt/.../map.rs:function:seed_structure`) |
| `Relation.to` | "Target entity ID (`gtw:{file}:{type}:{name}`)" | **bart namnet** (`"compute_file_zone"`) |

Konsekvens: konsumenter som litar på doc:en (vi gjorde) bygger
korsfilslänkar som aldrig träffar — src-kanterna pekar på
`gtw:{abs}:…` medan entitetsnoderna skapas från `Entity.id` (bart
namn). Dessutom gör abs-path:en att id:na blir maskin-specifika
(klona repo → annat id för samma symbol).

**Motor2-sidig omväg (6e14c56):** kanonisk namnrymd
`gtw:{relpath}:{type}:{name}` — `from` rewrite via abs-prefixet, `to`
löses mot filens egna entiteter → DB-uppslag per label (korsfil) →
placeholder namespaced under filen. Fungerar, men v1 förlorar
automatisk korsfilslänkning när målet ej redan indexerat.

**Begäran (triage):**
1. Bestäm ETT offentligt id-format (förslag: `gtw:{relpath}:{type}:{name}`
   — relativ, stabil, deterministisk) och låt `Entity.id`,
   `Relation.from` OCH `Relation.to` följa det.
2. Uppdatera rustdoc:en (id:na i `index_entities.rs`/`index_relations.rs`
   dokumenterar ett format koden inte producerar — §17-mönstret: dokens
   sanning separerad från koden).

**Bedömning (fylls vid triage):**
- Använt fel? — NEJ (parametrarna är path + include_private; inga
  format-flaggor finns).
- Bugg? — JA (id-format-drift + inaktuell doc), påverkar alla
  knowledge-graph-konsumenter.
- Prioritet: medel (Motor2 kör vidare på egen normalisering; buggen
  slår mot cross-project-konsistens).

---

## Issue 14 — list_nodes med filter svarar tomt / search_nodes utan paths (2026-10-09, Motor2-session)

**Kategori:** bugg (sub-funktion sviker för situationen)

**Situation:** strukturell diagnos inför rewrite av `gnawdriver/src/mcp.rs`
(§35.3-ritualen: GTW-försök först). `analyze` OK (1683 tokens), men:
- `list_nodes` med `filter: "function"` → **0 noder**
- `list_nodes` utan filter → 184 rader **utan dot-path**
- `search_nodes("serve_stdio")` → "Found 3 matches" **utan nod-sökvägar**

Utan dot-paths kan `edit_node` inte målinriktas — omvägen blev Write av
hela filen (tillåtet per §35.2, men förbigår GTW:s syntaxvalidering och
shadow-checkpoints).

**Reproduktion:** fil `gnawdriver/src/mcp.rs` (~280 rader Rust, en pub fn
+ en async fn + helper). Samma fil: `analyze` hittar noderna.

**Begäran (triage):**
1. `list_nodes` ska returnera dot-path i varje rad (docstringen lovar
   "one line per node: dot-path [type] name" — ett svar utan path bryter
   verktygets hela syfte: "the index for finding the path an edit targets").
2. `search_nodes` ska inkludera nod-sökvägarna i svaret (idag bara antal).
3. Om `filter` kräver exakta AST-typer ("function_item" inte "function"):
   returnera en ledtråd i 0-svaret i stället för tyst tomma (§16: saknas
   ska SKRIKA, inte se ut som "filen har inga funktioner").

**Bedömning (fylls vid triage):**
- Använt fel? — MÖJLIGT för filter-värdet ("function" vs "function_item"
  testades bara implicit), men 0-svar utan ledtråd är i så fall också en
  UX-bugg.
- Bugg? — Sannolikt JA (paths saknas i svaren oavsett filter).
- Prioritet: medel (omvägen fungerar men tappar validering + checkpoints).

**Triage 2026-10-13 (Motor2-session, fixad i källkod):**
- Reproducerat live på den frusna MCP-binären: `list_nodes(filter="function")`
  på `src/mcp/mod.rs` (3 556 rader) → "Found 0 nodes" utan ledtråd; utan
  filter → "Found 9 nodes" med paths ENDAST i payload-JSON (text-only-
  rendering tappar dem). Symptomet bekräftat ordagrant.
- Rot: text-delen (`content[0].text`) byggdes med enbart en räknar-header;
  payloaden hade paths men nådde inte läsaren.
- Fix: text-delen bär nu en rad per nod, `dot-path [type] name`
  (tak 100 rader i list_nodes, 50 i search_nodes, därefter "+N more" med
  hänvisning till payloaden); 0-filter-svar får ledtråd om att filtret
  matchar EXAKTA AST-typer ("function_item", inte "function"). Payloaderna
  oförändrade (strukturerade konsumenter opåverkade).
- Regressionstester: `mcp::mcp_server::read_path_tests` (4) — asserterar
  att texten faktiskt bär `[function_item]`-rader och filter-ledtråden.
- Status: fixad i källkod; når en session först efter omstart av hosten
  (MCP-binären är binärfäst per session).

## Issue 15 — analyze/search/list_nodes magra på STOR fil (2 515+ rader) — omväg tvingad (2026-10-10, Motor2 streaming.rs)

**Kategori:** under förmåga / inte hittad vid behov.

**Vad som hände:** strukturell redigering av
`crates/motor2-server/src/streaming.rs` (2 515 rader, ~29k tokens per
`analyze`). Tre GTW-försök enligt §35.3-ritualen:

- `analyze` → svarade ENDAST "Analyzed ... (28648 tokens)" — ingen nod-lista,
  inga dot-paths.
- `search_nodes("fn remove_task")` → "Found 4 matches" — inga sökvägar.
- `list_nodes(filter="fn", max_depth=2)` → "Found 13 nodes" — inga sökvägar.

Utan dot-paths var `insert_node`/`edit_node` omöjliga att målinrikta;
omväg blev Edit + Read-regioner (§35.2 — tillåtet, men utan GTW:s
syntaxvalidering; kostade en dubblerad `}` som fångades av byggfacit).

**Mönster:** identiskt med Issue 14 (gnawdriver mcp.rs, 280 rader) men på
en fil 9× större — antyder att svaren kollapsar när nodvolymen växer, inte
bara på vissa filer.

**Begäran (triage):**
1. `analyze` på stora filer: returnera top-nivå-noder (dot-path + typ +
   namn) även när totalen är stor — truncera med antal + ledtråd (§16:
   saknas ska SKRIKA), aldrig en tom bekräftelserad.
2. `search_nodes`/`list_nodes`: dot-path i VARJE träffrad (docstringen
   lovar det; svar utan paths bryter verktygets syfte).
3. Med `max_depth` respekterat och fortfarande 0 rader → förklara VARFÖR
   (filter-syntax? parse-fel? för stor fil?).

**Bedömning:**
- Använt fel? — Nej; samtliga tre anrop följde docstring-exemplen.
- Bugg? — Sannolikt JA (stor-fils-vägen returnerar diagnostext i stället
  för data).
- Prioritet: HÖG — stora filer är exakt där AST-verktyget slår text-editor
  (§35.1-beslutstabellen), och just där är det tyst.

**Triage 2026-10-13 (Motor2-session, fixad i källkod):**
- Rot: identisk med Issue 14 — `handle_analyze` satte bara headern
  "Analyzed <fil> (<tokens> tokens)" i texten; hela trädet låg i payloaden
  (som text-renderande klienter tappar). Storleken gjorde bara symptomet
  värre (klient-trunkering klippte payloaden).
- Fix: `handle_analyze` bygger nu en kompakt outline i TEXT-delen —
  `dot-path [type] name` för de första noderna till djup 3 (tak 60 rader),
  med totalräkning i headern och hänvisning till `list_nodes` för flat
  index + payloaden för hela trädet. Trädet serialiseras EN gång (tidigare
  anropades `analyze()` två gånger på stora filer).
- Bieffekt-fix: varning i `src/core/parse_cache.rs` (unused `tree2`) åtgärdad
  genom att assertera cache-träffens nodtyp (§2: varningar är fel).
- Status: fixad i källkod; kräver omstart av hosten för att nå sessioner.

## Issue 16 — sense (satellit) svarar "Failed to resolve bge-base config" i stället för vägledning (2026-10-13, Motor2-session)

**Kategori:** sub-funktion saknas för situationen / oärligt felmeddelande (§16-klassen).

**Situation:** satellit-sense (utan `file_path`) i Motor2-sessionen —
behov: "var sitter X?" utan känd fil (§35.1-diagnoskedjan, steg 2;
GTW-försök först enligt §35.3-ritualen).

| Anrop | Parametrar | Resultat |
|---|---|---|
| `sense` | query (utan file_path — projekt-scope) | **error: "Failed to resolve bge-base config"** |

**Förväntat (enligt verktygets egen beskrivning):** ett tomt/riktat svar
som pekar på åtgärden — "Saknas satellit-indexet? `index_project
{"action":"start"}` … meddelandet från tomt sense pekar på samma" — eller
explicit "modellen saknas, kör `ai setup`".

**Faktiskt:** kryptisk config-fel som inte säger VAD som saknas (bge-base-
modellen ej installerad i miljön och/eller projekt-indexet aldrig byggt —
felmeddelandet skiljer inte på de två) eller VAD som ska göras.

**Omväg som användes:** grep + read (§35.2 tillåtet; men satellit-kanalen
är tyst död i miljön utan att namnge orsak eller bot).

**Begäran (triage):**
1. I frånvaro av bge-base-modellen: sense ska svara med ett meddelande
   som namnger ROTEN ("modellen saknas: `.gnawtreewriter_ai/models/
   bge-base/` — kör `ai setup`") i stället för "Failed to resolve …
   config" (§16: saknas ska SKRIKA med nästa steg).
2. I frånvaro av byggt projekt-index: samma princip — peka på
   `index_project`, skilj fallet från modell-saknas.
3. Överväg auto-init: sense-kall utan modell kan trigga nedladdning
   (eller åtminstone erbjuda den i svaret).

**Bedömning (fylls vid triage):**
- Använt fel? — Möjligen delvis (modellen/indexet aldrig satt upp i
  denna miljö), men felmeddelandet förråder varken orsak eller bot.
- Bugg? — JA (felmeddelandet bryter mot verktygets egen docstring som
  lovar vägledande svar när satellit-indexet saknas).
- Prioritet: medel (omvägen fungerar, men hela semantik-kanalen är
  oanvändbar i nya miljöer utan diagnos).

**Triage 2026-10-13 (Motor2-session, redan fixad i källkod — binär-fråga):**
- Rotorsak: strängen "Failed to resolve bge-base config" finns INTE i
  nuvarande källkod. Felet kom från den frusna äldre daemon-binären.
- Nuvarande källkod har redan vägledande vägar (verifierat i koden):
  1. `sense_broker()`-init misslyckas → `E_MODEL_UNAVAILABLE` + "run
     `gnawtreewriter ai setup` to fetch models (ai status shows what is
     installed), then retry" (mcp/mod.rs).
  2. `load_model` (llm/ai_manager.rs) namnger den saknade filen exakt
     ("Missing config/tokenizer/weights: <path>").
  3. Tom satellit-matchning → pekar på `index_project` / `ai index`.
  4. `sense_with`-fel → "sense failed: {e} — `gnawtreewriter ai status`
     checks the model; on repeat fall back to search_nodes/grep and log it".
- Åtgärd: ingen kodändring erfordras. Verifiera efter omstart av hosten
  att det praktiska meddelandet är vägledande; om det fortfarande är
  kryptiskt → återöppna (sitter då djupare, ev. i candle/hf-hub-resolver).

## Issue 17 — insert_node: position utanför barnantalet bäddade in kod i ett syskon (ROTORSak för Motor2-batt 2026-10-13)

**Kategori:** bugg (korsprojekt-kontrakt + tyst felplacering; svaret påstod "filroten").

**Situation/bevis (Motor2 batt pass 1, session d3e87f73):**
- Motor2:s GTW-brygga (`motor2_gtw_insert_node`, `crates/motor2-gtw/src/tools.rs`)
  kodade "append sist i filen" som `position = children.len() + 1` (25 för 24
  toppnivå-noder) och rapporterade `insert_target: "filroten (toppnivå),
  position 25 (sist av 24 noder)"`.
- GTW:s `insert_node_at_path` godtyckliga index-gren (`idx - 3`) mappade 25 →
  `children[22].end_line` och klistrade in råtexten där: den nya funktionen
  hamnade som FÖRSTA barn inuti `fn fran_romartal` (rad 43) — inte i filroten.
- Beviskedja: pre-insert-backup (`.gnawtreewriter_backups/
  lib.rs_backup_20261013_112147_482.json`, 66 rader, INGEN `ar_giltig`) →
  post-insert (71 rader, `ar_giltig` nestlad rad 43). Diffen = 5 rader insatta
  direkt efter `pub fn fran_romartal(...) {`.
- Facit: `cargo test` exit=101 — `E0432 no ar_giltig in the root` +
  dead_code-varning. Batt 0/3 (fail-fast vid pass 1).

**Rotorsak (dubbel):**
1. GTW: en position utanför barnantalet gick inte till append utan via `idx-3`
   in i ett SYSKONS mitt — tyst, utan klammer-/radmedvetenhet.
2. Motor2: felkodning — GTW:s DOKUMENTERADE append för `source_file` ÄR
   position 1 (`tests/insert_position.rs`); antagandet `position:1 = först`
   var fel (kommentar i tools.rs) och drev `top+1`.

**Fix (2026-10-13):**
- GTW `insert_node_at_path`: `position > children.len()` ⇒ append efter SISTA
  barnet (`children[len-1].end_line`); in-range `idx-3` oförändrat
  (bakåtkompatibelt).
- GTW regressionstest `insert_out_of_range_position_appends_at_top_level`
  (`tests/insert_position.rs`) — bevisat: faller på gamla koden (ny fn före
  `fn main`), passerar efter fixen.
- Motor2 `resolve_insert_target`: rot-append ⇒ position 1 + ärlig etikett;
  Motor2-testet korrigerat (pos 1, inte 3).

**Lärdom (§14):** Motor2:s test `root_insert_without_position_appends_last`
verifierade HJÄLPAREN (`resolve_insert_target`), aldrig den faktiska
insertionen — kodningen var aldrig slut-till-slut-testad. Klass: enhetstest
grönt, beteende fel (logger framgång utan effekt, §10).

## 2026-10-14 — FÖRSLAG: symbol-existens-varning före skrivning (Motor2-relä, användarens initiativ)

**Saknad funktion** (taxonomi: sub-funktion saknas för situationen).

**Situationen:** Motor2:s kodnings-batts visar huvudfelklassen "gör-men-fel-ställe":
modellen redigerar flitigt (5-6 GTW-edits) men med påhittade/fel-namngivna
symboler — `romartal::ar_giltig` som lib:en aldrig exporterat (E0432), symbol
utanför scope (E0425). Felkoden syns först vid NÄSTA build; modellen fick
aldrig facitet och gissade vidare. GTW ser bara syntax — men stor del av
klassen går att fånga heuristiskt UTAN full namnupplösning.

**Förslag: symbol-existens-lint i `edit_node`/`insert_node`/`validate`-svaret**

1. Efter en lyckad edit: plocka refererade identifierare och sökvägar
   (`modul::symbol`, `crate::modul::symbol`) ur den editerade noden.
2. Kolla mot tre källor:
   a. samma fils definitioner (fn/struct/enum/const/mod)
   b. filens `use`-importer
   c. entitets-indexet (`index_entities`/kunskapsgrafen — finns redan)
3. Saknas symbolen → **WARNING (icke-blockerande)** i verktygssvaret med
   fuzzy-förslag: "`ar_giltig` refereras men finns inte i index (närmast:
   `is_valid`)". GTW:s befintliga fuzzy-sökning som byggsten.
4. Import-existens: `use crate::x::y;` där `x` saknas i indexet → varna
   före build.

**Formkontrakt:** WARNING, aldrig block — typer/traits/macros/generiska
parametrar ger falska positiva, och flödet får inte strypas. Kompilatorn
förblir sanningen (Motor2 §28); GTW-lagret är före-skärmen: billig varning
i SAMMA anrop som editen, i stället för att vänta på nästa build.

**Motor2-sidan (motparten, planerad — Motor2-ROADMAP "EDIT-KVALITETSKEDJAN"):**
varningarna hamnar i tool-spåret; Motor2:s CompletionVerifier nekar ett
task_complete-claim en gång om varningar lämnats oadresserade (ingen
re-edit/build/läsning efter) — enforcement via verifieraren, inte via
modellens lydnad. Öppna varningar vid leverans → Kanban-item med
provenance (nästa tur ärver skyldigheten synligt).

**Varifrån:** användaren 2026-10-14 ("gtw varnar för saker som saknad
import ... men detta är bara varning?" → "förmåga skulle delvis kunna
läggas till i gtw?") + batt-bevis 1791963425 (session 9a7b1713, rev
905cefc): 5 Accept-edits, E0432 kvar, ingen testkörning innan leverans.

## 2026-10-14 (II) — PANIC FIXAD: delete på stale tree (byte/rad-slice klassen, Motor2-relä)

**Bugg** (panik, §28.2-klassen — 8:e återkomsten i släkten): Motor2-batt
1791979346 (session 1d694d17, Motor2 rev 0e9ecba) — `PANIC på
src/core/mod.rs:998:25: range start index 6 out of range for slice of
length 5` → Motor2:s streaming-task dog, turen aborterad
("Tur panik — avbruten av panic").

**Rotorsak:** `delete_node_at_path` tog `lines[..start_idx]` på en nod
med `start_line` BEYOND filens nuvarande rader — en STALE tree: källan
har krympt (edit/radering) sedan skeleton/analyze, modellen (eller
cachen) höll gamla nod-paths. Trädet och källan divergerade tyst.

**Fix (commit i detta repo, samma dag):**
- `delete_node_at_path`: vakt FÖRE slicen — `start_idx >= lines.len() ||
  end_idx > lines.len()` → ärligt anyhow-fel: "Node range (X-Y) is out
  of bounds for the current source (N lines) — the tree is stale;
  re-run analyze/list_nodes to get fresh node paths".
- Regressionstest `delete_with_stale_node_range_is_an_error_not_a_panic`
  (stale-simulering: källan krymper utan re-parse; testet vore PANIC på
  gammal kod). 245 tester gröna.

**Kvarvarande (notera, inte fixat här):** samma stale-slice-mönster KAN
sitta i syskon-funktioner som slicar på node.start_line/end_line
(insert-varianten tog redan append-vakten 2026-10-13). En sweep av alla
`lines[..`-slicer mot `lines.len()` är inköpslistan nästa GTW-pass.

**Varifrån:** Motor2-batt (användarens "fortsätt"-kedja, P5-passet) —
7 panic-träffar i test-loggen räknade samma panik (hook + task-abort +
poster).

## 2026-10-18 — `search_semantic`: Missing config (bge-base-modellen ej installerad)

**Klass:** saknad funktion/under-funktion saknas för situationen.

**Symptom:** `search_semantic` svarar
`Semantic search failed: Missing config: "/home/tuulikk/.gnawtreewriter_ai/models/bge-base/config.json" — check the local model with 'gnawtreewriter ai status'`
i en nystartad opencode-session (Motor2-passet, H2.0-kvällen). Samma fel
förekommer säkert i alla sessioner utan bge-base lokalt.

**Påverkan:** semantic-sökningen (diagnoskedjans steg 2, §35.1-tabellen
"Var sitter X?") är oanvändbar → omväg till grep (exakt sträng fanns, så
billigt här) — men för "hur görs X?"-frågor utan exakt sträng står
agenten helt utan semantisk väg. `sense`/satellit kan drabbas samma väg.

**Önskad vägledning träffad?** Delvis: felmeddelandet ÄR ärligt (namnger
saknad modell + `ai status`), men det finns ingen tydlig
installations-väg beskriven ("check the local model" → HOW?). Antingen
(a) dokumentera install-kommandot i felmeddelandet, eller (b) låt
`search_semantic` degradera med ett tydligt svar istället för en error
(t.ex. "semantic index saknas — kör index_project; faller tillbaka till
search_nodes"). (b) föredras — verktyget ska aldrig vara helt borta.

**Varifrån:** Motor2 H2.0-passet 2026-10-18 (append-only-minnen), GTW
anropades enligt §35.3-ritualen, ett försök → grep-omväg enligt §35.2.

**Uppföljning 2026-10-19 (Motor2 H3-sliding-passet):** SAME fel igen —
`search_semantic` i en ny session ("sliding prune counter-var sitter?")
→ samma Missing-config-svar → grep-omväg (precis sträng fanns: "sliding").
Kvarstår oförändrad; frågan "vad krävs för att bge-base installeras
lokal" är fortfarande obesvarad i felmeddelandet. Träffräkning för
denna klass: **2 sessioner**.

## 2026-10-19 — edit NO-OP returneras som Err (claim-guard) mäts som misslyckad tool-call i konsument-metering (Motor2-relä)

**Klass:** kontrakt-design (vilken kanal en medveten NO-OP använder).

**Symptom:** Motor2s sliding-marathon pass 5 (session 220c225d) —
agenten editerade marker-answer.txt med innehåll som redan var korrekt
(från pass 4) → GTW edit_node svarade Err("NO-OP: new content is
identical…") → Motor2s gtw-verktyg mappade till success=false →
tool_calls-rad success=0 med v=Accept → organ-mätaren ("missfall =
success 0 utan vakt-stämpel") dömde editing-gruppen RÖTT fast ALLA
disk-facit var gröna och turen levererade.

**Kontrakt-analys:** NO-OP-guardet är RIKTIGT (tvinga agenten att
verifiera avsikten — inte påstå en edit som ändrar 0 bytes). Men Err
gör att VARJE konsument-metering med "Err = misslyckad tool-call"
missuppfattar en ärlig notice som missfall. Förväntat kontrakt: edit
returnerar Ok med applied=false + no_op-flagga; Guardian-block får
vara Err.

**Önskad vägledning:** ändra GTW edit-kontraktet (NO-OP = Ok-resultat
med applied=false/no_op-flagga) eller dokumentera i edit_node-
beskrivningen hur konsumenter skiljer notice-Err från missfall-Err.
Motor2-fakta: vår wrapper (motor2-gtw tools.rs) mappar NO-OP-Err →
success=true med "no_op": true — LAPP som rivs när GTW-kontraktet
ändras (§36-regeln).

**Varifrån:** Motor2 sliding-marathon-passet 2026-10-19 (mätning-
facit, syko-relationship via path-dep; record i GTW_MCP_ISSUE_LOG
enl behandla ändamålet).

## 2026-10-21 — batch saknar förankrad text-insert: 20 enrads-inserts i 7k-raders fil krävde Edit-omväg (Motor2-relä) — LÖST samma dag: `insert_text` i 0.20.0 (882abe8)

**Klass:** saknad sub-funktion för situationen (batch-insert utan
node-sökvägar).

**Symptom:** Motor2 §37.4-köjusteringen — 20 process-tester i
crates/motor2-llama/src/model_runtime.rs (~7 000 rader) skulle få en
identisk `let _slot = …`-rad som första sats. GTW batch (v0.3.2-dok,
BATCH_USAGE.md) dokumenterar bara node-path-"edit"-operationer —
*RÄTTELSE (§41.3, samma dag): enumen hade en `insert`-variant, men den
är node-trädbaserad (parent_path + barn-index), odokumenterad, och
saknar precis det som behövdes: text-ankare. Att hämta 20 node-sökvägar
ur en fil i get_skeletons kända storlekssvacka är högrisk; text-
Edit-verktyget med unika tvåraders-ankare (fn-namn + unik första
sats) + post-hoc grep-räkning (32/32 fasit) gjorde jobbet i 4 grupper.

**Kontrakt-analys:** situationen "många identiska enrads-inserts på
unika text-ankare, ETT stort fält" är vanlig i testtäckningsarbete.
Batchens atomicitet (allt-eller-inget + en undo) vore precis rätt —
men är onåbar utan node-sökvägar, som är dyra/otillförlitliga i
stora filer.

**Önskad vägledning:** batch-operation "insert" med text-ankare
(find/replace-form: anchor + position before/after), eller quick-mode
med multi-op per fil. Då blir testtäcknings-batchar atomära i stället
för 4 sekventiella Edit-grupper med manuell räkne-verifiering.

**Varifrån:** Motor2 köjusteringen 2026-10-21 (§35.2-loggning; omvägen
var tillåten och facit-verifierad, men nästa agent ska slippa valet).

**UPPLÖST 2026-10-21 (samma dag):** levererad som batch-op
`{"type": "insert_text", "file", "anchor", "position": "before"|"after",
"content"}` — unikt-ankar-kontrakt (0/2+ träffar = hard error med
antal), raw splice, samma rules+syntax-guards som node-ops,
multibyte-säkra splice-punkter, blandbar med node-ops. 8 enhetstester
+ E2E via CLI. 0.20.0, commit 882abe8. Ovägen (Edit-grupper) behövs
ej längre för denna situation.

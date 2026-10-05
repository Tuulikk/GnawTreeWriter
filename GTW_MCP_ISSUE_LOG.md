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

---

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

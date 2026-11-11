## [0.26.1] - 2026-11-11

### Changed (sense) — tröskeln är nu MÄTT, och 0,68 föll

- **`ai recall-eval --negatives <fil>`** mäter om ett enda cosinus-tal kan skilja
  "besvarbar" från "begrepp som inte finns" — och vägrar föreslå ett tal när
  mängderna överlappar. `evals/sense_negatives.json` (18 frågor) är negativmängden;
  positiva sidan är den befintliga recall-samlingen (50 frågor), så *träffens*
  cosinus mäts i stället för att minnas.
- **Mätresultat (bge-base-en-v1.5, 2026-11-11):** positiva min 0,586 / p50 0,758 /
  max 0,927; negativa min 0,505 / p50 0,622 / **max 0,714**. Separation
  **−0,127** → mängderna överlappar, så inget enskilt tal kan avgöra.
- **`SENSE_ANSWERABLE_COSINE` 0,68 → 0,72.** 0,68 var fel i *båda* riktningarna:
  den släppte igenom 3 av 18 frånvarande begrepp (de nådde 0,714) och kallade
  ändå 5 av 50 riktiga frågor overifierade. Vid 0,72: **0/18 negativa passerar,
  12/50 riktiga frågor läses som overifierade (24 %)** — ett medvetet val av
  felet "overifierad" framför "felaktigt självsäker", dokumenterat i konstantens
  doc-kommentar med tabellen.
- `answerable` är fortfarande en **etikett med cosinus bredvid sig**, inte en dom:
  överlappet är bevisat, och siffran får aldrig låtsas vara en klassificerare.

### Fixed (CI) — två röda `Validate` var grinden som gjorde sitt jobb

- **`Validate` failade på `29bb9a70` och `eaa078de`, grönt på `e28e29e8`.**
  Orsaken var inte Actions: `code-index.md` var inaktuell i de två första
  commitarna (jag ändrade källkod utan att regenerera artefakten) och färsk i
  den tredje. Det nya CI-steget `code-index . --check` fångade precis det det
  finns för — vilket betyder att det fungerar, men också att flödet saknade
  regenereringen.
- **Hooken** (`install-hook`) skrev bara storen (`--db`), aldrig den **spårade**
  artefakten. Nu skriver den även `code-index.md` när filen finns — men bara då,
  så repon utan artefakt förblir orörda.
- **`commit-build.sh`** uppdaterar artefakten **efter** install (med den *nya*
  binären, eftersom både versionsstämpeln och extraktionen kan skilja), committar
  den separat om den ändrats, och avslutar med `RESULT fail:code-index` i stället
  för `ok` om den fortfarande är inaktuell. Ett träd som lämnar skriptet kan
  alltså inte längre pushas rött.
- **`Cargo.lock` tas nu med i samma eftercommit.** Versionsbumpen lämnade den
  osmutsad-i-kommitten, så `v0.26.1`-taggen pekade först på ett träd vars
  lockfil fortfarande sa `0.26.0`. Taggen flyttades när det upptäcktes — en tagg
  som namnger fel version är ett tyst löfte om något som inte finns.
- Not: `--skip-commit`-grenens kommentar om att artefakten "alltid är dirty"
  gällde den rev-stämplade versionen och togs bort i 0.26.0 — nu är den bara
  dirty när innehållet faktiskt ändrats, vilket är exakt när den ska committas.

## [0.26.0] - 2026-11-11

### Added (MCP) — `sense { root }`, broker per rot, och modell↔index-vakt

- **`sense` kunde bara söka i daemonens cwd**, så en klient startad någon
  annanstans fick svar från ett främmande träd (fältfynd 2026-11-10). Vakten
  fångade `$HOME` och `/`, men inte "en förälderkatalog med flera repon".
  Nu: `root`-parameter (canonicaliseras, måste vara katalog, samma
  `project_root::guard`), broker-cache **per rot**, och svaret säger vilken rot
  som användes. Bevisat från `/tmp` → 10 träffar i repot.
- **Modell↔index-vakten:** `AiManager::index_model()` är enda ägaren till
  "inbäddaren som vektorindexet byggs med" (indexeraren hårdkodade `Bge` medan
  cache-roten probades på `modernbert`). `store_model_mismatch()` jämför storens
  `model_info` mot den upplösta modellen och `sense` skriver en högljudd rad
  (fält `model_mismatch`) i stället för att tyst ranka inkompatibla vektorer.
- En rot utan `.git`/`.gnawtreewriter_ai` säger nu att den saknar index och
  pinnade modeller — bevisat: `root: /tmp` gav lexikala nodnamn som såg ut som
  semantiska träffar.

### Fixed (CI) — grinden som ingen gick igenom

- `code-index --check` fanns men kördes **inte** i CI, så `code-index.md` var
  inaktuell på ren master. Nu i `validate.yml` (DB-fri re-render + jämförelse).
- Artefaktens titel berodde på hur roten stavades (`code-index .` gav
  `# CODE-INDEX — .`) — roten canonicaliseras före namnet; bevisat byte-identisk
  mellan `.` och absolut sökväg.
- `doctor`s grafkontroll varnade permanent (13 % mot en 60 %-bar när taket är
  ~10–13 %). Nu regressionsgolv 5 %, nämnaren utskriven och meningen säger att
  `calls`-baserad impact är en undre gräns. `doctor` = 29 kontroller, 0 varningar.

### Docs — Crush flaggad otestad; måsten i AGENTS.md

- **Crush: `crush` finns inte på utvecklingsmaskinen** → guide, `SKILL.md`,
  `KNOWN_ISSUES` och det genererade `.skill`-paketet säger nu uttryckligen att
  ingen Crush-session någonsin kört, och att `pack-skill --check` bara bevisar
  synk mot `SKILL.md`.
- **`AGENTS.md` 2.3: M1–M12 som bindande måsten**, varje regel med kommandot som
  bevisar den, de fem konkreta felfallen de finns för, och en
  "definition of done" att skriva ut punkt för punkt.

### Fixed (AI) — modellcachen löses per modell, inte per manager

- **`AiManager::new` valde EN cache-rot genom att söka efter `modernbert`.**
  Den proben avgjorde därför var *alla* modeller letades: ett projekt med lokal
  `modernbert` gjorde att `bge-base` slogs upp i `<projekt>/bge-base` — som inte
  finns — trots att den globala cachen hade den, och `sense` rapporterades
  "degraderad" med modellen installerad.
- `ModelSource`/`ResolvedModel`; `AiModel::{dir_name, probe, id}` är enda sanningen
  (sökvägar och probe-filer uppfanns tidigare på tre ställen).
  `resolve(model)` söker lokal → global, sidoeffektfri.
- `get_status`-flaggorna och modellistan kommer från **samma** uppslagning och
  kan inte längre säga emot varandra. `AiStatus` får `bge_installed` och
  `models` (additivt — `### BREAKING (lib)` gäller inte läsare).
- `ai status` skriver varje modell med sökväg och rot (`local`/`global`).
- Tester (hermetiska, markörfiler): en lokal `modernbert` gömmer inte en global
  `bge-base` — exakt det fall som föll förut.

### Fixed (sense) — en ägare till cosinuströskeln, och domen följer med svaret

- **Tröskeln 0,68 låg i `cli.rs`** och nådde därför bara CLI:t: MCP-anropare —
  ytan agenterna använder — fick ingen varning alls. Dessutom fanns **två tal för
  en bedömning** (0,68 i `cli.rs`, 0,5 inline i `gnaw_sense.rs`).
- `SENSE_ANSWERABLE_COSINE` (*är frågan besvarbar?*) och `SENSE_COSINE_LOW`
  (*är träffen rimlig?*) är nu separata, namngivna och bor hos måttet;
  `cli.rs` aliasar konstanten i stället för att definiera ett eget tal.
- `SatelliteQuality` får `answerable` + `cosine_threshold`, så en MCP-anropare kan
  skala likadant i stället för att hitta på egen tröskel. `answerable=false`
  betyder *overifierad ledtråd*, inte *svar*.
- `answerable_from_cosine()` utbruten och pinnad: 0,63 → false (frånvarande
  begrepp), 0,71 → true (sann toppträff), `None` → `None` (inga träffar är inte
  "obesvarbart").

### Docs — grafens tak publicerat, inte bortförklarat

- README säger nu vad grafen kan svara på: **10,2 % av `calls`** löses
  (2 166 / 21 304; `uses` 12,2 %, `imports` ~9,7 %), att taket beror på att
  `plain_name` (17 131) kräver **typinferens**, och att `calls`-baserad
  `impact` därför är en **undre gräns**. Samma siffror går att mäta per repo med
  `code-index --coverage`.

### Added (CLI + MCP) — `undo --files`: filskapande är en egen omfattning

- **`scaffold` loggade `Insert`, så en skapad fil kunde aldrig ångras.**
  `OperationType::CreateFile` införd; `scaffold` loggar den med innehållshash.
  Vanlig `undo` **vägrar** den och hänvisar till `undo --files` — innehålls-
  omfattningen raderar aldrig filer (ägarens distinktion: att återställa
  innehåll och att radera en fil är olika operationer med olika risk).
  `undo_file_creation()` tar den nyaste skapade filen **som fortfarande finns**
  och bara om den är byte-identisk med det GTW skrev (hash-bevis). `redo` av ett
  skapande vägrar ärligt: loggen bär en hash, inte innehållet.
  Bevisat 2026-11-11: vägra → radera → uttömt → ändrad-fil-vägran → kedja.
  Två buggar hittade av mina egna prov: en vägrad undo konsumerade stackposten,
  och en post vars fil redan var borta blockerade stacken för evigt.

### Added (validate) — heal-rådet är mätt, inte antaget

- **`healer::heal_advice()`** besiktigar filen och svarar antingen *"this IS a
  heal case"* (med kommandot) eller *"braces are balanced, so this is NOT a heal
  case"*. Samma funktion matar CLI och MCP `validate` (kan inte drifta), pinnad
  av `tests/heal_advice.rs` i båda riktningarna. Stänger den sista öppna delen
  av "ingen väg tillbaka från en oparsbar fil".

### Added (lib) — `core::experiment`, ej kopplad till CLI

- Modulen kompilerar nu som del av craten (registrerad i `core/mod.rs`).
  **Strukturell defekt rättad:** `git()` hade tappat sin svans (`Ok(...)` +
  klammer), vilket nästlade `ensure_disposable`, `start`, `latest_dir`, `load`
  och `append` inuti den. Klamrarna balanserade, så `heal` rapporterade korrekt
  "parses cleanly" — bara kompilatorn kunde se nästlingen.
  CLI/MCP-koppling är **medvetet inte** applicerad; den sparade patchen är
  bevisat trasig (hunk 3 landar inuti en rå sträng i `src/cli.rs`).

### Fixed (MCP) — en `$HOME`-rot betjänade främmande träd

- **`sense` och `code_index` avvisar nu en projektrot som är `$HOME` eller
  `/`** (`root_is_too_broad` + `broad_root_error`, kopplade i `handle_sense`
  och `handle_code_index_mcp`). Fältfynd: en MCP-daemon startad med
  `cwd=$HOME` svarade en `sense`-fråga med 5/5 träffar i `~/AndroidSDK`,
  rapporterade anroparens egen store som frånvarande och degraderade den
  semantiska kanalen — tre symptom, en felaktig rot. Rötter kommer från
  `std::env::current_dir()`; felet var att den ärvdes utan vakt, inte att
  något var hårdkodat.
- Test: `root_guard_tests` (3 fall, rena — ingen filsystem-beroende).

### Fixed (version) — staleness-larmet var avklippt

- **`build.stale` var alltid `false` i en installerad binär.** Båda
  jämförelserna läste `CARGO_MANIFEST_DIR` med `std::env::var` i **runtime** —
  variabeln finns bara under bygget — så `run_git` gav upp (`head_moved =
  false`) och mtime-kollen föll tillbaka på cwd. `doctor` rapporterade "färsk"
  för en 0.25.0-binär mot 0.25.1-källa. Nu `env!` (kompileringstid), med
  regressionstest `staleness_check_can_reach_git_and_the_source_tree`.
  Bevisat live: `touch src/version.rs` → `⚠ BINARY STALE`, ombygge → tyst.
- Hookens bash-koll (`src/cli.rs:2419`) fungerade hela tiden — två
  implementationer av samma fråga, en levande och en död.

### Known (kvar) — ärligt om det som INTE är klart i 0.26.0

- **Grafens `calls`-upplösning är ~10–13 % och stannar där** tills typinferens
  finns: `plain_name` (17 131 anrop som `self.parse()`, `x.iter()`) kräver
  mottagarens typ. Se README "What the graph can and cannot answer"; `impact`
  över `calls` är en **undre gräns**. Nästa steg är scoped resolution i nivåer
  (`same_file | import | unique_global`), inte en lapp i resolveren.
- **Cosinuströskeln (0,68) är fortfarande provisorisk** — 7 frågor. Den
  *rapporteras* nu (`answerable` + `cosine_threshold`) i stället för att avgöra
  tyst, vilket gör provisoriet ofarligt men inte färdigt. Kalibrera mot
  `evals/sense_recall.json`.
- **`core::experiment` är bibliotek-only** (CLI-koppling medvetet inte
  applicerad; den sparade patchen är bevisat trasig — hunk 3 landar inuti en rå
  sträng i `src/cli.rs`). Modulen kompilerar och är dokumenterad som parkerad.
- **Crush är otestad** — ingen Crush-session har någonsin kört (se
  `CRUSH_INTEGRATION_GUIDE.md` och `KNOWN_ISSUES.md`).

## [0.25.1] - 2026-11-10

### Changed (CLI) — `--db` skriver artefakten i stället för att dumma den

- **`code-index <root> --db`** skriver nu **`<root>/code-index.md`** — samma
  default som `--check` redan använde — och bekräftar med en rad
  (`code-index skriven: … (N rader)`). Tidigare gick hela dokumentet till
  stdout, så en *refresh* dumpade ~2 300 rader markdown i terminalen, och både
  hooken och operatören fick upprepa `-o <path>`. Tre ställen kände filnamnet,
  bara ett gjorde det till default.
- **`--stdout`** behåller gamla beteendet: `--db --stdout` skriver inget, bara
  skriver ut.
- **Hooken** behöver inte längre `-o` (genereras om vid `--install-hook`).
- `code-index <root>` **utan** `--db` är oförändrat: renderaren skriver till
  stdout så den kan pipe:as.

**Brytande för skript:** `code-index . --db > fil.md` skriver inte längre till
stdout. Använd `--stdout` (behåll pipe) eller `-o <path>` (explicit fil).



### Fixed — `edit_node` förstörde dokument- och configfiler

De fem "position-blinda" parsrarna (markdown, css, yaml, toml — och json via
sitt interim-spår) gav noder **utan byte-spann**: `start_col`/`end_col` var 0
och `content` var ofta rekonstruerad text i stället för en källslice. Splitten
föll då tillbaka på **helrads-ersättning**, vilket tyst skrev om grannnoder.
Verifierat mot 0.24.0-binären innan fixen:

- **markdown:** rubrik-edit skrev tillbaka enbart texten och **raderade `#`**;
  stycke-edit **åt upp blankraden** efter stycket. Ny parser: `tree-sitter-md`
  (byte-exakta spann + varje rubriksektion nästlas under föregående).
- **css:** edit av ett deklarationsvärde **ersatte hela regelraden** med värdet.
  Ny parser: `tree-sitter-css` (var tidigare avstängd i `Cargo.toml` med
  motiveringen "version conflicts" — proben visar att den motiveringen inte
  längre gäller).
- **yaml/toml:** noder rapporterade rad 1, kolumn 0-0 → alla edits **nekades**
  av Duplex-vakten, dvs formaten var i praktiken read-only. Nya parsrar:
  `tree-sitter-yaml` resp. `tree-sitter-toml-ng` (`pair`-noder låter värdet
  redigeras utan att nyckeln rörs).
- **panik vid leaf-edit:** `TreeVisualizer::generate_sparkline` delade med noll
  när den renderade en ensam nod (max-djup 0 — normalfallet när en **leaf**
  redigeras). Krashen kom **efter** att editen skrivits och verifierats på
  disk, så anroparen såg ett fel på en lyckad operation. Guard: `max(1)`
  (`visualizer.rs`, `visualizer_test.rs`).

## [0.25.0] - 2026-11-10

### Added — språk

- **PowerShell** (`.ps1`, `.psm1`, `.psd1`) — 20 filer i arbetsytan, ingen
  parser tidigare. Kända grammatikegenheter: tom fil ger ERROR-noder, och en
  funktionskropp med flera statements på EN rad ger ERROR vid `param`.
- **CMake** (`.cmake` + `CMakeLists.txt` via filnamn).
- **Makefile** (`Makefile`/`makefile`/`GNUmakefile` via filnamn).
- **Ruby** (`.rb`) — inte en ny funktion utan en **reparation**:
  `get_parser_for_language` mappade `ruby`/`rb` → `"rb"`, men `get_parser`
  hade ingen `"rb"`-arm, så Ruby föll tyst till `GenericParser` (en enda nod)
  och `language: ruby`-regler kompilerade mot ett tomt träd och matchade
  aldrig.
- **Tillägg ovanpå befintliga språk** (noll nya grammatiker): `.pyi` → Python,
  `.mts`/`.cts` → TypeScript, samt **filnamnsdetektering** för ändelselösa
  filer (`.bashrc`, `.bash_profile`, `.bash_aliases`, `.zshrc`, `.zshenv`,
  `.profile` → bash-parsern). `Path::extension()` är `None` för dem, så de
  föll tidigare till `GenericParser` trots att parsern fanns.
- **`code_index`-default:** 15 → 32 ändelser — bara sådana som har en riktig
  parser (`kt/kts`, `swift`, `cs`, `dart`, `svelte`, `sql`, `qml`, `slint`,
  `mjs/cjs`, `cc/cxx/hxx`, `pyi`, `mts/cts`, `sh/bash`). Nio grupper hade
  parser men indexerades aldrig.

### Changed

- `pulldown-cmark` borttagen: noll användningar i `src/` (markdown-parsern
  använde regex — `LANGUAGE_SUPPORT_PLAN.md` påstod pulldown-cmark).
- Ny delad konverterare `parser::tree_from_ts_node` + `parse_with_tree_sitter`:
  byte-spann-logiken bor på ett ställe i stället för att kopieras per parser.

### Added — graf-frågor (P2) och planerings-ytan

- **`code_graph`** — graf-frågor över code-index-storen: `reachable` (SQL
  rekursiv `UNION`, så cykler terminerar; djupgräns som parameter), `cycles`
  (iterativ DFS, rotationskanoniserad, djupbegränsad), `chain` (BFS med
  förälder vid första upptäckt) samt `impact` + `resolve_symbols` (ett bart
  *namn* slås upp mot `entities.name`; okänt namn = synligt tomt).
- **MCP:** `code_index kind=cycles|impact` med `symbol`/`depth` i schemat.
- **CLI:** `code-index --cycles`, `--impact <namn>`, `--why <namn>`,
  `--depth N` — read-only, kräver store.
- **`--why <namn>`** — per utgående kant: `resolved`, `to_id`, `target_defs`
  och `verdict` (`unique_in_index` / `ambiguous_pick` / `unresolved`). Gör ett
  grafsvar falsifierbart på ett anrop.
- **`meta`-kuvert på varje grafsvar:** `store_rev`, `worktree_rev`,
  `worktree_dirty`, `index_stale`, `static_only` — svaret bär sin egen
  osäkerhet i stället för att se ut som ett faktum.

### Fixed — sex agent-vända CLI-hål

| Före | Efter |
|---|---|
| `--version`/`--help` → exit 1 + stderr | `e.exit()`: stdout + 0; användningsfel → 2 |
| `analyze` → exit 0 när inget parsades | non-zero (delscan behåller 0) |
| `insert` panikade på **varje** anrop i debug-bygge | `position: Option<usize>` (default 1) |
| `curate` två `-m`, `pack` två `-i` (×2), `restore` bool-positional | long-only / flagga |
| cachad numerisk path skrev **tyst fel nod** | `edit --expect-name <namn>` → avvisar, fil orörd |
| `quick-replace '-1'` krävde `--` | `allow_hyphen_values` |

Nya grindar: `tests/cli_contract.rs` (`Cli::command().debug_assert()`) och
`tests/cli_behaviour.rs` (kör den riktiga binären: exit-koder/strömmar).

### Changed (beteende) — konsumenter påverkas

- **Resolutionen är striktare (extraktor v3).** Kors-fil-mål som är *metoder*
  resolvas inte längre: ett namn som är unikt i indexet är inte unikt i
  verkligheten (std indexeras inte). Mätt på GTW:s egen store: resolvade
  call-kanter **3 898 → 2 064**, kors-fil metodmål **1 848 → 0**, fler-nods­cykler
  **338 → 29**. `EXTRACTOR_VERSION` 2 → 3 ⇒ full ombyggnad vid nästa refresh.
  **Motor2 m.fl.:** räkna med lägre `to_id`-täckning — det som försvinner är
  fabricerade kanter, och `coverage` redovisar dem som `unresolved`.
- **Exit-koder** enligt ovan (skript som grenar på `!= 0` påverkas inte;
  `== 1` som "användningsfel" gör det).
- **`refresh()` vägrar nedgradera** en store byggd av en nyare binär.

### Fixed — post-commit-hooken kan inte längre skriva sönder storen

Hooken valde `gnawtreewriter` från PATH (den *installerade* binären). Med en
äldre binär skrev den över en nyare store med äldre regler (v2 över v3, med
fabricerade kanter tillbaka) och krympte `code-index.md` (32 → 15 ändelser).
Nu: `target/release` → `target/debug` → PATH, och **hoppar över** med en rad på
stderr när binären är äldre än `Cargo.toml`/`src`.

### Known (medvetet kvar)

- **JSON** går fortfarande på serde + `json_semantic_edit` (J1). Det fungerar
  (värde-edit ger giltig JSON), men paths är **nyckelnamn**, inte numeriska —
  ett byte till `tree-sitter-json` ändrar varje JSON-path och måste göras
  tillsammans med den funktionen. Separat fynd: `--preview` visar den råa
  splitten medan skarp edit tar den semantiska vägen; preview och apply är
  oense för JSON.
- **`.h` → C-parsern** (även för C++-headers med `class`/`template`). I den här
  ytan finns 49 `.h` mot 50 `.cpp`. Heuristik (`.h` med `.cpp`-syskon → cpp) är
  en beteendeförändring på befintliga filer och kräver egna tester.

## [0.24.0] - 2026-11-10

### Added — code-index coverage (fas 1), extraktor v2 och attribut-guard

- **`code-index --coverage` / MCP `code_index mode=coverage`**: grafkvalitet
  ur storen — upplösningsgrad per relationstyp, D7-källklasser (matched /
  synthetic_callsite / function_scope_unmatched / file_scope_unmatched /
  other), target-klasser (resolved / unresolved_macro /
  unresolved_qualified_path / unresolved_plain_name), symbols-sammanfattning
  och meta (extractor/generation/rev). Read-only: öppnar aldrig en store
  (beslut #8), loud error utan store med refresh-hint.
- **Extraktor v2** (`EXTRACTOR_VERSION = "2"` → full ombyggnad vid nästa
  refresh): metodanrop (`obj.method()` — metodnamnet är sista identifier-
  barnet, aldrig mottagaren), scoped-path-anrop (`super::`/`crate::`),
  makron (bang-suffix → `unresolved_macro`-klassen) och
  relationstyp-medveten implements-upplösning (matchar bara `kind=trait`).
- **Före/efter på GTW-repot (147 filer):** relations 4 650 → 22 095;
  resolved call-kanter 890 → **3 848 (4,3×)**; käll-matched 49,4 % →
  70,7 %. Upplösningsgrad per typ SANK (calls 34,8 → 19,2 %) — nämnaren
  växte med ärligt olösligt (stdlib + makron); mät per konstruktion,
  inte total-raten. implements 88,5 → 39,7 % är en KVALITETS-höjning:
  de gamla 69 innehöll fel-mål-resolutions till impl-entiteter, de nya
  31 pekar alla på riktiga traits (SQL-join-verifierat 31/31).
- **`E_DUPLICATE_ATTRIBUTE`-guard** i edit-funneln
  (`check_duplicate_attribute_above` + hook i `edit_node_at_path`):
  replacement-content som åter-included attributet ovanför målnoden
  avvisas högljuddt — dubbel-`#[test]`-fällan (giltig syntax, testet
  registreras och körs två gånger) kan inte längre uppstå tyst.
  Enhetstest med 4 fall (kollision, korrekt form, annan attribut-typ,
  doc-kommentar stoppar skanning konservativt).
- **Hermetisk fixtursvit** `tests/code_index_fixtures.rs` (D8-multiplicitet,
  F7-ambiguitet, post-C-inventering 11/5 — pinnad på MÄTTA värden) och
  handmärkt facit-projekt (`testprojekt/code-index-fixture/`, FACTS.md).
- **`docs/`**: `CODE_INDEX_RELATION_HISTORY_PROPOSAL.md` (revisionsbunden
  relationsgraf; granskningsbeslut D1–D10 + T1–T5), `HANDOVER-LATEST.md`
  (Motor2-mönstret), `AGENTS.md` OpenCode 2.x-sektion (klient–server,
  per-katalog-daemner, binär-pickup-kedjan).

### Changed — hjälp-manifestet synkat (var 28 av 54)

- `gnawtreewriter commands` listade 28 av 54 kommandon (code-index,
  validate, gnaw-* -familjen och AI-familjen saknades — manifest-drift
  sedan tidigare push). Synkad till fullständig lista; den varaktiga
  fixen (clap-genererad introspektion) är backlog. `--help` från clap
  var alltid korrekt.
- `GTW_INSTRUCTIONS.md`: coverage-läge + verktygsräkning 37 → 38.

### Fix

- `migrate_connection` delad mellan fil-backad öppning och in-memory
  test-schema — en DDL-sanning (fixtur-testerna använder exakt
  produktionsschemat).

## [0.23.1] - 2026-11-08

### Fixed — J1: JSON-strängnoder redigerbara i båda citat-formerna

- **Rotorsak** (rapport 2026-11-05 från Motor2-agenten, via gnawdrivers
  `GTW_MCP_ISSUE_LOG`): den legacy-JSON-parseren är **positionsblind** —
  alla noder får rad 1..1/kolumn 0 — så `edit_node_at_path:s` rad-splicing
  kan aldrig fungera för JSON. Båda citat-formerna nekades: ociterad text →
  "expected ident" (inte giltig JSON), citerad → "trailing characters"
  (splicingen förstörde dokumentet och re-parsen föll).
- **Fixen är semantisk, inte syntaktisk**: JSON redigeras på VÄRDE-nivå
  (`json_semantic_edit`) — nod-pathen navigeras (nycklar + array-index,
  samma form som `JsonParser` bygger), värdet ersätts, hela dokumentet
  re-serialiseras deterministiskt (pretty 2-space + avslutande newline).
  Rad-splicing och B2-radkontroll är rätt för kod med riktiga span — för
  JSON är värde-nivån abstraktionen.
- **Sträng-noder tar BÅDA formerna**: citerad (`"text"`) tolkas som
  JSON-värde; ociterad text blir literal-strängen. Ingen agent ska behöva
  gissa citat-form (ergonomi-poenget ur rapporten).
- **`serde_json` får `preserve_order`** — utan den hade varje JSON-edit
  sorterat om ALLA nycklar (BTreeMap) och varit diff-brus över hela
  filen; nu bevaras nyckelordningen (regressionstest).
- **Edit ändrar, den skapar inte**: okänd nyckel/utanför-gräns-index
  nekas högt (samma kontrakt som kod-språken; insert för tillägg).
- Båda vägarna delar fixen: MCP `edit_node`/`batch` + CLI `edit` (de går
  genom `GnawTreeWriter::edit`). Guardian/receipt/transactions oförändrade.
- Regressioner: `json_edit_both_quote_forms_and_nesting`,
  `json_edit_preserves_key_order`, `json_edit_refuses_structural_sneak`.
  Live-bevis: båda formerna genom riktiga CLI:t på en flödesliknande fil.
- **NOTERING — TILLFÄLLIG LÖSNING (användaren 2026-11-08):** värde-nivå-editen
  är ett INTERIM. JSON ska ha en riktig AST-parser med span som de andra
  språken (tree-sitter-json — idag medvetet avstått pga versionskonflikter —
  eller en egen span-spårare); då återfår JSON kirurgisk byte-precision och
  re-renderingen försvinner. Uppföljning parkerad tills behov/bevis visar
  sig (formateringsnormalisering är den enda kända kostnaden idag).

## [0.23.0] - 2026-11-04

### Fixed — B4: search_nodes alternation (`a|b` is OR)
- **`a|b` is alternation** (GTW_RAPPORT_2026-11-01 B4): each pipe-separated
  alternative is a substring match; results merge, deduped by node path,
  and every match names `matched_pattern`. Single patterns behave as
  before. An alternation with no non-empty alternatives errors loudly
  ("Empty search pattern") instead of the silent 0-hit answer B4
  reported. Grammar documented in the tool schema + GTW_INSTRUCTIONS.
  CLI parity: `gnawtreewriter search <file> "a|b"` shares the same
  alternation semantics (dedup + matched pattern in the output; loud
  error on empty alternation) — CLI and MCP no longer diverge on this,
  same anti-divergence class as B3's shared skeleton builder.
- tests/mcp_integration.rs: alternation finds both symbols (dedup +
  matched_pattern), single patterns unchanged, empty alternation errors.

### Fixed — B2: partial-replacement guard (loud rejection, nothing written)
- **`edit_node` now proves the replacement landed 1:1**: a remnant
  detector (`detect_replacement_remnant`) checks after every edit that
  the OLD node's distinctive content does not survive below the new
  content — the exact B2 signature ("rest of the OLD code below the
  new", duplicate definitions downstream, tool said success). Soundness:
  lines that already lived OUTSIDE the node's span in the original are
  other code, never remnants (this was the false-positive class the
  first detector version had — caught by the guardian reduction test).
  On detection: loud rejection, nothing written, B2 class named.
- Dogfood: the full funnel test-suite (300+ edits incl. doc-swallow,
  attribute, batch flows) runs through the guard without a single false
  positive; the crafted B2 signature fires.

### Added — B3: skeleton outline mode (named structure by default)
- **`get_skeleton` / `skeleton` default to OUTLINE**: named nodes only —
  comments, use/import statements, attributes and unnamed siblings
  collapse into a count ("N unnamed/comment/attribute node(s) collapsed
  — pass --raw to include"). The big-file answer is orientation, not a
  flat 289-row dump (B3's exact complaint).
- `--raw` (CLI) / `named_only: false` (MCP) restores the old full dump.
  Shared builder (`core::skeleton_outline`) — CLI and MCP can no longer
  diverge. MCP payload gains `named_only` + `collapsed`.

### Added — B1: path freshness — expect-guards + receipt path/hash
Implements the B1 wish from `GTW_RAPPORT_2026-11-01.md` (and pays for the
2026-11-03 incident, which was B1's cost in practice):
- **`EditReceipt` gains `new_node_path` + `node_content_hash`** (B1a):
  the addressed node re-resolved in the fresh tree after write, with its
  content hash — the freshness key for consumers replaying stored paths.
- **`edit_node` gains `expect_name` / `expect_kind`** (B1b): declared
  identity checked BEFORE any write; mismatch rejects with
  **`E_PATH_STALE`** and the guidance "re-run search_nodes" instead of
  writing the wrong node. Works with numeric dot-paths and `@kind:name`
  anchors (anchors are self-validating; the guard matters for
  positional paths).
- `GnawTreeWriter::expect_node` exposes the same guard to library
  callers. `@`-semantic paths were never stale-prone (resolved BY name);
  the guard targets positional paths — the replay case.
- tests/b1_path_validation.rs + an MCP integration test: receipt fields,
  mismatch rejects before writing, the 2026-11-03 incident shape is
  caught by an expect declaration.

## [0.22.0] - 2026-11-03

### Added — capability ladder (2026-10-31 proposal, spår 2)
- **Tools degrade, never die — and name the level that answered.**
  Ladder: `bas` (parse/AST/edit — always) → `struktur` (code-index store
  + knowledge graph — always when tree-sitter parses) → `semantik`
  (bge-base ranking). sense/search_semantic responses carry a `level`
  field (`semantik` on the model path, `struktur` on the T5 lexical
  degrade); `doctor` reports the ladder (`capability` field + CLI line).
  The always-works guarantee of the bas/struktur rungs is covered by the
  no-store tests (phase3/phase4 suites).

### Changed — T4: NO-OP edits are a success, not an error
- `edit` on identical content now returns **Ok with `changed: false`**
  (verdict notice `no_op` still set, history row "NO-OP (identical
  content)" recorded, file untouched) — previously `Err("NO-OP: …")`.
  Harmonizes CLI/library/batch flows with the MCP edit_node
  idempotent-retry path that already shielded its users. Callers that
  branched on the Err must branch on the receipt instead.

### Added — Phase 4 (core): staleness writes + post-commit hook
- **Edits mark the store stale** (decision #8): every successful edit
  through the core funnel appends to `staleness_queue` via
  `code_index_db::mark_stale` — open-existing ONLY (the edit path never
  creates the store), best-effort (never fails the edit), one upserted
  row per file. Doctor's staleness count is now meaningful between
  refreshes; a future daemon/organ polls the same queue.
- **`code-index --install-hook` / `--uninstall-hook`**: installs a
  post-commit hook that incrementally refreshes the store + regenerates
  `code-index.md` after each commit. Idempotent guarded block
  (`>>> gnawtreewriter code-index >>>`), preserves existing user hook
  logic, executable bit set. NEVER auto-stages (design: the regenerated
  file is committed explicitly). P4: personal + explicit — installed
  only by this command, per clone.
- Daemon polling remains ROADMAP add-on territory (not core).

### Added — Phase 3 complete: knowledge graph reads the store; JSON deprecated as writer
- **`RelationalIndexer` is DB-first** (consolidation rule fulfilled — one
  graph home): `load_all_graphs`/`index_directory`/`resolution_stats`/
  `analyze_impact` all serve from `.gnawtreewriter_ai/code_index.db`.
  The legacy parse+JSON pipeline remains as the no-DB fallback
  (capability ladder) and still covers pre-v2 stores.
- **Schema v3** (`user_version = 3`): `entities.node_path` — the DB
  graph now carries dot-paths, so definitions/from_path keep the
  read_node→edit_node chain (T6 freshness caveat applies: valid at the
  indexed hash).
- Mapping: `definitions` ← entities (name→node_path), `from_path` ←
  entity node_path (synthetic callsite from-ids expose their stable gtw
  id instead — honest and fresh-stable), `to_file` ← to_id join,
  `imports` ← last segment of import targets. `RelationType::Call` for
  "calls", everything else Reference (as before).
- FK-hygiene fix: a file's entity replacement nulls out cross-file
  `to_id` references first (no cascade by design — a relation belongs to
  its caller file; the after-loop pass re-resolves), removed files clean
  all three graph tables (rows no longer leak), full rebuild clears the
  graph tables too.
- Dogfood: doctor's knowledge-graph check now reports over the DB graph
  without any JSON files — 722/1786 calls resolved (40 %), 65 ambiguous
  names surfaced.

### Added — T6 resolved: canonical IDs + honest relation targets (Phase 3 foundation)
- **Entity IDs canonical** (`gtw:{canonical_relpath}:{type}:{name}`): paths
  POSIX + git-toplevel-relative regardless of caller input
  (`canonical_entity_path`); impl/trait methods **name-qualified**
  (`Foo::new`) — the old bare `function:new` collided across impls;
  deterministic `#N` disambiguation in line order for remaining
  duplicates. Parse IDs with `splitn(4, ':')` — the name segment may
  contain `::` and `#N`.
- **`Relation.to` → `to_name` + `to_id: Option<String>`** — the honest
  three-class target (review decision #12): `to_name` always set (bare
  identifier or module path); `to_id` only when the target resolves
  uniquely against the indexed entity set (same-file preferred). The old
  doc claimed `to` was always a full entity ID — it never was (imports
  carried module paths, calls bare names): the claim-scan class, caught
  while landing T6.
- **Cross-file call edges now exist**: the per-file "defined" filter hid
  every call to another file's functions; calls are now recorded
  unresolved and the full-index resolution pass resolves them
  (FK-integrity prerequisite for the DB relations table).
- **Store schema v2** (`PRAGMA user_version = 2`): `entities`,
  `relations` (FK on `to_id` only — synthetic callsite from-ids are not
  entity rows) and `symbols` (unresolved/external targets with
  occurrence counts). Filled during `--db` refresh from the same parse —
  no double parsing. MCP `code_index` query gains `kind: items |
  relations | symbols`.
- Dogfood on this repo: 2 464 entities, 4 464 relations (1 130
  resolved), 969 symbols (top: String/Ok/Option — std types).

### BREAKING (lib)
- `Relation` field `to: String` → **`to_name: String` + `to_id:
  Option<String>`** (serialized JSON shape changes; Motor2 consumers of
  `index_relations` output must read `to_name`/`to_id`).
- `extract_function`/`extract_entities_from_tree` signatures changed
  (qualifier param); `find_calls_in_scope` lost its `defined` parameter.
- Entity IDs for impl/trait methods change value (qualification) —
  stored graphs keyed on old IDs are stale by definition.
- Relation calls set grew: previously only same-file-defined or
  path-qualified callees were recorded.

### Added — code-index Phase 2: content-hash gate + MCP tool + doctor
- **`gnawtreewriter code-index <root> --check [--fix]`** — the gate from
  review decision #6, DB-free by construction (P6): fast tier = committed
  body vs its own `> Body-hash:` header (catches hand-edits/partial writes
  without parsing); full tier = fresh direct render vs committed body
  (catches staleness). No rev-string logic. Non-zero exit on failure
  (validate-style); `--fix` rewrites the artifact and reports.
- **MCP tool `code_index`** (`mode: refresh | query | check`): refresh =
  incremental store refresh + markdown/stats; query = item rows from the
  EXISTING store (`path_prefix` filter, never creates — decision #8);
  check = the gate with optional `fix`. `GTW_INSTRUCTIONS.md` updated in
  the same change (contract test parity).
- **doctor**: `code_index` store check — presence, file count, staleness
  queue depth, last-refresh rev vs HEAD (informational; the gate is
  `--check`).
- Markdown header now carries `> Body-hash: <md5>` over the body (the
  part from the first `## ` section) — enables the fast tier. Artifacts
  regenerate cleanly from either path (parity unaffected: both compute
  the same body hash).
- Root-normalization notice moved to **stderr only** — the artifact body
  stays byte-stable so subdir-produced artifacts pass the gate
  identically to toplevel-produced ones (recovery notices remain in the
  body by design: content-relevant, cleared by the next clean render).

### Added — code-index Phase 1: SQLite store (`--db`)
- **`gnawtreewriter code-index <root> --db`** — incremental mode per
  `docs/CODE_INDEX_SQLITE_DESIGN.md` (review 2026-11-02, decisions baked
  in): parse only files whose content hash changed, render markdown/JSON
  from the store. Byte parity with the direct render (modulo stamps +
  root normalization) is a **tested invariant** (`tests/code_index_db.rs`).
- Store: `.gnawtreewriter_ai/code_index.db` (gitignored, WAL) —
  **schema v1** (`meta`, `files`, `items`, `staleness_queue`; migrations
  via `PRAGMA user_version`). Schema changes will be called out here per
  the §36 intake lesson.
- Generation-keyed cache: `schema_version + extractor_version +
  extensions` (population) — presentation options (`max_sig`, `--json`)
  never invalidate. Signatures stored untruncated (cap 512), truncated at
  render only.
- Corruption recovery deletes `db` + `-wal` + `-shm`, rebuilds, and is
  surfaced as a notice in the artifact — never silent.
- `--db` normalizes the index root to git toplevel (one clone, one DB,
  same population as the committed markdown; announced when it differs).
- Concurrency: `busy_timeout=5000` + P5 degrade — on lock-class errors the
  CLI falls back to the direct path with a warning instead of failing.
- `items` rows carry `name` + GTW `node_path` — the consumer's
  revalidation key (valid at the indexed hash only; re-validate via
  read_node before editing).

### BREAKING (lib)
- `core::code_index::extract_items(root, max_sig)` → **`extract_items(root)`**
  — signature truncation moved to render time.
- `IndexItem` gains required fields **`name`** and **`node_path`**
  (struct-literal construction sites must update; serde output gains both,
  additive for readers).
- `generate_index` output: header gains the P3 merge-rule line;
  `--json` items now include `name`/`node_path`.

## [0.21.0] - 2026-10-31

### Fixed — guardian false-positive on body-block additions + MCP force (IV-triagen)
- **Additions inside body-blocks no longer misclassified as removals**:
  the guardian's structural delta used positional child pairing — adding a
  line at the TOP of a large block shifted every pair so the token
  fallback reported "Statement node removed" and **GUARDIAN BLOCK'ed valid
  edits** (Motor2 relay 2026-10-31 (IV), reproduced on a copy of their
  3 831-line lib.rs: the same multibyte addition passed at the block END
  and was blocked at the TOP; pure-ASCII additions were blocked too — not
  an encoding issue). `edit_delta::diff_walk` now matches children by
  (node_type, trimmed content) FIRST (set membership), pairs the
  leftovers by order — genuine removals still classify (regression test
  with a real condition deletion keeps surfacing ConditionRemoved), pure
  additions classify as additions. Tests: `body_block_top_addition_is_not_removal`,
  `body_block_end_addition_still_passes`, `genuine_condition_removal_is_still_detected`,
  `force_path_applies_top_addition` (`src/core/guardian.rs`).
- **MCP `edit_node` gains `force`** (default false): the guardian's
  rejection text says "Use --force to override" but only the CLI could —
  now the MCP agent has the same deliberate last resort; the rejection
  guidance names both routes.
- **`list_nodes` `node_path`-scope** remains OPEN (sub-function, (IV)/C):
  no such parameter exists; silent root-listing documented in the log.

### Changed — semantic channels degrade instead of dying (T5)
`sense`, `search_semantic` (MCP) and `sense` (CLI) no longer answer
"Missing config: .../bge-base/config.json" (or a raw model-init failure) and
leave the agent with nothing: when the embedding model or index is missing
they **degrade to lexical text matches** — node name / first content line
contains query terms, deepest node owns the hit, no ancestor noise — with an
honest label in EVERY reply: "Semantic channel unavailable (reason) —
DEGRADED to lexical text match (NO semantic ranking)", plus the fix path
(`ai setup` downloads models, `ai index` builds the project index).
`GNAW_JSON` mode returns structured degraded JSON (`"degraded": true,
"mode": "lexical"`). `doctor` now reports the semantic-channel state in one
run (`ai_runtime/semantic_model`: pass when bge-base files exist, warn with
guidance when they don't — the 2026-10-23 request). Core: `lexical_degrade`
in `src/core/gnaw_find.rs` (post-order walk, satellite capped at 400 files,
char-safe previews); regression tests ×3 (satellite name hits + container
leak guard, boundary queries, zoom first-line hits); full suite 268 green;
live smoke both directions (degraded temp-project ⚠ doctor / present-model
repo pass).

### Added
- **`undo --preview` i CLI** (T3, från 2026-10-06-entries): se-för-återställ —
  `gnawtreewriter undo --steps 2 --preview` listar de operationer som skulle
  återställas (timestamp, op-slag, fil, nod-sökväg, beskrivning, txn-id) och
  skriver NOLL bytes, via samma `peek_undo`-kärna som MCP:s `undo {preview}`.
  Tom logg svarar ärligt "Nothing to undo" + pekar på `history`. Regressionstest
  `test_undo_preview_writes_nothing` bevisar: preview äter inget från
  undo-stacken (riktig undo funkar därefter) och noll bytes ändras.

### Polished
- **Full fmt + clippy sweep**: `cargo fmt --check` and `cargo clippy --lib
  --tests -- -D warnings` are both clean repo-wide now — the 9.1-era
  test-module debt (token_count, pack, index_entities, state,
  semantic_index, mcp outline) is closed.

## [0.20.1] - 2026-10-31

### Added — `code-index`: project structure index generator (CLI)
New command born from a real consumer need (Motor2 2026-10-29: a fresh
session must answer "where is X?" without reading the codebase):
`gnawtreewriter code-index <root> -o code-index.md --intro <file>` walks
source files (git-aware, via the `ignore` crate), extracts named items
(functions, structs, impl+methods, enums, traits, classes, interfaces) with
line anchors from the tree-sitter tree, and emits a markdown (or `--json`)
index whose header carries git rev + GTW version — staleness is visible
instead of silent. Hand-written prose enters via `<!-- intro: dir -->`
blocks (`--intro`), spliced under matching section headings; unpaired
intros are flagged, never dropped silently. Lenient parsing keeps a
localized syntax error from killing a file's index (warned honestly in the
output). Keyword fallback matches grammar variants across languages but
refuses to match bare keyword TOKEN kinds ("impl", "fn") — proven noise.
5 unit tests; full Motor2 run: 2.2 s, 296 files, 5 024 items.
See `GTW_MCP_ISSUE_LOG.md` (2026-10-29) for the gap analysis that produced it.

### Fixed
- **insert: in-range position kunde landa INUTI ett syskon (T2, triage 2026-10-31)**: grenen för godtyckliga positioner använde `children[idx - 3].end_line` som rå insert-index — för enrads-/token-barn (anonyma `fn`/`identifier`-noder) betydde det MITT i syskonets kropp: live-repro, position 3 på en rot med 5 funktioner nestade den nya fn:en inuti den första (samma klass som Motor2 batt 2026-10-13, men in-range-fallet som Issue 17-fixen inte täckte). Fix: slot `idx - 3` infogar nu vid det adresserade syskonets START-gräns (`start_line - 1`), aldrig inuti det. Regressionstester: `insert_pos3_does_not_nest_inside_first_root_fn`, `insert_in_range_slot_lands_between_siblings` (`tests/insert_position.rs`).
- **quick-insert rapporterade "✓ applied: 0 insertion(s)" utan att skriva (T1, triage 2026-10-31)**: `--after` är ett REGEX, så ankaret `## [Unreleased]` (ob escaped) matchar ingen rad (`[...]` = teckenklass) — och noll träffar rapporterades ändå som ✓-framgång med no-op-transaktion i loggen (fynd #14-klassen). Fix: 0 insertion(s) är nu högljutt fel med regex-ledtråd, inget skrivs, ingen transaktion loggas. Regressionstest: `test_quick_insert_no_match_fails` (`src/cli.rs`).
- **edit_node korrumperade inte längre mid-line-block i JS/TS/Go/Java/++ (2026-10-30)**: 20 tree-sitter-parsers satte aldrig `start_col/end_col`, så `edit_node_at_path` föll i hel-raders-ersättning — ett `statement_block` inuti `.then(function () { ... })` åt hela sin första/sista rad (`.then(`-wrappern och `})`) och bröt promise-kedjan, med tyst "Node edited."-svar. Samma pass: rust.rs byte-kolumner vs splattens tecken-räkning (å/ä/ö-divergens). Fix: `char_col_pair()` i `parser/mod.rs` — tecken-baserade kolumner för alla tree-sitter-parsers; regressionstester i `tests/edit_node_midline_block_regression.rs`.
- **git-status parser hardened against char-boundary panics** (`src/mcp/mod.rs`): the porcelain status line is sliced via `str::get` instead of raw byte ranges — a changed git output format now degrades to a skipped line instead of a panic.
- **Regression test for multibyte edits** (`tests/edit_multibyte_regression.rs`): edit_node on a JS file with å/ä/ö comments and emoji in template literals must succeed — locks the 0.9.8 UTF-8-audit contract onto the edit write path. (The panics seen 2026-10-28 came from a stale 0.18.0 binary predating that fix, not from current source; see `GTW_MCP_ISSUE_LOG.md`.)

## [0.20.0] - 2026-10-21

### Added — Batch `insert_text`: anchored text splices with the same guards
Batch operations gain a fourth op type, born from a real consumer need
(Motor2 §37.4 test-coverage work): 20 identical one-line inserts across
test functions in a ~7 000-line file were previously impossible to do
atomically — node-path inserts (`{"type": "insert"}`) need parent paths
that are impractical to obtain at that file size.

```json
{"type": "insert_text", "file": "a.rs", "anchor": "async fn foo() {",
 "position": "after", "content": "\n    let _guard = ...;"}
```

Semantics: the anchor must occur **exactly once** — 0 matches and 2+
matches are hard errors naming the occurrence count (a silent
wrong-spot edit is the failure mode this op refuses). `content` is
spliced raw (include your own `\n`). The spliced result passes the
same rules guardian + syntax validation as every op, so broken code is
rejected before any write; a failed batch touches nothing. Splice
offsets are match start/end — always char boundaries, so multibyte
anchors (å/ä/ö, emoji) are safe by construction. `insert_text` ops mix
freely with node ops in one batch (applied in spec order). 8 new unit
tests cover after/before splices, not-found, ambiguous, empty-content,
syntax-guard rejection, multibyte safety and batch atomicity.

## [0.19.0] - 2026-10-07

### Changed — Semantic search: three-channel reranking + richer index content
Session arc on the 50-case eval (evals/sense_recall.json): pipeline
recall **60/64/70/72 → 58/68/92% @ k=5/10/100**, recall@1 20 → 30%,
MRR 0.351 → 0.437 — the pipeline now **beats raw cosine at the tail**
(92 vs 90 @ k=100) while keeping the diverse headline agents read.

### Added
- **Graph-proximity channel in the sense reranker**:
  `rerank_satellite_with_graphs` fuses a third reciprocal-rank channel —
  call-graph adjacency of the top seed files (callers + callees,
  BFS-best-seed order) — into the ranking. Hubs (files with >100
  resolved edges, e.g. `core/mod.rs`, `cli.rs`) are excluded as seeds
  and neighbors: adjacency to them is topical noise (measured: w=0.4
  without hub exclusion cost −2 recall@5). Graph JSONs store absolute
  paths while the semantic index stores relative ones; `graph_path_key`
  normalizes both from the first `src/` segment (before this the
  channel was silently inert — proven by w=3.0 changing nothing until
  the fix, then collapsing recall to 2%: adjacency flows). Absent or
  empty graphs degrade silently to the two-channel ranking.
  `ai recall-eval` runs the same pipeline (incl. the graph channel).
- **recall-eval diagnostics**: per-case `expected_cosine_rank`
  (model-side), `expected_pipeline_rank` (full reranked order, not
  capped at k) and `top_pipeline_files` (displacement audit) —
  separates "the model never surfaced it" from "the reranker pushed it
  out". This dataset drove the whole calibration.
- **Headline-zone diversification**: the per-file cap (MAX_PER_FILE=4)
  binds only while filling the first 5 SERVED slots; entries deferred
  there LEAD the tail pass in fused order — nothing inside the served
  window is ever dropped. Eval showed 9/12 misses @k=100 were exact
  symbols crowded out by their own file's better-ranked heads (raw
  ranks 5-90) while the pipeline already named the right FILE.
- **Index preview depth 97 → 240 chars**: the embedded (and stored)
  preview carries 2.5× more source context — real signal instead of
  bare signature heads. Raw cosine recall@1 rose 16% → 24%.
- **File module-doc suffix in embeddings**: each indexed entry appends
  the file's leading `//!` documentation to its embedded text — the
  file's own description bridges agent queries to implementation
  symbols ("avoid reparsing unchanged files" ↔ parse_cache.rs;
  "structural deltas between node trees" ↔ edit_delta.rs). Prefixing
  was measured and rejected: it anchors every entry to the file topic
  and dilutes the symbol signal (recall@5 60 → 46); the suffix lifts
  recall@100 90 → 92% and recall@1 28 → 30%.

### Fixed
- The pre-graph fused sort was lost during a debug-block removal:
  queries with no graph adjacency were served in raw cosine order
  (lex channel + priors silently ignored). Caught by rerank unit
  tests; eval numbers had masked it because the graph path re-sorts.

### Upgrade notes
- **Re-index after upgrading** (`gnawtreewriter ai index`, GPU build
  recommended): preview depth and module-doc suffix change every
  vector. `sense`/MCP signatures are backward compatible (the plain
  `rerank_satellite` wrapper is kept).

## [0.18.0] - 2026-10-06

### Changed — Semantic search: real retrieval embedder (BGE)
- **Embedding backend switched** from the raw ModernBERT masked-LM
  checkpoint (`answerdotai/ModernBERT-base`, mean pooling) to
  **BAAI/bge-base-en-v1.5** (CLS pooling + L2 normalization), because
  the new recall@k harness (`ai recall-eval`, evals/sense_recall.json)
  measured **recall@5 = 0%** on the MLM features — anisotropic space,
  all cosines ~0.9. After the switch (BGE, GPU-indexed, SQLite-stored):
  **recall@1 12.5%, recall@5 54.2%, MRR 0.30**. The legacy path is kept
  as `EmbeddingBackbone::ModernBertMlm` so old model dirs still load.
- **Query/passage split**: `get_query_embedding` prepends the BGE v1.5
  retrieval prefix ("Represent this sentence for searching relevant
  passages:") on the query side only; `get_embedding`/`get_embeddings`
  embed documents raw. `gnaw_sense`, `recall-eval` use the query path;
  `ai index` embeds documents.
- **512-token truncation** on the BGE tokenizer (was unbounded encode
  on ModernBERT); long node bodies are silently truncated at embed time.
- `AiModel::Bge` added; models cached under
  `.gnawtreewriter_ai/models/bge-base/`; `ai setup` downloads it.
  **Re-index after upgrading** — vectors from the MLM model are useless:
  delete the old index and run `gnawtreewriter ai index` again.
- Known limitation: BGE documents are embedded one forward per node on
  CPU (no batching yet) — indexing is noticeably slower than the old
  batched MLM path, but produces embeddings that actually work. GPU
  builds (`scripts/build-gpu.sh` + `indexing.device: auto`) index a
  full src/ tree in ~20 s.

### BREAKING (lib)
- `AiModel` (`src/llm/ai_manager.rs`) gained a variant `Bge` —
  exhaustive matches break; `AiModel::ModernBert` remains as the legacy
  MLM path. `ModernBertModel`'s field `model: ModernBert` was replaced
  by `backbone: EmbeddingBackbone` (new pub enum:
  `ModernBertMlm(ModernBert)` | `Bge(BertModel)`); embed via
  `get_embedding` (documents) / `get_query_embedding` (queries), never
  on the inner model.
- `IntegrityReport` (`src/core/guardian.rs`) gained a field
  `deltas: Vec<EditDelta>` (`#[serde(skip)]`). Struct-literal
  construction of `IntegrityReport` outside the crate breaks — build
  reports via `GuardianEngine::audit_edit[_with_language]` instead.
  (Guardian v2, docs/GUARDIAN_V2_PLAN.md Fas 4.)
- `Batch` (`src/core/batch.rs`) gained fields `last_verdict:
  RefCell<Option<Value>>` and `impacts: RefCell<Vec<Value>>`
  (`#[serde(skip)]`). Struct-literal construction breaks; prefer
  `Batch::new`/`from_json`/`with_file`. (Fas 4.1 batch parity.)

### Added — recall@k harness
- `ai recall-eval <eval-set> [--k N] [--json]`: runs an eval set
  (JSON array of `{query, expect_file, expect_preview?}`) against the
  current index and reports recall@1 / recall@k / MRR plus embed/search
  timings. `evals/sense_recall.json` ships 24 cases as the project
  baseline. Measurement only — the basis for model and storage
  decisions (found the 0% recall of the MLM checkpoint).
  Requires the `modernbert` feature.

### Added — SQLite-backed index storage
- The vector index is now a single `embeddings.db` (SQLite, WAL,
  bundled rusqlite) instead of one pretty-JSON shard per file: ~8×
  smaller on disk (93 MB → 11 MB on this repo) and fast to load
  (raw BLOB reads replace JSON parsing of every shard).
- **Orphan fix**: `save_index` now deletes a file's previous rows in
  the same transaction before inserting — re-indexed files can no
  longer leave stale entries behind (the JSON shards had no delete at
  all, so renamed/moved node sets accumulated forever).
- **Automatic one-time migration**: the first `load_project_index`
  imports legacy `<file_hash>.json` shards + `model_info.json` into
  the DB and renames the sources `*.migrated` (idempotent — never
  re-imports). `model_info` moved to the DB with a legacy-JSON
  fallback read. Public API unchanged; verified A/B: recall identical
  pre/post migration.

### Added — Guardian v2 (docs/GUARDIAN_V2_PLAN.md, Fas 0–4 + 5.1–5.3)
- **Structural deltas (Fas 1)**: `EditDelta` diff of old/new node trees
  detects operator inversion, dropped conditions, lost error handling
  and signature changes; context-aware severity (lone operator change
  stays a warning, contract-broken context escalates to error).
- **Invariant contracts (Fas 2)**: text-level contract of the old node
  (guards, error handling, asserts, unwrap-freedom, doc lines) must not
  be lost entirely — catches agent-style full regenerations.
- **No-op guard + EditReceipt (Fas 3)**: identical-content edits
  rejected loudly; post-write disk verification; `verified` +
  `bytes_changed` in MCP edit/insert responses and CLI output.
- **Rule fixes + healer hints in rejections (5.2)**: blocks name
  `[rule_id:line]` and the rule's `fix:` suggestion; healer failures
  named explicitly.
- **Structured `edit_verdict` (5.1+5.3)**: all four rejection paths
  record `{level, score, findings, suggestions}`; MCP
  `edit_node`/`insert_node` (and `semantic_edit` via delegation) attach
  it to `E_EDIT_REJECTED` responses. Contract test included.
- **Impact report (Fas 4)**: signature-changing edits carry
  `impact: {symbol, callers, sites}` in MCP responses and a `📊 Impact`
  CLI line, sourced from the knowledge graph
  (`.gnawtreewriter_ai/graph/`). Missing index ⇒ field omitted, never
  an error. `ImpactAnalyzer::load_all_graphs` implemented (was a
  placeholder) and `impact_analyzer` registered in `llm` (was
  unregistered).
- **Limitation fixes (Fas 4.1)**: honest `to_file` resolution in the
  relational index (Some(file) only when unambiguous — no more
  first-match guess); `analyze_impact` uses its `defined_in` parameter
  to exclude callers of same-name symbols defined elsewhere; batch
  runs the full single-edit validation (NO-OP guard, Guardian, rules)
  before any write and carries the same `edit_verdict` on
  `E_BATCH_ROLLED_BACK` plus `impacts` on successful applies.
- **Resolution observability + same-file rule (Fas 4.2, ROADMAP
  "Symbol resolution toward LSP-grade" steps 1–2)**:
  `RelationalIndexer::resolution_stats()` reports unique vs ambiguous
  symbol names, call-resolution rate and top-N ambiguous names;
  `doctor` gained a `knowledge_graph.resolution` check; impact
  responses carry `unresolved: N` (MCP) and the CLI prints
  `📊 Impact … (N unresolved)` so caller counts can be trusted
  appropriately. A definition in the current file now wins for
  bare-name calls (same-module precedence), resolving most same-name
  ambiguity without path parsing.
- **Import-aware call narrowing (Fas 4.2 step 3)**: `FileGraph` now
  stores the identifier tokens of each file's use/import statements
  (`#[serde(default)]` — old graph JSON stays loadable). When a bare
  call's name is defined in several files, exactly one candidate whose
  file stem appears in the caller's import tokens resolves the site
  (`use crate::utils::parse`, `from utils import parse`,
  `import { parse } from "./utils"`); zero or several stem matches
  stay honestly ambiguous.
  **BREAKING (lib)**: `FileGraph` gained a field — struct literals
  must add `imports` (or `..Default::default()`).

## [0.17.0] - 2026-10-05

### Added — Phase 10: Agent Confidence & Motivation (trygghet + maning)
- **`validate <file>` (CLI + MCP `{file_path}`)**: strict pre-edit syntax
  gate — node count on success; on failure `E_STRICT_PARSE` + the error
  position + pointer to the partial read paths, nonzero exit on the CLI.
  (AGENTS.md advertised this command in three places while it did not
  exist — following our own docs gave "unrecognized subcommand".)
  Shared core: `parser::validate_file`.
- **MCP `diff`**: independent post-write verification — `{old_file,
  new_file}` (text/json) or `{file_path, old_path, node_path}`, same core
  as CLI `gnaw-diff`, reads only.
- **`undo {preview: true}`**: lists what WOULD be reverted
  (id/file/operation/description, newest first) with zero mutation —
  `UndoRedoManager::peek_undo`; e2e-verified by history counts before and
  after.
- **Write receipts**: `transaction_id` on edit_node/insert/semantic_insert
  responses, `transaction_ids [src, tgt]` on move, one id per written file
  on batch apply — correlates with `history`/`undo` (`RefCell`,
  source-compatible for lib consumers).
- **Idempotent retries**: `edit_node` whose target already equals the
  requested content answers `already_applied: true` (trimmed compare, zero
  bytes written) instead of a scary `E_EDIT_REJECTED` — closes the
  historical timeout → retry → rejection spiral.
- **MCP `guide {situation?}`**: machine-readable mirror of the skill
  situations table (21 when/tool/example rows) for hosts that never load
  SKILL.md (Motor2); token-match filtering, no-match fails loudly;
  test pins every referenced tool to the live registry.
- **Cold-start notes** in the five model-tool descriptions ("first call
  may take 20-30 s — raise the host timeout") — untold latencies were the
  source of false "Not connected" exits.

### BREAKING (lib) — policy established
- Release checklist (AGENTS.md) now requires a `### BREAKING (lib)`
  section before release for enum/struct field additions and new
  required params; the retroactive 0.16.0 note (SenseResponse::quality)
  stands as the first entry. Integrators that path-depend (Motor2) rely
  on being warned.

### Fixed
- **`index_relations`: "calls" relations never carry an empty `from`**
  (`a655cd2`, Motor2 dashboard report): call sites inside impl blocks
  resolved to `gtw:{file}:function:{enclosing}`; scope-less calls get the
  synthetic `gtw:{file}:callsite:{line}`. Verified against Motor2's
  exact report file: 91 relations, 0 empty — their "(intern)" UI patch
  can go when they bump the path-dep. Rustdoc documents the contract;
  invariants tested on fixture and on a real 3000-line file.
- **`--no-default-features` builds** (Motor2 bug report, fixed in
  `18962b2`): `sense_with` lacked a feature gate while its body uses
  modernbert-gated code — 8 compile errors for path-dependents like
  motor2-gtw. Now gated with an honest not-modernbert stub; regression
  threshold added: `validate.yml` runs
  `cargo check --no-default-features --all-targets` (default AND mamba
  both carry modernbert, only this config catches gate holes).
  `examples/debug_loading.rs` got `required-features` (pre-existing
  candle hole).

### Docs
- **Doc-drift pass**: GTW_INSTRUCTIONS.md said "30 tools" while 34 were
  registered (doctor/history/stats/index_project had landed without
  updating it) — now complete, and `integration_mcp_instructions_listed`
  pins EVERY registered tool name to the file so the drift cannot
  recur. SKILL.md gained situations-rows/MCP-ref entries for
  index_project/history/stats plus `expand`/`search_quality` guidance
  and the `E_STRICT_PARSE` row; README lists updated (incl. a
  nonexistent `diff_to_batch` MCP tool claim corrected to CLI-only
  `diff-to-batch`); v0.16.0 notes test count corrected 232 → 233.

## [0.16.0] - 2026-10-05

### Added
- **Query expansion for satellite `sense`** (`expand: true`, opt-in):
  LFM2.5 expands the question into content terms (same prompt as
  investigate step 1); the expanded text runs as a SECOND semantic
  channel fused with the original by max cosine
  (`fuse_by_max`), and feeds the reranker's lexical side. Costs one LLM
  call + one extra embedding — default false. Non-mamba builds respond
  with an explicit note instead of failing. Live-verified in a
  release+mamba build (7.4 s end-to-end, terms like
  `['transaction log', 'persistence', 'entry storage', …]`).
- **Search feedback loop** (roadmap AUTO-koppling):
  - `search_quality {prior_failures, suspect_reason, expansion}` on
    every satellite answer; suspect reasons: `empty`, `low_cosine`
    (top raw cosine < 0.5), `no_lex_overlap` (top hit shares no query
    word — catches the flat-cosine saturation where unrelated code
    scores 0.85+).
  - Failures append to `.gnawtreewriter_search_log.jsonl`
    (gitignored, rotated at 1 MB → newest 500 lines); `prior_failures`
    counts earlier failures of the SAME normalized query and is shown
    in both the success note and the no-matches error — an agent sees
    "this query keeps failing" instead of trusting a flat field.
  - Live-verified: nonsense query flagged `no_lex_overlap`, second call
    showed `prior_failures: 1`, JSONL entries well-formed.

### Fixed
### BREAKING (lib)
- **`SenseResponse::Satelite` gained the `quality` field** (search feedback loop): downstream code that pattern-matches the variant with an exact field list must add `quality: _` (or `..`). MCP clients are unaffected (wire format only gained optional fields). Policy from now on: every lib-API breaking change gets a `### BREAKING (lib)` section here BEFORE release — integrators like Motor2 path-dep GTW and rely on being warned (see the AGENTS.md release checklist).

### Fixed
- `edit --ask "…" --all --preview` ignored the preview flag (writes
  were gated only by `!force`) — preview now always wins, even
  combined with `--force`; also clears the mamba-only unused-parameter
  warning.

### Docs
- `sense` schema: `expand` property + description documents
  `search_quality` semantics; `.gitignore` covers the search log.

## [0.15.0] - 2026-10-05

### Added
- **MCP `index_project` tool** (Motor2 bug 2): builds the SEMANTIC/vector
  index satellite `sense` searches — the MCP side of `ai index`, same
  pipeline, same GPU/20%-VRAM gate. Background runs (`{action: start}` /
  `status`) so calls never outrun client timeouts; single-run guard via
  compare_exchange. `index_entities`/`index_relations` descriptions now
  state they build the knowledge graph, NOT this index; the satellite
  no-matches error points at the tool first.
- **MCP `history` + `stats` tools** (Motor2 plan #24): transaction log
  newest-first with stable ids (empty log = normal state with setup
  guidance) and full ProjectStats with a one-line summary.
- **Semantic edit transparency** (Motor2 plan #19): `semantic_edit` hit
  returns `semantic_match {matched_path, confidence, candidates[5]}` on
  the result AND in the text channel; a miss attaches explicit zero
  confidence + empty candidates instead of a bare denial.
  `semantic_insert` success adds structured
  `{anchor_path, confidence, parent_path, position}`.
- **Stable error codes** (Motor2 plan #25): `tool_error_code` puts
  `code` next to the human text — `E_FILE_NOT_FOUND`, `E_STRICT_PARSE`,
  `E_NODE_NOT_FOUND`, `E_EDIT_REJECTED`, `E_VALIDATION`,
  `E_SEMANTIC_NO_MATCH`, `E_MODEL_UNAVAILABLE`, `E_RULE_REJECTED`,
  `E_BATCH_ROLLED_BACK` (~40 call sites). Registry:
  `docs/ERROR_CODES.md`, kept in sync mechanically by
  `integration_error_codes_documented` (both directions).
- **Duplex metrics** (Motor2 plan #23): `proposed`/`validated`/
  `rejected`/`applied` counters persisted at
  `.gnawtreewriter_metrics.json` from edit_node/insert/edit_ask/
  semantic_insert outcomes (move/batch keep their own transactions —
  documented scope).
- **Retry-with-feedback** (Motor2 plan #23): `edit_ask` feeds the AST
  validation error back to the model for ONE repair round; success is
  flagged `retried: true` with `first_error`, double failure reports
  both attempts.
- **Partial-grace parsing for READ paths** (Motor2 brief): new
  `ParserEngine::parse_lenient` (default = strict; Rust overrides via
  tree-sitter's error-tolerant tree) + `GnawTreeWriter::new_lenient`.
  `analyze`/`get_skeleton`/`list_nodes`/`search_nodes`/`read_node`
  now answer broken-but-largely-valid files with the partial tree and
  an explicit `syntax_warning` (text + structured), while EDITORS stay
  strict — same file gets `E_STRICT_PARSE` with a pointer to the read
  paths, zero bytes written. Contract test:
  `integration_mcp_partial_parse_read_paths`.

### Fixed
- Strict parse refusals were coded `E_FILE_NOT_FOUND` (misleading
  aggregation — syntax errors would count as missing files); split via
  `open_error` into `E_STRICT_PARSE` vs `E_FILE_NOT_FOUND`.
- `preview_edit` validation failure carried a bogus "sense failed"
  guidance — now `E_VALIDATION` with preview-specific next steps.
- `test_debug_mcp_relations` sanity cap made size-relative (fixed 200
  broke as mcp/mod.rs grew with real features).

### Docs
- `docs/ERROR_CODES.md` (new): registry table with meanings and next
  steps; sync-tested against `tool_error_code` call sites.

## [0.14.0] - 2026-10-05

### Added
- **Satellite sense ranking quality**: `rerank_satellite` in front of raw
  cosine. Declarations (module/use/attribute heads) are demoted with a
  penalty scaled by query-term overlap, implementations get a nudge, and
  lexical matches count across preview + file/node paths; `(Chunk N)`
  prefixes are classified on the real code head. Raw window widened
  (2000 @ floor 0.1) so long, diffuse implementations are even *visible*
  to the reranker, then trimmed to 10. Inventory-intent queries ("which
  modules exist…") flip the priors: declarations ARE the answer and get
  the item bonus instead of the penalty. Result on the reference query
  "how is undo implemented…": real `impl`/`fn` nodes on top — before,
  all ten hits were module declarations and `#[command]` attributes.
- **Batched embeddings** (`ModernBertModel::get_embeddings`): one
  forward per window of up to 16 texts with padded masked mean-pooling,
  order-preserving, single row exactly equivalent to `get_embedding`.
  Wired into the project indexer (two-phase: walk collects
  (path, preview, text) triples, then one batched call per file) and
  both zoom-JIT node loops. Forward passes go from N to ~N/16.

### Fixed
- **CUDA OOM during GPU indexing**: a flat padded batch of long
  definition bodies (≤8000 chars each) blew VRAM at B=16. Batching is
  now budget-aware — greedy consecutive windows keep `B × max_len² ≤
  2^22`, so short rows (the common case) stay batched while giant rows
  degenerate to B=1, the sequential path that already ran fine on GPU.
  Full reindex after the fix: **223 files in 90 s, exit 0** (RTX 4070
  SUPER, 20%-VRAM gate passed at 28% free).

### Changed
- `#[ignore]` on the heavy ModernBERT equivalence test (debug-mode
  forwards ~2-3 min): run explicitly with
  `cargo test --test ai_modernbert_tests -- --ignored --nocapture`
  when touching `get_embeddings` — it also prints sequential-vs-batch
  timing. Short-text debug micro-bench shows 1.1-1.2× (debug overhead
  dominates); the structural win is fewer forwards + launch reduction
  + long-row OOM safety.

### Docs
- Issue log: reply to the Motor2 2026-10-05 entry (root cause of bug 1:
  tree-sitter-rust 0.24.0 treated plain `&raw` borrows as Rust 1.82
  `&raw const/mut` starts — fixed by 0.24.2, committed in 6413302;
  bug 2 confirmed as two-separate-indexes gap, MCP index tool pending).

## [0.13.0] - 2026-10-04

### Added
- **SKILL.md rewritten for agents** (ROADMAP 9.6): situations-table (16 rows —
  situation → MCP tool → complete example call), the fallback rule (one timeout
  → one detour → log to `GTW_MCP_ISSUE_LOG.md` → verify bytes on suspicious
  success, never `git checkout` around GTW), per-tool example calls for the
  core MCP set, and `doctor`-based status instead of a hardcoded version claim.
- **AGENTS.md "För agenter som ANVÄNDER GTW" section**: six-step diagnostic
  chain, the issue-log note taxonomy (7 categories — every closed entry
  corresponds to a real fix), and the escalation rule.
- **Error-guidance contract test** `integration_error_strings_carry_guidance`:
  bans raw `e.to_string()` passthrough, bare feature-gate strings, unguided
  IO errors, and the historic stub lies from live (non-comment) code — the
  "no raw error strings" rule can no longer silently regress.

### Changed
- **Every `tool_error` in the MCP server now carries next-step guidance**
  (~30 sites): IO errors point at `search_nodes`/`analyze`, feature gates at
  the rebuild command, model failures at `ai setup`/`ai status` with a
  search_nodes/grep fallback, edit rejections at `list_nodes`+`preview_edit`,
  rule failures at `rules list`. Zero raw passthroughs remain.

### Fixed
- SKILL.md contained the forbidden batch format (`search`/`replace` instead of
  `{file, path, content}` — finding #13's spec lie), `gnawtreewolf` typos (×2),
  and a stale hardcoded version claim.

### Docs
- ROADMAP 9.6 complete. Roadmap Phase 9 fully done (9.1–9.6).

## [0.12.0] - 2026-10-04

### Added
- **MCP `doctor` tool** (ROADMAP 9.5): one-call health diagnostic — parser
  smokes, backup integrity, transaction log → `{healthy, passed, failed,
  warnings, total, checks[]}` with guided text. Shares `run_full_doctor`
  with the CLI (`gnawtreewriter doctor`) so the two can never diverge
  (same pattern as `run_lint`).
- **Lint rule `rust_string_byte_slice`** (`$X[..$Y]` + fix template
  `$X.get(..$Y).unwrap_or($X)`) — the byte-slice class left over from the
  0.9.8 UTF-8 audit, smoke-proven against real hits in `xml.rs`/`qml.rs`
  and through `lint --fix --preview`.
- **`$X`-binding interpolation in rule messages** (`interpolate_message`):
  findings read "… if `code` is a String/&str" instead of the raw template;
  unbound names stay literal (messages never fail).

### Fixed
- **Finding #14: `quick-replace` could report "✓ applied" while changing
  zero bytes** — the exact lie the tool must never tell. A replace now
  fails loudly when the search text is not found (or the replacement is
  identical), with next-step guidance ("re-read the file after cargo fmt
  and retry"). No bytes are written on a no-match; preview fails the same
  way. Two regression tests.
- **`get_skeleton` could answer with a bare header** (issue-log triage):
  the skeleton now rides in `content.text`, `truncated` is an explicit
  flag at the 500-node cap, and an empty skeleton is a guided error —
  never a silent empty success. Contract test added.
- **`sense` zoom/satellite returned header-only text** (issue-log FIX
  prescription): top matches now appear in the text channel
  (`"Zoom …: 5 node(s) — 106 (0.81) …"`); empty zoom returns a guided
  error instead of a naked label.
- **Unknown `--rule` id passed silently as "No issues"** — now fails
  loudly with the loaded-rule count and pointers (`rules list`,
  `rules add`).
- **`doctor` took 80 s in backup-heavy projects**: `check_backups` now
  light-scans (count by filename, validate the 3 newest by mtime) instead
  of fully parsing every backup JSON (2 GB/128 files here); parser smokes
  run in parallel. Doctor: 80 s → 0.5 s.

### Changed
- CLI `doctor` section headers collapsed into one line (checks unchanged);
  internals delegated to the shared `run_full_doctor`.

### Docs
- ROADMAP 9.5 complete (all items, with implementation notes).
- `GTW_MCP_ISSUE_LOG.md`: all three 2026-09-30/10-01 triage checklists
  closed with verdicts; zoom/satellite text-channel FIX marked done;
  open remainder documented (satellite index-build lacks an MCP tool —
  known gap, outside 9.5 scope).

## [0.11.0] - 2026-10-04

### Added
- **GPU-accelerated indexing — opt-in, conservative by default**: GnawTreeWriter
  never touches the GPU unless told to (the tool was designed CPU-only and stays
  that way for users). Two ways to opt in:
  - persistent: `indexing.device: auto` in the new optional `gnawtreewriter.yaml`
    (values `auto`|`cpu`|`cuda`; absent file or setting = CPU)
  - one-off: `gnawtreewriter ai index --gpu`
- **20%-VRAM safety gate** (`decide_index_device` in ai_manager): `auto` uses the
  GPU only when the binary was built with `--features cuda` **and** ≥20% of VRAM
  is free — never squeezes the OS or co-resident GPU users (verified live against
  a co-resident llama-server). Unknown/invalid config values fail safe to CPU.
  Every decision is logged with its reason (guided principle: never silently
  degrade). Query paths (sense/zoom/MCP) always stay CPU by design.
- **`scripts/build-gpu.sh`** — distribution-friendly GPU build: compiles
  `--features cuda` inside an `nvidia/cuda` container (host needs only podman +
  NVIDIA driver — **no CUDA toolkit**, which Fedora does not ship), auto-detects
  compute capability from the host's `nvidia-smi`, ships exactly the CUDA
  runtime libs the host lacks (driver libs correctly left to the host), and
  installs a wrapper + binary. One command, idempotent, cached volumes for
  rebuilds (~2 min warm).
- **Docs**: AGENTS.md "GPU indexing (opt-in; devs recommended)" section — the
  recommendation for developer machines is to change the one flag in the file;
  README "GPU-accelerated indexing (optional, off by default)" section with the
  same three-line setup.

### Fixed
- **Finding #14 (GTW tool bug, logged in GTW_MCP_ISSUE_LOG.md)**: `quick-replace`
  reported "✓ applied" while changing zero bytes (oldString didn't match
  post-`cargo fmt` text) — caught by failing tests. A replace must change bytes
  or fail loudly with guidance ("search text not found — re-read the file after
  cargo fmt and retry"), never fake success.

### Docs
- `GTW_MCP_ISSUE_LOG.md`: GPU-entry + finding #14.

## [0.10.0] - 2026-10-04

### Added
- **MCP `lint` tool** (ROADMAP 9.1): AST pattern linting through MCP with full schema — paths, `recursive`, severity/rule filters, and ad-hoc `$X` pattern search (`pattern` + `language`). Shared core `rules::run_lint` with the CLI so the two never diverge; report-only, never writes. CLI gained parity flags `--pattern`/`--language`.
- **`fix:` rule support** (ROADMAP 9.2): rules may carry a rewrite template (`fix: "$X.expect(\"msg\")"` in YAML, `--fix` on `rules add`, `fix` arg on MCP `add_rule`). Templates expand with `$X` bindings from the match, are validated before saving (`validate_fix`: identifier substitution, scaffold fallback `fn probe() { … }` for expression forms like `?`-chains — unknown languages fall through to the generic parser, matching `compile_rule`), and apply through `lint --fix` (CLI) / `lint {fix: true}` (MCP) as an **atomic batch** — Edit-op per finding, node content replaced in place so paths don't shift, dedup per file+path. `--fix --preview` shows the diff and writes nothing. Writes only ever happen behind the explicit flag; `LintResult.rules` carries the effective rule set. `py_bare_except` ships with a builtin fix.
- **MCP adoption contract** (ROADMAP 9.3): all 30 tools now carry self-teaching descriptions (VAD + NÄR vs Read/Grep/Edit + RETURNERAR + EXEMPEL, ≥100 chars) and complete `inputSchema`s. Schema lies fixed against the actual dispatch: `list_nodes` exposes `filter`/`max_depth`, `explain`/`edit_ask`/`investigate` gained `required`, `index_entities`/`index_relations` gained `anyOf` (file_path | file_paths), `save_state` is honestly zero-arg. New `integration_mcp_tools_adoption_contract` test mechanically pins the catalog: unique names, description length, `required ⊆ properties`, honest zero-arg, ≥30 tools, priority tools (`batch`, `edit_node`, `insert_node`, `search_nodes`, `sense`, `semantic_edit`) with real schemas — documentation drift can no longer regress silently.
- **Source citations (`sources`) in LLM answers** (ROADMAP 9.4): `get_semantic_report` returns `sources: [{file, node_path}]` (one provenance pointer per finding, deduped); `investigate` returns `sources: [{file}]` for the evidence the answer was actually synthesized from. Pure payload builders (`semantic_report_payload`, `investigate_payload`) make the contract testable without a model — `integration_sources_semantic_report_payload` + `integration_sources_investigate_payload` (mamba) plus 6 unit tests.
- **`FileMatch.content_preview`**: satellite `sense`/`search_semantic` matches now include a `preview` straight from the index — score + content in one response saves an extra `read_node` round-trip.

### Fixed
- **"Resurrection" mystery root-caused** (GTW_MCP_ISSUE_LOG.md finding 12): the repeated silent reversion of recent edits (8 victims across `rules.rs`/`cli.rs`) was never a cache bug — `integration_mcp_tools_call_undo` called the MCP `undo` tool against the **real repo transaction log** on every `cargo test` (the server takes `project_root` from CWD), replaying the latest GTW transactions onto source files. New `serve_with_shutdown_root(listener, token, project_root, shutdown)` lets tests bind an explicit project root; the undo test now runs against a throwaway temp project and asserts the deterministic "Nothing to undo". Regression-proofed: two consecutive full-suite runs with unchanged file checksums.
- **Batch test used a nonexistent spec format** (finding 13): `integration_mcp_tools_call_batch` wrote `replace` instead of the real `BatchFile`/`BatchOp` format (`{file, path, content}`) and could never pass against a real parser — masked by the undo-test chaos. Fixed with root-node replacement.
- **`--features mamba` did not compile**: a mamba-gated `Rule` initializer in `cli.rs` predated the 9.2 `fix` field. Now parses `fix` from the rule source (parity with `rules add`).
- MCP handler signature/docs now match dispatch for `explain` (requires `file_path`), `edit_ask` (requires `file_path` + `request`), `investigate` (requires `question`).

### Changed
- `undo` MCP tool documents the transaction-log contract and points at `gnawtreewriter history` / `restore-project` for failed reverts; `batch` tool description covers atomicity + preview semantics.

### Docs
- `GTW_INSTRUCTIONS.md` regenerated: all 30 tools in a category table + the diagnostic chain (`analyze`/`get_skeleton` → `sense`/`search_nodes` → `read_node` → `edit_node`/`semantic_edit` → verify → `undo`) — the previous version had drifted to 17 tools.
- `GTW_MCP_ISSUE_LOG.md`: 9.1-continuation entry (findings 1–8), 9.2 entry (findings 9–11), 9.3 entry (findings 12–13, root-cause reclassification), 9.4 live-verification entry.
- `ROADMAP.md`: Phase 9 items 9.1–9.4 marked complete with implementation notes.
- `README.md`: Rules Engine documents `fix:` templates and `lint --fix [--preview]`; MCP tool lists include `lint`.

## [0.9.8] - 2026-10-01

### Fixed
- **MCP reliability on large files** (found via GTW_MCP_ISSUE_LOG.md omgång 3-5): `sense`/`get_skeleton` on 1000+ node files completed within client timeouts instead of cascading into timeouts and a dead "Not connected" connection.
- **Per-request panic isolation** in both stdio and HTTP MCP loops: a panicking handler now returns a JSON-RPC internal error instead of unwinding through the read loop and killing the connection.
- **Shared GnawSense broker** (`AppState` + `OnceCell`): the ModernBERT model and JIT index cache now persist across MCP calls instead of reloading per call. First zoom-sense ~25-30 s, following calls ~1 s (cache hit).
- **Char-boundary panics on multibyte UTF-8** (em-dash, CJK, emoji) — byte slicing (`&s[..97]`, `[..3000]`, byte-based `chunk_text`) replaced with char-safe truncation across sense previews, pipeline evidence caps, project indexer chunking, `edit` removed-preview and secrets redaction. Systematic UTF-8 audit; a `no_byte_slice_strings` lint rule was added to `gnawtreewriter.rules.yaml`.
- **ModernBERT rope crash on huge nodes** (content > 8192 tokens crashed the model with "inconsistent last dim size in rope") — node content truncated before embedding.
- **Degenerate `Satelite search results` response** — now reports match count and hints at building the project index when empty.
- **Bounded JIT cache** (32 files with eviction) — unbounded growth reached 511 MB RSS in long-lived MCP sessions.
- clippy `question_mark` warnings in `parser/xml.rs`.

### Changed
- Zoom indexing caps for first-call latency: max 24 embedded nodes per file (largest definitions first), max 1200 chars per node. Zoom remains a localization tool; the full project index is built with `ai index`.
- `project_indexer` chunk thresholds made rope-safe (chunk at 4000 chars from 8000, was 10000 from 15000 — CJK can cost ~1 token/char).

### Docs
- `GTW_MCP_ISSUE_LOG.md`: omgång 3 (reproduction: timeout → Not connected, degenerate satellite response), omgång 4 (root causes + fixes + validation), omgång 5 (UTF-8 audit: 7 more byte-slice bugs found and fixed).

## [0.9.7] - 2026-08-25

### Added
- **Rules engine (`lint --rules`)**: semgrep-like pattern matching against ASTs (spec: docs/RULES_ENGINE_SPEC.md).
  - YAML rules with `$X` placeholders, compiled to structural AST matching in Rust
  - Builtin rules (rust_unwrap, rust_self_assignment, py_bare_except, py_eval, js_console_log)
  - Project rules auto-loaded from `gnawtreewriter.rules.yaml`
  - `--rules <file>`, `--severity`, `--rule <id>` filters, JSON output
  - `lint` is now real lint (structural rules), not just a parse check
- **Rules guardian on edit (Duplex Loop 2.0)**: after validation, builtin rules run on the new code — error-severity findings block the edit (unless `--force`), warnings are printed but allowed. From "does it parse?" to "is it good code?".
- **Rule annotations in `edit --ask` prompts**: known rule violations in the file are injected into the LLM prompt so the model can avoid introducing/worsening them (rule-injected expertise). `edit --ask` switched to a line-based proposal format (`{"line": N, "new": "..."}`) — far more reliable for a small model than JSON-escaping whole code snippets.
- **Agent-written rules (`rules add` + MCP `add_rule`)**: validate and append a semgrep-like rule to `gnawtreewriter.rules.yaml`. The pattern must compile for the language; invalid rules are rejected. Agents can now write rules to GTW through a tool.
- **`lint --discover`**: the local LFM2.5 model proposes project-specific rules from linted files; each proposal is validated (must compile and match ≥1 file) before saving. Few-shot examples in the prompt make the model produce valid patterns.
- **Multi-edit (`edit --ask "..." --all`)**: the model proposes a change for one occurrence; GTW applies the same replacement to every identical line/occurrence in the file, validating the whole file reparses before writing. Step 5: rule-guided consistent edits.
- **Docs**: README "Rules Engine" section, ROADMAP Phase 8 updated (all 5 rule steps + LLM missions marked done), RULES_ENGINE_SPEC.md marked implemented.
- **Rule library expanded**: 19 builtin rules across rust (7), python (4), javascript (4), typescript (2) — original formulations of well-known anti-patterns (not copied from semgrep-rules; the license forbids redistributing them). Written with `$X` placeholders for the structural matcher.
- **30 more rules added** across go (9), java (10), c (9): self-assignment, error handling, memory safety, type safety, and concurrency anti-patterns. Builtin library now totals 46 rules across 7 languages.
- **40+ more rules added** across all 7 languages, bringing the builtin library to ~70 rules (10 per language): raise/assert/imports for Python, alert/prompt/setTimeout/XHR/addEventListener for JS, catch unknown/undefined check/type assertion for TS, collect/magic number/slice clone for Rust, make/for-loop/defer for Go, Object/parseInt/synchronized for Java, malloc/free/sprintf/goto for C.
- **Duplicate-finding fix**: `run_rule` now deduplicates findings by code location `(line, column)`, so a statement and its inner child matching the same pattern are reported once, not several times. Verified: bare `except: pass` reports once; separate `unwrap()` calls keep their own findings.
- **Local LLM command extension (LFM2.5-1.2B, Q4)** behind the `mamba` feature:
  - `explain <file> [--node <path>]` — plain-language explanation of a code node
  - `summarize <dir>` — hierarchical AST-skeleton map-reduce summary
  - `investigate "question"` — query expansion → index search → ranked answer with file references
  - All three available as MCP tools (`explain`, `summarize`, `investigate`)
  - `ai calibrate` — measures this machine's inference speed and saves a timing profile
- **`--resolution` flag** (fast / balanced / thorough) on explain/summarize/investigate: trades speed vs detail via chunk size and output budgets.
- **Token & time transparency**: every command reports a budget (`expected/actual tokens`, `calls`, `estimated/actual seconds`, truncation warning). No silent cut-offs.
- **`edit --ask "request"`** (+ MCP `edit_ask`): the local model proposes a minimal old→new change; GTW finds the containing AST node, applies the replacement inside it, and validates through the Duplex Loop before apply. The AST places, the model states intent — a small model stays viable because it never needs mechanical precision.
- **ROADMAP.md**: updated to current status (v0.9.7) with a new Phase 8 (Local LLM Command Extension) section and the planned LLM missions.

### Performance notes
- LFM2.5 prefill in candle is ~O(seq²), so pipeline steps chunk to small sizes and feed compact AST skeletons instead of raw source (~98% smaller) — summarize went from >3 min/file to ~15s/file.

## [0.9.6] - 2026-08-24

### Added
- **`explore` command**: Map-like navigation with 4 zoom levels (overview / directory / file / full), each node carrying token counts and drill-down hints.
- **Session-level parse cache** (`parse_cache.rs`): Files parsed once are reused across tool calls in the same session — no redundant AST parses.
- **`pack --compress-threshold N`**: Compress only files larger than N tokens; 0 (default) compresses everything.
- **`scripts/benchmark_ai.sh`**: Reproducible benchmark for explore/pack/index/state operations.

### Changed
- **Parallel processing (rayon)**: pack, explore (directory level), and MCP index batch handlers now run across all CPU cores. Output order is preserved — results remain byte-identical to the sequential versions.

### Performance (measured on own codebase, 79 files / 169k tokens)
| Operation | Before | After |
|---|---|---|
| pack --compress | 676 ms | 335 ms (-50%) |
| explore directory (level 1) | 318 ms | 130 ms (-59%) |
| index_entities (20 files) | 177 ms | 107 ms (-40%) |
| index_relations (20 files) | 84 ms | 40 ms (-52%) |

## [0.9.5] - 2026-08-23

### Added
- **AI-Friendly Context Tools** (inspired by Repomix analysis):
  - `compress` command: Replace function bodies with `⋮----` placeholders (~70% token reduction)
  - `pack` command: Package entire project into AI-optimized format (markdown/json/plain)
  - `curate` command: Intelligent file selection based on task description
  - Token counting in `analyze` output (estimated_tokens field)
  - Secret detection and redaction (18 patterns: AWS, GitHub, GitLab, Stripe, JWT, etc.)
  - Git-aware file walking (respects .gitignore, replaces hardcoded skip-lists)

- **New MCP tools**: `compress`, `pack`, `curate`

- **New modules**:
  - `file_walker.rs`: Git-aware traversal using `ignore` crate
  - `token_count.rs`: Heuristic token estimation for LLM context planning
  - `compress.rs`: AST-based code compression
  - `pack.rs`: Project packaging with token budgets
  - `secrets.rs`: Credential detection and redaction
  - `curator.rs`: Multi-strategy context curation (relevance, git changes, dependencies)

### Changed
- Replaced hardcoded skip-lists in `gnaw_find`, `blast`, `inspect`, `gnaw_refactor`, `project_indexer`, `relational_index` with unified git-aware walker

## [0.9.4] - 2026-04-30

### Added
- **`multi-replace` command**: Multiple search/replace pairs in one pass
  - Supports STDIN (`--pairs -`), inline JSON, or file path
  - Single file read, single backup, single transaction
  - Atomic: all-or-nothing validation
  - Auto-unescape of `\n`/`\t` in replacement text
  - Example: `echo '[{"search":"a","replace":"b"}]' | gtw multi-replace file.rs --pairs -`

- **Batch STDIN support**: `batch -` reads JSON from STDIN
  - No temp file needed for pipelining
  - Example: `cat ops.json | gtw batch - --preview`

### Documentation
- **SKILL.md**: Added Quick STDIN Reference table at top
- **SKILL.md**: Added "Agent Workflows" section with 10 patterns
- **GTW_AGENT_COOKBOOK.md**: New 287-line cookbook with 10 recipes for AI agents

## [0.9.3] - 2026-04-27

### Added
- **GnawSense Semantic Navigation** (AI-powered code search & insertion):
  - `sense` command: search code by meaning using local ModernBERT model
  - `sense-insert` command: insert code at semantically located anchors
  - All 4 intents supported: `after`, `before`, `inside`, `replace`
  - `--auto-index` flag: skip interactive prompt for AI agents/CI
  - `GNAW_JSON=1` environment variable for machine-readable output
  - Confidence threshold: filters < 0.2, warns < 0.5
  - Multi-language `extract_name_from_preview`: 15+ patterns (Rust, Python, Go, JS, Java, C, QML)
  - Standardized `err_modernbert_disabled()` helper with JSON support

### Fixed
- **quick-replace literal \n bug**: Auto-detects literal `\n`/`\t` in replacement text and converts to real newlines/tabs (prevented broken file writes from CLI escaping)
- **sense-insert position logic**: Fixed off-by-one in `get_next_index()` — was `idx+3+1`, now `idx+3`
- **Compiler warnings**: Eliminated all warnings (unused variable, dead code)

### Changed
- **Performance**: Model caching with `OnceLock<ModernBertModel>` — loads once, reuses across calls
- **Performance**: JIT file index cache with content-hash invalidation in `GnawSenseBroker`
- **SemanticIndex** now derives `Clone` for caching support

### Documentation
- GnawSense SKILL.md for AI agents at `~/.pi/agent/skills/gnaw-sense/`
- ROADMAP.md: detailed 5-tier GnawSense improvement plan with measured baselines
# Changelog

All notable changes to GnawTreeWriter.

## [0.9.2] - 2026-02-05

### Fixed
- **Critical Compilation Errors**: Fixed 56+ syntax errors in `src/cli.rs` caused by unescaped double quotes in help text examples. All println! statements containing nested quotes have been properly escaped.
- **Build System**: Restored compilation on Rust stable by fixing string literal syntax issues.

### Technical Details
- Problem: String literals like `println!("text "quote" more")` were interpreted as separate tokens
- Solution: Escaped all nested quotes as `println!("text \"quote\" more")`
- Affected: Help text in examples subcommand covering editing, search, restoration, and AI features
- Lines modified: ~80 println! statements across 30+ example categories

## [0.9.1] - 2026-02-04

### Added
- **Surgical Inline Editing**: Character-level precision for code edits. You can now edit specific nodes (like parameters or variable names) within a single line without affecting surrounding code.
- **Pedagogical Syntax Tips**: The editor now provides language-specific advice when an edit fails syntax validation (Rust, QML, Python).
- **Column-Aware TreeNode**: Upgraded `TreeNode` structure and the Rust parser to track and utilize character offsets for enhanced precision.

### Changed
- **Enhanced Documentation**: Updated `README.md`, `examples`, and the interactive `wizard` to reflect the new surgical precision capabilities.
- **Version Bump**: Major refinement release marking the transition to v0.9.1 "The Surgical Update".

### Fixed
- **Precision Failures**: Resolved issues where inline edits would inadvertently delete parts of the line.
- **CLI Robustness**: Improved error reporting for JSON and cross-file operations.

## [0.9.0] - 2026-01-31

### Added
- **Slint Support**: Full AST-based editing and analysis for `.slint` files. Powered by `tree-sitter-slint`.
- **AI Default**: The `modernbert` (GnawSense) and `mcp` features are now enabled by default. No more `--features` flags needed for standard usage.
- **Enhanced Status**: The `status` command now proudly displays the state of **GnawSense**, **HRM2** (Hierarchical Reasoning), and **Undo/Redo** history.
- **GnawTree Architect Skill**: A specialized agent skill (`gnawtree-architect`) to guide AI agents in surgical code editing.

### Fixed
- **Safety Nets**: Implemented node count limits (500-1000 nodes) and depth limits in `list`, `skeleton`, and MCP tools to prevent agent context crashes.
- **Memory Optimization**: Refactored `list_nodes` to avoid cloning entire subtrees, significantly reducing memory usage on large files.
- **CLI Hygiene**: Removed duplicate `Status` command handlers and cleaned up unused imports in core modules.
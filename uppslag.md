# Uppslag — öppna trådar, osäkerheter och halvbevis

> **Vad hör hit:** påståenden vi tror på men inte bevisat, danger-klassade
> luckor, och info som är svår att hitta. Inte buggar med repro (de går i
> `GTW_MCP_ISSUE_LOG.md`), inte "vad ska byggas" (`ROADMAP.md`), inte
> dokumenterade begränsningar (`KNOWN_ISSUES.md`).
>
> **Form:** nyast först. Varje post: påstående → bevis → hur det verifieras →
> status → vad som händer om det är fel.
>
> Status: `obelagt` (gissning) · `belagt` (mätt eller läst i kod) · `åtgärdat` ·
> `falsifierat`

---

## Kvarvarande trådar (sammanställning 2026-11-11)

Fyra öppna, ingen av dem tyst, ingen akut. Ordning = billigast först.

| # | Tråd | Varför den väntar | Insats |
|---|---|---|---|
| 1 | `AiManager` väljer modellcache **allt-eller-inget på `modernbert`** (`ai_manager.rs:563`) | påverkar var `ai setup` installerar; ska ha test åt båda hållen | ~1 h |
| 2 | `sense` saknar `root`-parameter | kräver broker-cache per rot — modell-laddningen är delad per process idag | ~2–3 h |
| 3 | Grafen: 21 % (doctor) / 10,1 % (store), **imports 9,7 %** misstänkt, recall 4/5 | kräver utredning: *vilka* kanter löses inte och varför. `use crate::…` borde lösas — troligen en riktig bugg | utredning först |
| 4 | Cosinus-tröskeln **0,68** är fitted på 7 frågor | behöver ~15 frågor med kontrollerat facit innan den är annat än provisorisk | ~1 h |

Plus två beslut som är ägarens, inte kodens: `code-index.md`-stämpelns kontrakt
(1 rads churn per commit) och om Crush-skillen ska testas.

---

## 2026-11-10 — `sense` saknar `root`-parameter  🟨

**Belagt:** `code_index` tar `root` (default `.`), `sense` tar bara
`query`/`file_path`/`expand`. Från en `$HOME`-rotad daemon kan en agent alltså
lösa `code_index` med `root=<absolut>` (bevisat: gav 11 riktiga noder), men
**inte** `sense` — där återstår `file_path` (zoom), CLI:t, eller att klienten
startas i projektet.

**Delvis åtgärdat:** rotvakten sa först "pass an explicit absolute root/path"
även för `sense` — råd som inte gick att följa. Nu är meddelandet
verktygsspecifikt (`af3fa63`) och testet
`broad_root_error_advises_only_what_the_tool_can_do` låser fast det.

**Kvar:** ge `sense` en riktig `root`. Kräver att sense-brokern kan instansieras
per rot (idag byggs den en gång ur `state.project_root`) — alltså en cache
`root → broker` i stället för en enda. Inte gjort; rör modell-laddning.

---

## 2026-11-10 — `$HOME`-roten är en Cline-grej, inte opencode  ✅ mätt

**Bevis:** alla fem `gnawtreewriter mcp`-daemoner hade förälder
`cline-cli --cline-hub-daemon --cwd /home/tuulikk`; ingen hade opencode som
förälder. Fyra stod i `/home/tuulikk`, **en** i `GnawTreeWriter` — samma
förälder. Alltså: hubben *kan* spawna rätt; det beror på vilken workspace som
öppnas, inte på GTW.

**Slutsats:** `$HOME`-roten kommer från att Cline-hubben körs med
`--cwd /home/tuulikk` och att GTW-entryn saknar `cwd`. OpenCode:s modell (en
daemon per katalog) har inte den felklassen — men det är bevisat för *nuvarande*
daemoner, inte för alla framtida.

**Klientagnostiskt är däremot staleness** (långlivade stdio-servrar håller sin
bild) — därför sitter rotvakten serverside och skyddar båda.

---

## 2026-11-10 — Staleness-larmet var avklippt  ✅ ÅTGÄRDAT

**Mätt:** `doctor` rapporterade `stale: false` för en **0.25.0**-binär mot
**0.25.1**-källa. Orsak: `src/version.rs` läste `CARGO_MANIFEST_DIR` med
`std::env::var` (**runtime**) — variabeln finns bara under bygget, så `run_git`
gav upp (`None` → `head_moved = false`) och mtime-jämförelsen föll tillbaka på
cwd (daemonens `$HOME`). **Larmet var byggt och tråden klippt.**

**Åtgärdat:** `const MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR")` på båda
ställena + regressionstest `staleness_check_can_reach_git_and_the_source_tree`.
Bevisat i skarp miljö: `touch src/version.rs` → doctor säger `⚠ BINARY STALE`;
ombygge → tyst.

**Notera dubbelimplementationen:** hooken har sin egen koll i bash
(`src/cli.rs:2419`) som *fungerade* hela tiden. Två implementationer av samma
fråga, en levande och en död — samma driftklass som extensions-listan,
upplösningsregeln och `code-index.md`-namnet.

---

## 2026-11-10 — Växeln finns redan som OSS — en lucka kvar  🟨

**Kontrollerat 2026-11-10 (README + docs/architecture.md):**

| Verktyg | Vad det gör | Lucka |
|---|---|---|
| [`mcpmon`](https://github.com/neilopet/mcpmon) (Node, MIT) | Transparent stdio-proxy; startar om barnet vid filändring, **klienten förblir ansluten**, buffrar meddelanden, krasch-omstart med backoff. `MCPMON_WATCH` styr vad som bevakas. | Inget `--cwd`. Arkitekturdokumentet nämner **inte** `initialize`-replay till det nya barnet — måste verifieras. |
| [`mcp-reloader`](https://github.com/0xHayd3n/mcp-reloader) (Node, MIT) | Injicerar ett `reload`-**verktyg** som agenten anropar (`--build` + respawn), utnyttjar `tools/list_changed`. Har `--cwd`. | Uttryckligen **osäker mitt i ett anrop**: "any tool call still in flight against the old backend will error out… not for firing mid-operation". |

**Slutsats:** bygg ingen egen växel först. `mcpmon` täcker drift-halvan
(klientanslutning + auto-omstart + krasch-omstart). Den **odefinierade** delen
är MCP-handskakningen: ett nytt barn som aldrig sett `initialize` +
`notifications/initialized` får enligt specen inte ta `tools/call`. Att fånga
klientens `initialize` och spela upp den mot det nya barnet är ~50 rader — och
det gynnar *varje* MCP-server, inte bara GTW. Det är den bidragen som är värd
något, inte ännu en proxy.

**För vårt eget bruk, i den ordningen:** (1) `mcpmon` framför GTW med
`MCPMON_WATCH` på `src/` + binären, `bash -c "cd <proj> && exec …"` för cwd:t
(mcpmon har ingen `--cwd`); (2) mät om ett omstartat barn faktiskt tar
`tools/call` utan replay; (3) är svaret nej → bidra med replayen uppströms.

---

**Avgränsning (mätt 2026-11-10):** det är **GTW och gnawdriver** som byggs om
ofta — inte memory-server/dokubase. Då kollapsar "delat system för många
projekt" till *en wrapper, en watch-path*: båda binärerna hamnar i samma
`$CARGO_TARGET_DIR/release/` (`gnawtreewriter` 75 MB, `gnawdriver` 21 MB), så en
bevakning av den katalogen täcker båda.

Asymmetrin som förklarar varför GTW känns värre: **gnawdrivers MCP-entry pekar
redan på build-output** (`${CARGO_TARGET_DIR}/release/gnawdriver`) medan
**GTW:s pekar på install-output** (`~/.cargo/bin/gnawtreewriter`). Samma kod,
olika halveringstid — GTW kräver ett extra `cargo install` innan nästa spawn är
färsk. Att peka GTW på `${CARGO_TARGET_DIR}/release/gnawtreewriter` (bytes-identisk
idag, verifierat med `cmp`) tar bort det steget. **Men** det gör bara *nästa*
spawn färsk — den körande daemonen håller fortfarande sin bild. Pointern gör
bytet billigt; wrappern är det som gör det.

`default = ["modernbert", "mcp"]` betyder att en vanlig `cargo build --release`
är tjänsteduglig — så en swap-validering behövs inte för normalbygget (däremot
för `--no-default-features`, som CI kör).

---

Om man ändå bygger egen växel (först då) är designen: se till att **ingen
process håller en kodbild längre än koden lever**.

```
klient (stdio) ⇄ relay (stabil, ~200 rader: radramning + byte-pump)
                   ⇄ [worker]  exec:ad från disk, byts ut vid behov
```

- Relayn håller klientanslutningen och **byter worker** när binärens identitet
  (path + inode + mtime) ändras — mellan två rader, aldrig mitt i ett anrop.
  Den gamla workern får köra klart och avsluta.
- Workern är disposable: kraschar den startar relayn en ny — vilket samtidigt
  fixar "MCP-servern dog = sessionen död".
- Staleness blir omöjligt *by construction*: ingen process lever längre än den
  binär den startade med.

**Varför inte bara självutgång:** den kräver att *någon annan* startar en ny
process. Mätt 2026-11-10: klienten gör inte det ("MCP server is not connected"
— kom inte tillbaka av sig själv). Växeln tar bort det beroendet, eftersom den
aldrig släpper klientanslutningen.

**Kostnader (ärliga):** en ny komponent i tillitskedjan — den kan krasha och
måste testas för swap-mitt-i-anrop, krasch-omstart och dubbel-swap;
modellvärme tappas vid swap (acceptabelt, swap är sällsynt — högst en gång per
installation); och den måste vara **högljudd** (logga varje swap, exponera
`worker_generation` i health) annars återinför den en tyst felklass.

**Fasning:** 0 = `daemon-hygiene.sh` (klart) → 1 = `E_DAEMON_STALE` + exit efter
svar (billigt, men beroende av att klienten respawnar) → 2 = `mcpmon` framför
servern, alternativt ett replay-bidrag uppströms. Starta inte 2 före en paus:
den rör MCP-livscykeln och behöver egna tester.

---

## 2026-11-10 — Daemoner som överlever sin binär: sopa eller självutgång?  🟨

**Mätt:** åtta `gnawtreewriter mcp`-daemoner, 95–285 minuter gamla, **alla** med
`readlink /proc/<pid>/exe` → `(deleted)`. De betjänade kod som inte längre finns
på disk — det är hela förklaringen till att "fixen inte tog" under sessionen.

**Gjort:** alla åtta dödade. `scripts/daemon-hygiene.sh` listar (pid, ålder, cwd,
`deleted`/`on-disk`, RSS) och `reap` dödar **bara** `(deleted)`-poster, med
`--older-than` och dry-run som default. Bevisat på en kontrollerad stale-daemon
(binär kopierad, startad, sedan raderad → verktyget såg `(deleted)` och dödade
exakt den, medan de levande fick stå).

**Bättre än en periodisk sopare (förslag, ej byggt).** Låt daemonen känna sin
egen föråldring i stället för att en janitor gissar:

1. vid start: spara identitet för `/proc/self/exe` (path + inode + mtime);
2. per anrop: stat'a om — ändrad eller försvunnen → svara `E_DAEMON_STALE`
   ("nyare binär på disk, starta om MCP-servern") och avsluta **efter** svaret;
3. idle-timeout (t.ex. 30 min, env-styrd) → exit 0, så oanvända daemoner dör
   själva utan att någon sveper.

En sopare behöver ett schema och en definition av "inaktiv" som **inte går att
mäta utifrån** — man ser CPU-tid, inte klientens avsikt — och den kommer förr
eller senare att döda någon som används. Självutgång avgörs av den enda part
som vet svaret.

**Mätt baksida (avgörande för utformningen):** att döda *den här sessionens*
daemon kopplade ner klienten — *"MCP server is not connected"* — och den kom
**inte** tillbaka av sig själv. Därför dry-run som default, `--older-than`, och
reapa helst mellan sessioner. Samma skäl kräver att en idle-timeout är generös
(≥30 min): en lång klient-tur utan verktygsanrop får inte se daemonen dö.

---

## 2026-11-10 — JSON: `--preview` och apply är oense  ✅ ÅTGÄRDAT

`preview_edit` tog en **annan väg** än den skarpa edit: apply grenar till
`json_semantic_edit` för JSON, preview anropade `edit_node_at_path` för allt.
Alltså visade `--preview` den råa textdelningen medan apply skrev om värdet
semantiskt — en förhandsvisning av ett dokument som aldrig skulle uppstå.

**Fixat:** ett enda predikat `is_json_document()` avgör för **båda**, och
invarianten är låst i testet `json_preview_and_apply_agree` (*efter* apply ska
samma edit förhandsvisa **ingen** ändring). Verifierat på installerad binär:
preview `- "name": "old"` / `+ "name": "new"` → apply skrev exakt det → ny
preview utan ändring.

---

## 2026-11-10 — Rotvakten fångar `$HOME` men INTE förälderkataloger  ✅ ÅTGÄRDAT

**Påstående:** `root_is_too_broad` avvisar `$HOME` och `/`, men en MCP-daemon
med `cwd=/mnt/.../Gnaw-Software` — förälder till sex repon — passerar vakten,
och `sense`/`lexical_degrade` vandrar då **alla** repon.

**Bevis:** `readlink /proc/<pid>/cwd` 2026-11-10 visade en daemon med exakt den
katalogen (pid 3419795). Vakten jämför bara mot `$HOME` och `/`.

**Verifieras:** `printf … | gnawtreewriter mcp stdio` med `cd` till
förälderkatalogen. Svarar `sense` med träffar ur syskonrepon är luckan kvar.

**Status:** `belagt`, ej åtgärdat — vakten är ofullständig.

**ÅTGÄRDAT 2026-11-10** (`looks_like_workspace_container`): en katalog som
saknar egen projektmarkör (`.git`/`Cargo.toml`/`package.json`/`pyproject.toml`/
`gnawtreewriter.yaml`) men där **majoriteten av underkatalogerna** är projekt
avvisas nu. Regeln kräver majoritet, inte "två ser ut som projekt": första
versionen nekade `/tmp` (temp-byggda kataloger med `Cargo.toml` ser ut som
projekt) — det fångades av `a_project_directory_is_not_too_broad`, som nu testar
en junk-drawer med exakt ett projekt. Monorepo (markör i roten) går fri.
Beslut av ägaren: **neka**, inte varna.

**Om den förblir olöst:** en agent som öppnar sin klient i en workspace-rot får
tyst svar från flera projekt. Samma klass som AndroidSDK-läckan.

**Möjliga riktningar:** kräv projektmarkör (`.git`/`gnawtreewriter.yaml`) i
roten, eller `root`-parameter på `sense`. Undvik heuristik som gissar åt
användaren.

---

## 2026-11-10 — `sense`-poäng ~0,01–0,02: gammalt index eller fel embedder?  ✅ ÅTGÄRDAT

**Påstående (obesvisat):** de låga poängen beror på att `embeddings.db` är
daterad 2026-10-07, och att `gnawtreewriter ai index` höjer dem.

**Bevis:** CLI `sense` i repot gav topp **0,02** ("Low confidence < 0.5") med
träffar i `src/core/anchor.rs` och `src/core/curator.rs` — **fel svar**, eftersom
`src/core/code_graph.rs` (som äger `cycles`) finns i repot. MCP-probe gav **0,01**
över 10 träffar. `embeddings.db` var daterad 2026-10-07.

**Embedder-mismatch är UTESLUTEN (mätt):** `model_info` i indexet säger
`bge-base-en-v1.5` med `dimension 768` — samma modell som konfigurationen
använder. Alltså pekar allt mot gammalt *innehåll*, inte fel modell.

**Nytt fynd — `ai index` är inte ett loop-steg:** på CPU (binären är byggd utan
`cuda`) tog en full omindexering av 180 filer **>9 minuter utan att bli klar**
(≈340 % CPU, 520 MB RSS, upsertar ~7 600 rader). Det är en bakgrundssyssla man
startar och går ifrån, inte något som hör i en snabb cykel — och docs som säger
"kör `ai index`" bör säga det.

**MÄTT OCH ÅTGÄRDAT 2026-11-11.** Två buggar låg staplade, och de förklarade
tillsammans varför `sense` såg trasigt ut:

| | gammalt index (7 okt) | färskt index | + cosinus-fix |
|---|---|---|---|
| topp-träff | `anchor.rs` — **fel** | `docs/…EXTENSIONS.md` | samma |
| `code_graph.rs` (äger `cycles`) | **saknas** | **rank 5 och 7** | rank 5 och 7 |
| visad poäng | 0,02 | 0,02 | **cosine 0,63–0,73** |
| "Low confidence"-varning | **alltid** | **alltid** | **borta** |

1. **Indexet var stale** → fel *svar*. Botat med `gnawtreewriter-gpu ai index
   --gpu` (276 filer, ~2 min).
2. **Poängen var fel mått** → fel *skala*. `score` är en RRF-fusion:
   `1/(60+rank)` ≈ **0,017** vid rank 0, medan `src/cli.rs` jämförde den mot
   **0,5** — en cosinus-tröskel. Koden *dokumenterade* till och med faran
   (`semantic_index.rs`: *"Consumers must never mistake the adjusted value for a
   cosine"*) och CLI:t gjorde det ändå. Varningen kunde aldrig utebli.
   Fix: `sense_confidence(score, cosine)` väljer den **rå cosinus** som
   `FileMatch` redan bar; varningen jämför nu 0,5 mot ett mått där 0,5 betyder
   något. Test: `confidence_uses_the_raw_cosine_when_available`.

**Läxa:** en sökning som fungerar kan se trasig ut enbart för att
diagnostiken mäter fel storhet — och ett larm som alltid lyser lär folk
ignorera det (samma klass som `ai status` och det avklippta stale-larmet).

### Är 0,5 rätt tröskel? NEJ — mätt med kalibreringsset

Sju frågor, facit kontrollerat i koden (två av mina första "facit" var fel och
är korrigerade nedan):

| Fråga | topp-cosinus | facit |
|---|---|---|
| batch apply atomic | 0,79 | ✅ `batch.rs` i listan |
| dependency cycles | 0,73 | ✅ `code_graph.rs` |
| markdown parser | 0,73 | ✅ `markdown.rs` |
| transaction log | 0,72 | ✅ `transaction_log.rs` |
| mcp tool dispatch | 0,71 | ❌ **miss** (`mcp/mod.rs` finns, `tools/call` där) |
| *"OAuth login"* (finns ej) | **0,63** | — negativ |
| *"backup rotation"* (finns ej) | **0,62** | — negativ |

**Tre slutsatser:**

1. **0,5 släpper igenom negativa frågor.** En fråga om något som inte finns i
   repot får 0,62–0,63 — över tröskeln. "Ingen varning" betyder alltså **inte**
   "svaret finns".
2. **Höjning till ~0,7 löser det inte heller:** sanna träffar på rank 5 låg på
   0,65–0,66, så en 0,7-grind skulle varna trots att rätt fil låg i listan.
   Ett enda cosinus-tal kan inte göra båda jobben (gate för "finns svar" vs
   "toppträffen är rätt").
3. **Rätt signal finns redan:** `Harmony` bär cosinus + lexikal överlappning +
   graf-pelare, och en negativ fråga har typiskt låg lexikal överlappning. Att
   döma på cosinus **ensam** är därför arkitektoniskt fel, inte bara
   okalibrerat.

**Kvar:** ~~byt confidencen till `harmony`~~ — **GJORT 2026-11-11**
(`sense_confidence` i `src/cli.rs`). Två signaler, för ett tal kan inte göra
båda jobben:

| Fråga | cosinus | verdict per träff | utfall |
|---|---|---|---|
| "dependency cycles" (finns) | 0,73 | minor dissonance | **ingen varning** |
| "OAuth login" (**finns inte**) | 0,63 | minor dissonance | **varning**: *"best cosine 0.63 < 0.68 and the channels do not agree"* |

Regeln: `varna = disharmonious || (cosinus < 0.68 && ej harmonious)`. Alltså kan
harmony **rädda** en medioker cosinus (pelarna är ense) och **fälla** en hög
(utpräglad oenighet), medan cosinus-grinden tar "konceptet finns inte"-fallet.
Tröskeln 0,68 är **provvisorisk** — fitted på de sju frågorna (sanna toppträffar
0,71–0,79; negativa 0,62–0,63) och måste mätas om för annat repo/embedder.

**Mätt under vägen:** `Pillars { semantic: 0.63, lexical: 0.0, graph: 0.0 }` ger
**MinorDissonance**, inte Disharmonious (dissonans 0,397 mot gränserna 0,25/0,50)
— min första förväntan var fel, och testet pinnar nu det verkliga beteendet.

**Nytt fynd — `ai index` på GPU:** ~2 minuter för 276 filer (734 MiB VRAM, 97 %
ledigt vid start, 20 %-grinden passerad) mot **>22 min utan att bli klar** på
CPU. GPU-vägen kräver `gnawtreewriter-gpu` (eget namn sedan 2026-11-11, så
`cargo install` inte kan klippa den).

**Om påståendet är fel:** vi lägger en eftermiddag på en ombyggnad som inte
löser något, medan docs säger "fixa med `ai index`" och `sense` förblir
oanvändbart.

---

## 2026-11-10 — `AiManager` väljer modellcache på fel villkor  🟧

**Belagt i kod** (`src/llm/ai_manager.rs:563`): `local_cache` används bara om
**`modernbert`** finns lokalt, annars används hela `$HOME`s katalog. En global
`bge-base` ignoreras alltså när `modernbert` finns lokalt — och tvärtom.

**Symptom:** `sense` rapporterar "degraderad, modell saknas" trots att modellen
ligger i projektcachen, medan `ai status` visar ✅ (den rapporterar en annan
katalog).

**Status:** `belagt`, ej åtgärdat. Rätta: lös **per modell**, inte per katalog.

---

## 2026-11-10 — Kunskapsgrafens 21 %: användbar eller bara trovärdig?

**Mätt:** `doctor` → 3 335 / 15 257 call-kanter upplösta (**21 %**), 70
ambivalenta symbolnamn (`new`, `default`, `derive` …).

**Osäkerhet:** om `impact`/`chain` ger *användbara* svar vid 21 %, eller bara
*trovärdiga*, är inte prövat på ett verkligt beslut. `--why` gör varje kant
falsifierbar — det är svaret på "får man lita på den", inte siffran.

**Verifieras:** ta en verklig ändring, kör `--impact`, jämför mot `grep`, räkna
falska och missade kanter.

---

## 2026-11-10 — CLI-vägen saknar rotvakt  ✅ ÅTGÄRDAT

**Var:** `code-index`, `gnaw-find` och `sense` på CLI:t hade ingen
`root_is_too_broad`; från `$HOME` indexerade de hela hemkatalogen medan MCP-verktygen
nekade. Två implementationer av samma fråga — samma driftklass som resten av dagen.

**Fixat 2026-11-11:** regeln bor nu i **`src/core/project_root.rs`** (`root_is_too_broad`,
`looks_like_workspace_container`, `guard`), och både MCP-verktygen och CLI:t
anropar den. `Commands::CodeIndex` nekar med exit **2**.

**Verifierat live:**

```
cd $HOME && gnawtreewriter code-index . --coverage   → exit 2
   "refusing to use . as a project root — it is the home directory …"
cd <repo> && gnawtreewriter code-index . --coverage  → exit 0, 183 filer
```

Testerna flyttade med (`core::project_root::tests`, 4 st) och MCP behåller bara
det som är verktygsspecifikt: rådet i felmeddelandet.

---

## 2026-11-10 — Dokument som redan rostat (samma sjuka som Crush-paketet)

- `KNOWN_ISSUES.md`: *"Current as of v0.9.8"* (verifierad 2026-10-04) — 16
  minor-versioner bakom. Antingen verifiera om eller märk som historisk.
- `CRUSH_INTEGRATION_GUIDE.md`: innehöll "✅ MCP link registered in **Gemini**
  CLI". Påståendelistan är ersatt med verifieringskommandon, men resten av
  guiden är från v0.9.x-eran och bör läsas kritiskt.
- Mönstret: **en påståendelista utan kommandon bakom sig rostar tyst.** Ersätt
  med kommandon (`--version`, `doctor`, `pack-skill.sh --check`).

---

## 2026-11-10 — JSON: `--preview` och apply är oense  ✅ ÅTGÄRDAT

*Dubblett av posten ovan; behållen som pekare eftersom den gamla texten felaktigt
sa "kräver designbeslut, inte en fix". Det var en fix: `is_json_document()`
används nu av både preview och apply, låst av `json_preview_and_apply_agree`.*

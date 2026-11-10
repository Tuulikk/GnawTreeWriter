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

## 2026-11-10 — Staleness-larmet var avklippt  🟥 FARLIG

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

## 2026-11-10 — Daemoner som överlever sin binär: sopa eller självutgång?  🟥 FARLIG

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

## 2026-11-10 — Rotvakten fångar `$HOME` men INTE förälderkataloger  🟥 FARLIG

**Påstående:** `root_is_too_broad` avvisar `$HOME` och `/`, men en MCP-daemon
med `cwd=/mnt/.../Gnaw-Software` — förälder till sex repon — passerar vakten,
och `sense`/`lexical_degrade` vandrar då **alla** repon.

**Bevis:** `readlink /proc/<pid>/cwd` 2026-11-10 visade en daemon med exakt den
katalogen (pid 3419795). Vakten jämför bara mot `$HOME` och `/`.

**Verifieras:** `printf … | gnawtreewriter mcp stdio` med `cd` till
förälderkatalogen. Svarar `sense` med träffar ur syskonrepon är luckan kvar.

**Status:** `belagt`, ej åtgärdat — vakten är ofullständig.

**Om den förblir olöst:** en agent som öppnar sin klient i en workspace-rot får
tyst svar från flera projekt. Samma klass som AndroidSDK-läckan.

**Möjliga riktningar:** kräv projektmarkör (`.git`/`gnawtreewriter.yaml`) i
roten, eller `root`-parameter på `sense`. Undvik heuristik som gissar åt
användaren.

---

## 2026-11-10 — `sense`-poäng ~0,01–0,02: gammalt index eller fel embedder?  🟨

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

**Kvar att mäta:** efter-poängen. Kommandot:
`gnawtreewriter sense 'where are dependency cycles detected'` → jämför topp-poäng
och om `code_graph.rs` hamnar i listan.

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

## 2026-11-10 — CLI-vägen saknar rotvakt

`code-index`, `gnaw-find` och `sense` på CLI:t har ingen `root_is_too_broad`.
Från `$HOME` indexerar eller letar de hela hemkatalogen. MCP-vakten täcker dem
inte. **Status:** `belagt`, oavsiktligt — vakten sattes där läckan uppstod.

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

## 2026-11-10 — JSON: `--preview` och apply är oense

Känt och pinnat i test: `--preview` visar rå split, skarp edit tar
`json_semantic_edit`. En agent som litar på preview-diffen får ett annat resultat
än den fick se. Kräver designbeslut, inte en fix.

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

**Bevis:** CLI `sense` i repot gav topp **0,02** ("Low confidence < 0.5");
MCP-probe gav **0,01** över 10 träffar. `ls -la
.gnawtreewriter_ai/index/embeddings.db` → 7 okt 04.29.

**Verifieras:** kör `ai index`, mät om topp-poängen stiger. Stiger de **inte** är
orsaken embedder/dimension (index byggt med annan modell) — då krävs ombyggnad
med metadatakontroll, inte bara ett nytt index.

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

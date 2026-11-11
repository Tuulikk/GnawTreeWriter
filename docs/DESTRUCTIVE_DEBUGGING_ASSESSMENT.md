# Bedömning: destruktiv felsökning som GTW-metod

**Granskning av förslaget "Destruktiv felsökning och experimentell problemisolering"**
(underlag från ägaren 2026-11-11) · bedömd mot koden 2026-11-11 · GTW 0.25.1

## Sammanfattning

**Metoden passar GTW — den är GTW:s kärna tillämpad på diagnos i stället för
ändring.** Kirurgisk nedmontering, maskinell reversibilitet och en journal är
exakt vad verktyget redan är byggt för. Men förslaget har **sju förmågor där tre
räcker**, och den viktigaste förutsättningen för hela metoden är underdimensionerad:
**observationen**.

Rekommendation: bygg **en** kommandoyta (`experiment <spec.json>`), inte sju
förmågor, och låt allt som inte är kodkirurgi eller journal komma utifrån
(`--observe`).

## Vad som redan finns (mappning)

| Förslagets förmåga | Befintligt i GTW | Status |
|---|---|---|
| 1. Experimentell arbetsyta | `save_state`, `diff_since`, `RestoreProject`, transaktionslogg | **delvis** — ingen git-worktree-hjälp |
| 2. Kontrollerad nedmontering | `edit_node` + `batch` (atomisk, validerar före skrivning), `delete`, `insert` | **klart** — detta är kärnan |
| 3. Systematisk återuppbyggnad | `undo`, `restore-session`, plus worktree-checkout | **klart** (samma loop, färre nedmonteringar) |
| 4. Experimentjournal | mönstret finns: `.gnawtreewriter_*.jsonl` (söklogg, metrics), `history`, receipts med `node_content_hash` | **saknar schema** för hypotes/förväntan/observation |
| 5. Observation | *inget* — GTW kör bara `git` (`Command::new("git")` är enda externa anropet) | **saknas helt** |
| 6. Intelligent val av experiment | `code_index` (items, `impact`, `cycles`), `blast` | **delvis** — kandidater kan tas fram; valet är agentens |
| 7. Överlämning | `uppslag.md`, `GTW_MCP_ISSUE_LOG.md`, Motor2-minne | **klart** som mönster |

Slutsats: **5 av 7 bygger på befintlig funktionalitet.** Det som saknas är en
observerbar orakelkanal och ett journalschema — inte en experimentmotor.

## Vad som måste ändras i förslaget

### a) Oraklet är hela metoden — och det är underdimensionerat

Förslaget säger "observera rendering och beteende" utan att definiera vad som
avgör ett utfall. Utan en **maskinavgörbar** observation urartar destruktiv
felsökning till gissningar med extra steg, och en liten modell kan inte alls
avgöra om något förändrades.

Kravet: observationen ska vara ett **kommando** med ett **predikat**:

```json
"observe": { "cmd": "cargo test --test ui 2>&1 | tail -40",
             "expect": { "exit": null, "absent": "SIGSEGV", "present": "rendered" } }
```

GTW ska **inte** tolka "felet" — bara köra, fånga exitkod/stdout-hash och jämföra
mot predikatet. Det håller GTW utanför domänlogik och gör metoden användbar för
vilket språk som helst.

### b) Observation är en ny, säkerhetskänslig förmåga

GTW kör i dag **endast** `git`. Att köra godtyckliga kommandon kräver, utöver
ovanstående: explicit opt-in i specen, **ingen shell-expansion av nodinnehåll**,
timeout per körning, och att kommandot körs i experimentytan — aldrig i
användarens arbetsträd. Detta är den enda riktigt nya attackytan i förslaget och
bör vara medvetet smal.

### c) UI-observation hör inte i GTW

Rendering, skärmbilder och layout är **gnawdriver/Motor2:s** domän (de har redan
screenshot, DOM-observation och `verify`). GTW ska definiera **kontraktet**
(exitkod + artefaktvägar) och låta `--observe` peka på deras verktyg. Att bygga
skärmdumpsstöd i GTW är exakt det "parallella system" förslaget varnar för.

### d) Slå ihop 2 och 3

"Nedmontering" och "systematisk återuppbyggnad" är **samma mekanik** med olika
ordning: `disable`/`restore` i en loop. Två förmågor blir två kodvägar som
driver isär — dagens mest återkommande buggklass i det här repot.

### e) "Intelligent val av experiment" ska inte in i verktyget

Att välja försök är agentens (eller människans) uppgift; ett verktyg som gissar
åt användaren är mot doktrinen. Verktyget ska i stället **föreslå kandidater**:
subträd från `code_index` sorterade på `impact`-storlek, så valet blir upplyst i
stället för påhittat.

Undantag: **ren halvering kan vara mekanisk** (`strategy: "bisect"`) — den kräver
ingen förståelse alls, bara ett orakel.

### f) Budget är inte en detalj

Varje försök kostar en byggnad/testkörning. Utan `max_runs` och `max_seconds`
blir metoden en tids- och tokensänka som ingen kan avbryta. Motor2:s flödesbudget
är förebilden: budgeten ska stoppa **ärligt och rapportera**, inte kasta.

### g) "Orsak vs beroende" behöver mekanik, inte en varning

Förslaget varnar korrekt för att "felet försvann när X togs bort" inte bevisar att
X är fel — X kan vara en förutsättning. Men varningen behöver stöd: journalen ska
per nedmonterad nod registrera dess `impact`-beroenden, och rapporten ska flagga
utfallet **"symptomet försvann samtidigt som konsumenterna slutade anropas"** som
`unused_not_cause`. Det är skillnaden mellan en observation och en slutsats.


## Minimal prototyp (förslag)

**En** kommandoyta och **ett** MCP-verktyg — inte sju.

```
gnawtreewriter experiment <spec.json> [--dry-run] [--json]
```

```json
{
  "workspace": "/tmp/gtw-exp-42",          // MÅSTE vara en git worktree (annars: vägra)
  "observe": { "cmd": "cargo test --test ui 2>&1 | tail -40",
               "expect": { "exit": null, "absent": "SIGSEGV", "present": "rendered" } },
  "baseline": "must_reproduce",            // annars avbryt: inget att isolera
  "candidates": ["1.2", "1.3.1"],          // eller "auto" -> code_index-subträd
  "strategy": "bisect | single | rebuild",
  "budget": { "max_runs": 12, "max_seconds": 900 },
  "journal": "experiments.jsonl"
}
```

**Loop (åtta steg, exakt förslagets prototyp):**

1. Vägra om `workspace` inte är en worktree med rent träd (annars kan experiment
   förstöra användarens arbete — förslagets viktigaste säkerhetskrav).
2. Kör `observe` → **baslinje**. Upprepas inte felet: avbryt med
   `{"reproduced": false}`. Ett icke-reproducerbart fel går inte att isolera.
3. Välj kandidat(er) enligt `strategy`; `bisect` halverar listan mekaniskt.
4. Nedmontera med befintlig `batch` (atomisk, validerar före skrivning) —
   `edit_node` till tom kropp, eller `insert` av en `cfg(any())`-grind.
5. Kör `observe` igen. Registrera exit, stdout-hash, diff mot baslinjen.
6. `git -C <workspace> checkout -- .` (reversibilitet utan att modellen minns
   något), sedan nästa kandidat.
7. Upprepa tills budgeten tar slut eller utfallet är stabilt.
8. Skriv rapport: minsta avgränsning, experimenten, `impact`-beroenden för varje
   nedmonterad nod, `unused_not_cause`-flaggor, kvarvarande osäkerheter.

**Rapportformatet** ska vara det som förs vidare till reparationsfasen:

```json
{ "reproduced": true,
  "minimal_scope": ["src/ui/render.rs 24.3"],
  "experiments": [ { "n": 1, "disabled": "src/ui/render.rs 24.3", "expected": "symptom gone",
                     "observed": "symptom gone", "exit": 0, "duration_ms": 4210,
                     "impact": ["Widget::draw"], "verdict": "candidate" } ],
  "dependencies": ["Widget::draw -> Renderer::present"],
  "uncertainties": ["symptomet kan ha flyttat till annan widget — ej observerat"] }
```

## Vad som INTE ska byggas

| Inte | I stället |
|---|---|
| Egen skärmdumps-/UI-observation | `--observe` pekar på gnawdriver/Motor2 |
| Egen testkörare | `--observe` **är** testkörningen |
| Egen isoleringsmekanism | `git worktree` (en rad, beprövat, gratis återställning) |
| Egen agent/loop med egen kontext | MCP-verktyget; agenten äger strategin |
| Auto-val av försök | kandidatförslag från `code_index` + människa/agent väljer |

## Liten modell: gör loopen deterministisk

Förslaget frågar hur en liten lokal modell ska kunna använda metoden utan att
förstå projektet. Svaret: **den behöver inte**. Dela upp det:

- **Modellen** gör två saker: formulerar oraklet (vilket kommando, vilket
  predikat) och väljer startpunkt. Det kräver domänförståelse men inget
  projektsammanhang.
- **Verktyget** gör resten mekaniskt: halvering, nedmontering, observation,
  återställning, journal, budget.

Med `strategy: "bisect"` är loopen deterministisk givet oraklet — vilket är
precis vad en 7B-modell klarar och en 70B-modell inte behöver.

## Koppling till öppna trådar

Kandidatvalet vinner på en bättre graf: `imports` ligger på **9,7 %** upplösning
(se `uppslag.md`), och importkanter är precis vad som avgör om en nedmonterad nod
gör en konsument oanvänd. Prototypen kan köras utan (manuell kandidatlista), men
`unused_not_cause`-flaggan blir trubbig tills importupplösningen är utredd.

## Rekommendation

1. **Prototypen är värd att bygga** — den är ~en dags arbete eftersom 5 av 7
   delar redan finns, och den enda nya ytan (`--observe`) är medvetet smal.
2. **Börja med `single` + manuell kandidatlista**; lägg `bisect` när oraklet
   bevisat sig stabilt. `rebuild` sist — det är samma loop baklänges.
3. **Skriv journalen först.** Utan den är varje experiment en engångsobservation
   som försvinner vid nästa kontextkomprimering — och då uppfyller metoden inte
   sitt eget syfte.

# Bedömning: destruktiv felsökning som GTW-metod

**Granskning av förslaget "Destruktiv felsökning och experimentell problemisolering"**
(underlag från ägaren 2026-11-11) · bedömd mot koden 2026-11-11 · GTW 0.25.1

## Sammanfattning

**Metoden passar GTW — den är GTW:s kärna tillämpad på diagnos i stället för
ändring.** Kirurgisk nedmontering, maskinell reversibilitet och en journal är
exakt vad verktyget redan är byggt för. Men förslaget har **sju förmågor där tre
räcker**, och den viktigaste förutsättningen för hela metoden är underdimensionerad:
**observationen**.

Rekommendation: bygg **en** kommandofamilj (`experiment start|candidates|disable|
record|restore|report`), inte sju förmågor. GTW **kör inga observationer** — den
tar emot dem från agenten, verifierar sina egna ingrepp och sköter journalen.

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

### a) Oraklet kommer UTIFRÅN — GTW tar emot observationen (reviderat 2026-11-11)

Första utkastet föreslog att GTW skulle köra observationen (`--observe "<cmd>"`).
Ägarens invändning är riktig: **GTW behöver inte äta observationerna.** Agenten
har redan ett skal, och observationen är ofta något ett kommando inte kan fånga
(en människa ser att layouten är fel). GTW ska ta **emot** observationen och göra
det enkelt att logga den.

Det flyttar ansvaret dit det hör:

| | Agenten | GTW |
|---|---|---|
| Observera (köra testet, titta på UI:t) | **ja** | nej |
| Veta vad som förväntades | ja | nej |
| Nedmontera kirurgiskt | nej | **ja** (`batch`, validerar före skrivning) |
| Bevisa att nedmonteringen verkligen skedde | nej | **ja** (parsa om noden) |
| Bevisa att återställningen är byte-identisk | nej | **ja** (hash före/efter) |
| Journal, budget, halveringsförslag, rapport | nej | **ja** |

Vinsten är stor: ingen kommandoexekvering i GTW, ingen timeout- eller
shell-injektionsyta, och metoden fungerar även när observationen är visuell eller
mänsklig.

### b) Konsekvensen som måste sägas högt: GTW kan inte verifiera observationen

Om agenten rapporterar "symptomet försvann" kan GTW bara **registrera** det, inte
kontrollera det. Därför måste journalen skilja på två sorters fakta, annars
återupprepar vi dagens felsort: en rapport som ser ut som en mätning men är ett
påstående.

- `verified: true` — det GTW själv kontrollerat: parsen är giltig, noden finns
  inte längre, återställningen gav samma hash som före.
- `reported: "agent"` — det agenten uppgav: utfall, bevis (kommando + exitkod om
  den finns), fritext.

Rapporten ska aldrig blanda dem. Ett experiment vars utfall bara är *reported* är
en hypotes med stöd, inte ett mätvärde.


### c) UI-observation hör inte i GTW

Rendering, skärmbilder och layout är **gnawdriver/Motor2:s** domän (de har redan
screenshot, DOM-observation och `verify`). GTW ska bara ta emot **utfallet** —
`record --outcome changed --evidence "<vad agenten såg>"` — och aldrig själv försöka
avgöra hur något ser ut. Att bygga skärmdumpsstöd i GTW är exakt det "parallella
system" förslaget varnar för.

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

**En** kommandofamilj och **ett** MCP-verktyg — inte sju. Tillståndet lever i
`.gnawtreewriter_ai/experiments/<id>.jsonl`, så agenten kan göra observationen
mellan anropen (och till och med byta agent mitt i — journalen bär kontexten).

```bash
gnawtreewriter experiment start  --workspace <worktree> --symptom "UI fryser vid klick"
gnawtreewriter experiment candidates            # code_index-subträd + impact-sortering
gnawtreewriter experiment disable <path> <node> # via batch; VERIFIERAR att noden är borta
gnawtreewriter experiment record  --outcome gone|same|changed|other \
                                  --evidence "cargo test --test ui → exit 0" [--exit 0]
gnawtreewriter experiment restore               # hash-jämför mot före nedmonteringen
gnawtreewriter experiment report [--json]
```

MCP-motsvarighet: **ett** verktyg `experiment {action, ...}` med samma sex lägen —
inte sex verktyg, och ingen kommandoexekvering någonstans.

`record` ska vara **billigt att anropa**: bara `--outcome` är obligatoriskt, allt
annat är fritt och får saknas. En agent som inte har en exitkod ska inte behöva
hitta på en.

## Loop (åtta steg, förslagets prototyp — med observationen utflyttad)

1. `start`: vägra om `--workspace` inte är en **git worktree** med rent träd
   (annars kan experiment förstöra användarens arbete).
2. Agenten observerar **själv** och rapporterar baslinjen via
   `record --outcome same`. GTW registrerar den som `reported`, inte `verified`.
   Upprepas inte symptomet: `report` säger `{"reproduced": false}` och avbryter.
3. `candidates`: GTW föreslår subträd ur `code_index` sorterade på `impact`.
   `strategy: bisect` halverar listan mekaniskt; annars väljer agenten.
4. `disable`: GTW nedmonterar med `batch` (atomisk, validerar före skrivning) och
   **verifierar** att noden inte längre går att läsa → `verified: true`.
5. Agenten observerar (test, skärmbild, egen bedömning) och anropar `record`
   med utfall + bevis.
6. `restore`: GTW återställer och **hash-jämför** mot tillståndet före
   nedmonteringen → `verified: true/false`. Reversibiliteten kräver aldrig att
   modellen minns något.
7. Upprepa 3–6 tills budgeten tar slut eller utfallet är stabilt.
8. `report`: minsta avgränsning, experimenten (`verified` vs `reported`),
   `impact`-beroenden per nedmonterad nod, `unused_not_cause`-flaggor,
   kvarvarande osäkerheter.

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
| Egen **kommandoexekvering** (test, skärmdump, `sh -c`) | observationen kommer från agenten via `record` |
| Egen skärmdumps-/UI-observation | agenten/gnawdriver observerar, GTW loggar |
| Egen testkörare | agenten kör; `record --evidence` bär beviset |
| Egen isoleringsmekanism | `git worktree` (en rad, beprövat, gratis återställning) |
| Egen agent/loop med egen kontext | MCP-verktyget; agenten äger strategin |
| Auto-val av försök | kandidatförslag från `code_index` + människa/agent väljer |

## Liten modell: gör loopen deterministisk

Förslaget frågar hur en liten lokal modell ska kunna använda metoden utan att
förstå projektet. Svaret: **den behöver inte**. Dela upp det:

- **Modellen** observerar och rapporterar (den kan köra ett test eller titta på en
  skärmbild) samt väljer startpunkt. Det kräver domänförståelse men inget
  projektsammanhang — och inget skal-kommando behöver gå genom GTW.
- **Verktyget** gör resten mekaniskt: halvering, nedmontering, verifiering,
  återställning (hash), journal, budget.

Med `strategy: "bisect"` är loopen deterministisk givet att agenten kan säga
`gone|same|changed` — vilket är precis vad en 7B-modell klarar och en 70B-modell
inte behöver.

## Koppling till öppna trådar

Kandidatvalet vinner på en bättre graf: `imports` ligger på **9,7 %** upplösning
(se `uppslag.md`), och importkanter är precis vad som avgör om en nedmonterad nod
gör en konsument oanvänd. Prototypen kan köras utan (manuell kandidatlista), men
`unused_not_cause`-flaggan blir trubbig tills importupplösningen är utredd.

## Rekommendation

1. **Prototypen är värd att bygga** — den är ~en dags arbete eftersom 5 av 7
   delar redan finns, och **ingen ny exekveryta behövs**: observationen kommer
   från agenten, GTW verifierar bara sina egna ingrepp.
2. **Börja med `single` + manuell kandidatlista**; lägg `bisect` när oraklet
   bevisat sig stabilt. `rebuild` sist — det är samma loop baklänges.
3. **Skriv journalen först.** Utan den är varje experiment en engångsobservation
   som försvinner vid nästa kontextkomprimering — och då uppfyller metoden inte
   sitt eget syfte.

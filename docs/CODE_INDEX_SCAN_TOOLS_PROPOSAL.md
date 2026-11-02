# CODE-INDEX SCAN TOOLS — Förslag

**Status:** Förslag (design, ej byggd) · **Datum:** 2026-11-02
**Proveniens:** Motor2-session 2026-11-02. Original-idé: användaren. Utkast: agent (GLM 5.3 Flash).
**Annan relevans:** `docs/CODE_INDEX_SQLITE_DESIGN.md` (index-lagring), `GAP-REPORT-2026-10-04.md` (genren "hål-rapport" finns redan i repot).

---

## 1. Problem & motiv

Att hitta strukturella hål ("funktioner som saknar koppling där de borde vara",
"påståenden som koden inte håller") kräver idag ad hoc bash-pipelines: grep × N,
manuell läsning, manuella test/produktion-åtskillnader. Bevisade fall från EN
session i Motor2 (2026-11-02):

1. **Doc-header-lögn:** `lib.rs` lovade "Fork/merge/list/switch" — koden
   implementerar bara fork/branch/list. Hittat via kors-join mellan doc-claim
   och code-index-listing.
2. **Kopplings-hål:** ExtensionRegistry-hooks (`run_before_tool_call` m.fl.) —
   konstruerade, registrerade, loggade "initialiserat" — men noll dispatch
   i tool-loopen. `self.extensions` skriven en gång, aldrig läst. Gröna
   enhetstester (testerna anropar hooks:na direkt) maskerade hålet.
3. **Aktualitetsproblemet:** samma granskning visade att flera hooks var
   *superseded* av det nyare `motor2-scanner`-systemet — ett verktyg som
   jämför generationer (hooks vs scanner) hade svarat på "vad ersatte vad"
   utan manuell grävning.

Tre konsumenter, samma verktyg:

| Konsument | Läge | Vinst |
|---|---|---|
| **Agent** (MCP) | Ett anrop i stället för grep-kedjor | Små modeller slipper orkestrering (felfrekvent + kontext-svältande); stora modeller slipper matching-problemet (rätt rad i brus) |
| **Systemet** (organ/flow) | Periodisk körning + deploy-grind | Nya fynd postas; grinden nekar med orsak — promotions-pipelinen: fynd → lint → grind |
| **Användaren** (CLI) | Prosa-rapport | "Fina presentationer" — metadata, förklaringar, uppställningstext |

**Kognitiv förstärkning genom redovisning:** verktygets värde för modellen
ligger inte bara i att den hittar hålen utan i *hur* den redovisar dem. Ett
förklarat fynd (metadata + varför + felklass + ankare) kan modellen resonera
över direkt; en rå grep-dump måste modellen först tillverka förståelse av.
Verktyget tillverkar förståelsen, modellen gör omdömet.

---

## 2. Designprinciper

1. **Index = fakta, prosa = claims.** Hålen bor i sömmarna mellan dem och som
   avvikelser inom fakta själv. Varje verktyg äger EN sömnadstyp.
2. **En scan, ett syfte.** Inga "gör allt"-verktyg. `holscan` är doctor-mönstret
   (sammanfattning över familjer), inte en jättescan.
3. **Determinism.** Samma rev + samma input → samma utdata. Det är det som gör
   utdata jämförbar över modeller (stora och små ser SAMMA fakta) och gör fynd
   till lint-material.
4. **Ärlig signatur-gräns.** Index ser strukturnivån (signaturer, listningar) —
   inte funktionskroppar. Verktygen påstår ALDRIG beteendehål ("loggar framgång
   utan effekt" kan bara körning bevisa). Gränsen ska stå i varje rapport.
5. **Test/produktion-åtskillnad som hård regel.** Referenser inuti
   `#[cfg(test)]`/tests-filer räknas inte som kopplings-bevis (live-läran:
   extensions-tests anropade hooks direkt).
6. **Ankare i allt.** Varje fynd bär `fil:rad` + rev. Ett fynd utan ankare är
   en gissning.

---

## 3. Verktygen

### 3.1 `claim_scan` — påståenden mot verklighet

- **Syfte:** Kors-joina prosa-lagrets claims mot index-listingen.
- **Claims-källor:** doc-headers i `lib.rs`/`mod.rs` (default), intro-block i
  indexet (default), valfria styrdokument via `--claims <fil…>` (UPPSLAG-rad
  som namnger test-fn, instructions-filer som räknar verktyg, m.m.).
- **Output:** *Lögn-claims* (namnger fn/typ/test som saknas), *odokumenterad
  yta* (pub-symbol som ingen prosa nämner), *stale referenser* (raden ändrats
  sedan claimet skrevs).
- **Motiverande fall:** "Fork/merge/list/switch"-ljugandet (live 2026-11-02).
- **Konsumenter:** lint-kandidat (tvinga som `integration_mcp_instructions_listed`-
  mönstret), deploy-grind, agent-orientering inför arbete.
- **Prototyp-status:** i linje med Motor2:s kontraktstest-mönster — generaliserar
  det till alla claims-källor.

### 3.2 `island_scan` — pub-ytor utan koppling

- **Syfte:** Hitta byggd-men-aldrig-ihopkopplad struktur (§3/§10/§20-klassen).
- **Population:** alla `pub fn`/`pub struct`/`pub trait` ur indexet.
- **Kopplings-bevis:** referenser utanför definierande fil, test-filer EXKLUDERADE;
  trait-metoder utan call-sites utanför sin impl; fält skrivna men aldrig lästa
  (begränsad grep per kandidat — index ger populationen, grepen ger beviset).
- **Output:** rankad ö-lista: "0 externa referenser utanför tester", med
  metod-ytans storlek som prioritetssignal (en Manager med 8 metoder utan
  konsument väger tyngre än en helper).
- **Motiverande fall:** extensions-dispatch-hålet (live 2026-11-02) — 6 verktygs-
  anrop från population till verifierat hål.
- **Gräns:** kan inte se "anropad men resultat kastat" (kroppsnivå) — rapporteras
  som "koppling okänd, kropp ej analyserad" när det gäller.

### 3.3 `manifest_check` — populations mot manifest

- **Syfte:** Varje "registreringspunkt" kollas i båda riktningarna.
- **Manifest-typer:** rout-tabell ↔ handler-fn:er, MCP-instructions-tabell ↔
  registrerade verktyg, sensor-register, export-listor, test-health-bitlista.
- **Output:** oregistrerad medlem (handler utan route), manifest-post utan
  medlem (route till saknad fn), räknar-drift ("37 verktyg" men 36 listade).
- **Referens:** generaliserar Motor2:s `integration_mcp_instructions_listed`-
  kontraktstest till alla manifest-typer.

### 3.4 `sibling_diff` — syskon-asymmetri

- **Syfte:** Hitta halvlevererad design genom att jämföra syskon-moduler med
  liknande roll (två Managers, två adapters).
- **Output:** metod-set-diff per par ("A har X, B saknar X"), med confidence
  (namnlikhet + roll-heuristik). Fångar §51.10-klassen: resolver utan
  provisionering, adapter utan cleanup-väg.
- **Gräns:** heuristisk — presenteras som kandidater att granska, inte sanning.

### 3.5 `scar_scan` — ärr och död config

- **Syfte:** Hitta lapparnas fotspår och §19-död config.
- **Signaler:** versioned names (`_v2`, `_new`, `_fixed`, `_legacy`, `_old`),
  config-structs/fält utan läsare utanför definition/serde.
- **Live-erfarenhet (Motor2 2026-11-02):** lågt utbyte i signatur-namn (ärr bor
  i kommentarer där) — metoden är billig, avkastningen varierar per repo. Skal
  rapportera utbytet ärligt, inte som alarm.

### 3.6 `drift_report` — strukturdynamik över tid

- **Syfte:** Indexet är rev-stämplat — diff mot föregående rev visar vad som
  rör sig i strukturen.
- **Output:** försvunna symboler som fortfarande claims i prosa (claim_scan
  för tidsskiktet), monoton-växande filer (nästa delning), signatur-churn
  (instabilitet), nya ö-ar sedan senaste rev.

### 3.7 `holscan` — doctor-mönstret

- **Syfte:** ETT anrops-orientering: kör alla familjer, rapportera hälsa per
  familj (🟢/🟡/🔴) + top-fynd + jämförelse mot senaste körningen.
- **Konsument:** system-organet (periodisk körning → post vid nya fynd),
  deploy-grinden (välj familjer), agentens session-start.
- **Referens:** GTW:s befintliga `doctor` — `holscan` är dess struktursömnads-
  syskon, inte en ersättare.

---

## 4. Gemensam output-standard

Varje verktyg rapporterar i tre lager — det Är den kognitiva förstärkningen:

```
## Sammanfattning
<antal fynd per familj · värsta fyndet i en mening · rev + tidsstämpel>

## Fynd (rankade)
### <titel med siffror>                        ← uppställningstext, §50-stil
- metadata: rev | familj | severity | confidence | ankare (fil:rad)
- felklass: <t.ex. "claim utan faktum", "byggd utan koppling">
- varför: <1–3 meningar — VAD sömnaden är och VARFÖR det är ett hål>
- förslag: <nästa steg + vem som bör reagera>
- uppställningstext: <klipp-och-klistra för rapport/handover>

## Appendix
<rådata: population, join-tabeller, exkluderade kandidater med orsak>
```

- **`--json`:** maskinläge för system/flow — samma data, inga förklaringar.
- **Språk:** identifierare/JSON-nycklar engelska (§47); förklarande prosa
  följer konsumentens språk.
- **Uppställningstext:** alltid självförklarande (namnger vad det handlar om
  med NAMN + SIFFROR, aldrig bara intern-id) — klar för handovern eller
  VERIFIERING-raden utan omskrivning.

**Exempel (förkortat) — extensions-fallet som det hade setts ut:**

> ### Extensions-hooks: 4 registrerade, 0 dispatchade
> - metadata: `b513cad` | island_scan | severity HIGH | confidence HIGH
>   | anchor: `bridge/mod.rs:2018` (write), `registry.rs:96–172` (pop.)
> - felklass: byggd utan koppling (§10-familjen); gröna tester maskerar
>   (testerna anropar hooks direkt — produktion: 0 call-sites)
> - varför: `self.extensions` tilldelas i `init_extensions()` (H3-fixen gav
>   konstruktören en anropare) men fältet läses aldrig; `run_*`-fn:erna har
>   ingen dispatch i tool-loopen. Loggen "initialiserat" rapporterar framgång
>   utan effekt.
> - förslag: dispatch-punkt i tool-loopen ELLER arkivering per hook med
>   motivering (aktualitetsgranskning krävs — se TP1).
> - uppställningstext: "Extensions-systemet (5 hooks) konstrueras och
>   registreras i produktion men dispatchas aldrig — 0 av 4 hooks har någon
>   call-site utanför sin egen crate."

---

## 5. Integration i GTW

- **CLI:** nya subcommands i code-index-familjen — `gtw claim_scan <root>
  [--claims fil…]`, `gtw island_scan <root>`, `gtw manifest_check <root>
  --manifest routes|mcp|sensors`, `gtw sibling_diff <root> [--pair a,b]`,
  `gtw scar_scan <root>`, `gtw drift_report <root> [--against rev]`,
  `gtw holscan <root> [--families …]`. Alla med `--json`.
- **MCP:** exponeras i verktygstabellen; **`GTW_INSTRUCTIONS.md` uppdateras i
  SAMMA ändring** (Motor2:s kontraktstest `integration_mcp_instructions_listed`
  kräver paritet — instruktions-tabellen ljuger annars, samma klass av hål som
  verktygen själva hittar).
- **Relation till befintliga verktyg:** `lint`/`add_rule` (claim_scan-fynd kan
  pinnas som regler = parsed-tier-promotion), `doctor` (holscan ansluter som
  structur-sektion), `analyze`/`sense` (de hittar kod; scan-verktygen hittar
  hålen mellan kod och påståenden).
- **Lagring:** läser indexet (cache/JSON — se `CODE_INDEX_SQLITE_DESIGN.md`);
  evidens-greps begränsas per kandidat (population ur index, bevis ur grep —
  aldrig repo-bred blind grep per verktyg).
- **Stora repos:** familj-val, `--file <subset>`, inkrementell drift via
  rev-stämplade index-diffs.

---

## 6. Prioritering (förslag)

1. `claim_scan` — billigast, bevisat värde (live-fynd dag 1), lint-möjlig direkt
2. `island_scan` — fann extensions-hålet live; hög effekt per anrop
3. `manifest_check` — generaliserar ett redan beprövat mönster
4. `holscan` — när ≥2 familjer finns (doctor-mönstret lönar sig först då)
5. `drift_report`, `sibling_diff`, `scar_scan` — efter de fyra

---

## 7. Triage-stöd — dömma utan arkitekts-IQ

Användarens korrektion (2026-11-02): "höja intelligens-förmåga" — intelligens
är byggmaterialet, inte måttet. Vi lägger intelligens en gång i verktyg;
verktygen gör sedan det som annars krävde intelligens — för varje modell,
inklusive 9B. Därför är "omdöme om fynd" inte modell-begränsat i sin helhet:
omdömet delas i tre delfrågor, och två av dem kapas i verktyget.

| Omdömets delfråga | Stödet (intelligens kapat i verktyget) | Kvar för modellen |
|---|---|---|
| *Vad ska åtgärdas?* | Prioritetspoäng beräknad: severity + spridradie (island_scan **vet** konsument-antal) + testtäckning + exponering (pub/intern) | Granska den beräknade prioriteten |
| *I vilken ordning?* | Beroendegraf mellan fynd-symboler → föreslagen ordning + "fynd 3,5 troligen moot efter fix 1" (§29 domino som algoritm) | Sänka/höja med motivering |
| *Design-implikationer?* | Beslutsrum förberäknat: kända åtgärdsvägar per hålklass med trade-offs (koppla in / radera / arkivera med motivering); precedens ur minnet ("denna klass triage:ats tidigare — 80 % raderad"); beslutskort ETT i taget med tvångsval (mönstret `edit_ask`) | Välja ur förklarad meny; flagga osäkerhet |

**Motgiften mot godkänna-stämpel — mekanisk gradering av motiveringar:**
modellen motiverar ("A före B, A är uppströms") → verktyget kontrollerar mot
grafen och studsar tillbaka fel motiveringar med bevis. Efter fix: om-scan
verifierar att påstådd effekt inträffade. Ytliga omdömen fångas av återkopplings-
loopen i stället för att passera tyst (§51.5-motgiftet).

**Eskalering som första-klassens verdict:** osäkerhet är ett giltigt svar —
"ovanför min löneklass → ASK operatören" är rätt utgång för en 9B när fyndet
kräver arkitektbedömning (§34.6-mönstret). Den ärliga takten: triage-tekniker
blir små modeller exellenta på; arkitekt-slutbedömningen (nytt organ vs radera)
förblir operatör/stor modell — korrekt arbetsfördelning, inte begränsning.

**Verktygspåverkan:** scan-fynden utökas med beräknade fält (`priority_score`,
`fix_order`, `moot_after`, `remediation_options`, `precedent_links`) + ett
interaktionsläge `triage` (beslutskort, tvångsval, motiveringsgradering,
om-scan-verifiering). Mätbarhet: ablation rå-JSON vs redovisning + triage-stöd,
på sådda-hål-uppgifter med varianter (samma scaffold, nytt innehåll) — Pass^3,
kontextförbrukning, avspårningar, falsk-positiv-rapportering.

---

## 8. Öppna frågor

1. **Claims-källor som default:** doc-headers + intro-block alltid — ska
   styrdokument (AGENTS/UPPSLAG/FORVANTAN-namngivna symboler) vara opt-in
   per repo eller config-styrt?
2. **Internt dokumentationsformat som spec-källa:** Motor2s DokubaseValidator-
   arv (spec-gate före kod) vill peka på Motor2:s interna format — det är ett
   Motor2-beslut, men pekar på en framtida GTW-scan (`spec_scan`: täcker
   dokumentationslagret target-området?) — värd en egen diskussion.
3. **Var rapporteras fynd i Motor2** — postlådan (system-körning), VERIFIERING
   (agent-körning), WebUI (användarkörning)? Alla tre är legitima; verktyget
   ska bara redovisa, konsumenten bokför.
4. **Falsk-positiv-policy:** island_scan:s "ö" är misstänkt, inte domd —
   hur märks confidence och vad krävs för att en ö ska eskaleras till lint?

---

*Levande dokument — uppdateras när verktygen designas/byggs. Varifrån:
Motor2-session 2026-11-02 (code-index-hålscan-idea + extensions-fyndet),
minnesservern: `Motor2:idea:code-index-holscan`, `Motor2:bug:extensions-hooks-aldrig-dispatchade`.*

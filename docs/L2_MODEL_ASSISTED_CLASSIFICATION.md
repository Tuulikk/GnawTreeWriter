# L2: modellassisterad klassificering — intelligens lagrad i Code Index

**Datum:** 2026-11-19 · **Status:** spec, ingen kodändring gjord (infrastrukturen
finns redan — se §2) · **Ägare:** Tuulikk

Frågan den svarar på (ägarens ord): *"använda mindre modell än bge-base för delar
av indexering och hanteringsjobbet … modellerna kan inte läsa av varandras
arbete, så det skulle handla om applicering som synliga, bestående delar i Code
Index."* Samt: *"förutom snabbhet också bra täckning och på något sätt bra stöd
och intelligens lagrad i databas och funktion."*

---

## 0. Sammanfattning

1. **Vektorlagret (L3) är stängt** för en andra modell: en vektor från modell A
   är ogeometriskt jämförbar med en från modell B. `AiManager::index_model()` är
   den enda ägaren av vilken embedder som gäller, och storen lagrar
   `model_name` + `dimension` för att kunna vägra.
2. **L2 är öppet** — men L2 har idag **ingen modell alls**. Det är disciplinerat
   (extraherat, inte gissat) och det är där täckningsgapet sitter: **82 % av
   upplösta-missar är `unresolved_plain_name`**, en klass som kräver
   typinferens, inte en språkmodell.
3. **Den rätta platsen för en mindre modell är inte L2:s sanning utan ett
   CLAIM-lager ovanpå den** — och det lagret finns redan: `prose_annotations`
   med `producer`, `model`, `anchor_hash`, `status`, `invalid_reason`.
   **Mätt 2026-11-19: 0 rader, 0 anropare.** Skrivaren `upsert_prose` finns,
   men ingen producerar.
4. Därför är den här specen inte "bygg ett modellager". Den är **"koppla in en
   producent i en tabell som redan är designad för det"** — med de regler som
   gör att modellens påståenden aldrig smittar av sig som fakta.

---

## 1. Varför L3 är stängt (mätt, inte antaget)

```rust
// ai_manager.rs — en ägare för "vilken embedder gäller"
pub fn index_model() -> AiModel { AiModel::Bge }
```

Storen bär `model_info { model_name: "bge-base-en-v1.5", dimension: 768 }`, och
`project_indexer` läser den för att upptäcka mismatch. Att fylla på med en
mindre modells vektorer skulle kräva **ett andra vektorrum** med egen sökning
per modell — två svar på "var ligger X?", vilket är exakt den sjuka repot jagar
(`file_hash`, `ai status`, två procent om samma sak).

**Slutsats: en mindre modell får inte skriva vektorer i den befintliga storen.**
Om L3 ska ha flera modeller blir det en separat `embeddings_<model>`-yta med
egen recall-mätning — och det är en annan, dyrare fråga (§7).

---

## 2. Vad som REDAN finns (mätt 2026-11-19)

### 2.1 `prose_annotations` — claim-lagret, byggt men tomt

```sql
CREATE TABLE prose_annotations (
    id, anchor_kind, anchor_key, anchor_hash, anchor_name,
    body,                 -- the CLAIM text
    producer,             -- who said it: model id, agent, tool
    model,                -- optional model note
    language, revision_id, generated_at,
    status,               -- fresh | stale | invalid
    invalid_reason, suggested_key,
    UNIQUE (anchor_kind, anchor_key, producer, language)
);
```

Skrivaren `upsert_prose(conn, anchor_kind, anchor_key, anchor_hash, anchor_name,
body, producer, model, language, revision_id)` finns i `code_index_db.rs:873`.

Mätt i den levande storen:

| Mätning | Värde |
|---|---|
| `prose_annotations` rader | **0** |
| anropare av `upsert_prose` i `src/` | **0** |
| `sweep_prose_freshness` (status-svep) | finns, körd efter varje revision |
| `code-index --coverage` prose-sektion | finns (`fresh/stale/invalid`) |

Infrastrukturen är alltså **komplett på läs-/livscykelsidan** och **helt utan
producent**. Det är samma mönster som `island_scan` fångade: byggt, testat,
oanvänt.

### 2.2 Extraktorn producerar redan `producer`-formad kunskap — utan fältet

`extractor_version` ligger i `meta` (en version för **all** extraktion). Men
`entities`/`relations`/`symbols` bär ingen radproveniens. Man kan inte se om en
kant kom från träd-sitter-extraktion eller från en efterbearbetning.

---

## 3. Täckningsgapet, mätt — och vad en modell INTE löser

`code-index --coverage` i detta repo (2026-11-19):

| relation_type | extraherade | upplösta | andel |
|---|---|---|---|
| `calls` | 25 414 | 3 257 | 12,8 % |
| `uses` | 2 099 | 253 | 12,1 % |
| `imports` | 488 | 43 | **8,8 %** |
| `implements` | 87 | 35 | 40,2 % |

Målklasser:

| klass | antal | vad den kräver |
|---|---|---|
| `unresolved_plain_name` | **19 758** | **typinferens** — inte en språkmodell |
| `unresolved_macro` | 4 313 | makroexpansion |
| `resolved` | 3 588 | — |
| `unresolved_qualified_path` | 429 | modulsökvägsupplösning |

`calls`-lokal upplösning, uppdelad: `external: 13 582` (std/prelude — kan inte
lösas utan att indexera std), `ambiguous: 2 350`, `needs_receiver: 1 912`,
`resolved: 3 257`.

**Konsekvensen för planen:** en mindre språkmodell höjer **inte**
`resolution_rate` nämnvärt. 19 758 `plain_name` kräver att man vet `x`s typ.
Det som faktiskt kan bli bättre är de **2 350 ambivalenta** namnen, där kontext
kan skilja `len` (Vec) från `len` (str) — men repot har redan en vakt mot att
gissa där (`is_method_entity_id`, 1 840-fabricerade-kanter-läxan).

Så: **modellen ska inte lösa kanter. Den ska märka, förklara och rangordna dem.**

---

## 4. Formen: claim ovanpå fakta, aldrig i fakta

Principen, som redan är kodad i tabellens kommentar: *"the index owns only the
FACTS about those claims (anchor + hash + status); prose never gates anything."*

| Lager | Ägs av | Exempel | Får styra ett svar? |
|---|---|---|---|
| **L1** AST | parser | noder, nodvägar | Ja (grunden) |
| **L2** graf | extraktor | `calls`, `imports`, `entities` | Ja, med `graph_coverage`-nämnare |
| **claim** | producent (modell/agent/verktyg) | "den här filen gör X", "kanten är troligen `Vec::len`" | **Nej — visas, aldrig som fakta** |
| **L3** vektorer | en embedder | likhet | Ja, inom sitt eget rum |

Tre regler gör claims ofarliga:

1. **Ankare + hash.** Varje claim binds till `(anchor_kind, anchor_key,
   anchor_hash)`. När ankaret rör sig blir raden `stale`, när det försvinner
   `invalid` — deterministiskt, utan modell (§2.1:s svep finns).
2. **`producer` är obligatorisk och synlig.** Ett svar som blandar extraherad
   fakta och modellpåståenden ska kunna skilja dem på fältet, alltid.
3. **Claims får aldrig mata grafen.** Ingen `prose_annotations`-rad får bli en
   `relations`-rad. Det är samma gräns som `capabilities` redan drar för L4.

---

## 5. Vad en mindre modell ska göra (tre nivåer, billigast först)

### Nivå 1 — deskriptiv berikning (ingen risk)

Producera `prose_annotations` för filer/noder: vad gör den, vad är ingången,
vilka antaganden. Ankare = filens hash. `producer = "lfm25"`, `model` fyllt.
Läsare: `explain` kan citera en färsk claim i stället för att generera om, och
`code-index query` kan visa den med `status` + `producer`.

**Vinst:** täckning (alla filer, inte bara de någon frågat om) och att svaret är
beständigt. **Kostnad:** en bakgrundskörning, cachelagrad per `anchor_hash`.

### Nivå 2 — klassificering av det ambivalenta (kräver grind)

För de `ambiguous: 2 350`-fallen: låt modellen föreslå vilken `to_id` som avses,
**som claim** (`producer = "model:classify"`), aldrig som `relations.to_id`.
Ett `code-index query kind=relations` kan visa båda sidor: `to_id` (fakta) och
`claim` (förslag). En grind krävs: claim får bara presenteras när
`graph_coverage`-nämnaren är utskriven, och aldrig räknas in i `impact`.

**Risk:** exakt 1 840-fabricerade-kanter-läxan, i ny skrud. Därför nivå 2 efter
nivå 1, med mätning på facit.

### Nivå 3 — underhåll (den du efterfrågade: "hanteringsjobbet")

`prose_annotations` bär redan `status`. En modell kan:
- **rangordna** vad som bör beskrivas (orörda filer, hög `island`-grad),
- **upptäcka drift**: claim som blivit `stale` men där ingen ny skrivits,
- **föreslå** `suggested_key` när ett ankare bytt namn.

Allt detta är **förslag i tabellen**, aldrig handlingar. Repot har redan formen
(`invalid_reason`, `suggested_key`).

---

## 6. Täckning som mätbar storhet (M7: nämnaren utskriven)

Idag mäter `--coverage` grafens upplösning. Med claims tillkommer en andra axel,
och de får **aldrig slås ihop**:

| Mått | Fråga | Nämnare |
|---|---|---|
| `graph_coverage` | hur mycket av grafen är upplöst | extraherade kanter |
| **`claim_coverage`** | hur mycket av koden har en färsk claim | filer/noder med `status = fresh` |
| `claim_freshness` | hur många claims är `stale`/`invalid` | alla claims |

En fil kan ha 0 % `graph_coverage` för sina kanter och 100 % `claim_coverage`
— och det är **inte en motsägelse**, det är två frågor. Samma läxa som
två-procent-fyndet.

---

## 7. Vad som INTE föreslås

- **En mindre embedder i den befintliga L3-storen.** Se §1 — två vektorrum.
- **Claims som kant-källa.** Se §4 regel 3.
- **Modellen som resolver.** 19 758 `plain_name` kräver typinferens
  (roadmap, LSP-klass), inte en språkmodell. Att låta en modell "lösa" dem är
  att byta ut en ärlig 12,8 % mot en trovärdig lögn.
- **Ett nytt lagringssystem.** `prose_annotations` finns; att bygga en femte
  indexfamilj är uttryckligen förbjudet av konsolideringsregeln.

---

## 8. Minsta första skiva (mätbar, självständigt levererbar)

1. **En producent**: `ai describe <path>` (eller MCP `describe`) som skriver
   `upsert_prose(anchor_kind="file", anchor_key=path, anchor_hash=<files.hash>,
   body=<modellens text>, producer="lfm25", model=…, language=…, revision_id=…)`.
2. **En läsare**: `code-index query` visar claims med `status` + `producer`
   bredvid fakta — aldrig i stället för.
3. **Ett mått**: `--coverage` får `claim_coverage` + `claim_freshness`, med
   nämnare.
4. **Ett bevis åt båda håll**: en claim blir `stale` när filen ändras (hash),
   `invalid` när filen försvinner — och en modell som inte finns ger ett
   högljutt fel, aldrig en tyst tom tabell.

Steg 4 är det som gör skivan till en *mätning* i stället för en yta: tabellen
är idag tom, så "0 claims" ser friskt ut (M14-klassen). En tom
`prose_annotations` **måste** skiljas från "ingen producent finns".

---

## 9. Öppna frågor (ägarbeslut)

1. **Vilken modell för nivå 1?** Den inbyggda (LFM2.5, CPU, 81 s för en
   `explain`) eller en mindre? En mindre modell kan bara användas här om den
   skriver TEXT (vilket claims är) — den är alltså frikopplad från L3-frågan.
2. **Körningstid:** bakgrund (som `index_project`) eller on-demand? Claims är
   billiga att läsa och dyra att producera — bakgrund passar, men kräver
   `revision_id`-bindning (§8 steg 1).
3. **Ska `claim_coverage` vara en grind i CI?** En tom tabell som ser grön ut är
   precis vad `claim_scan`-läxan varnar för.
4. **Nivå 2 (klassificering) — alls?** Den rör den enda kant repot aktivt
   skyddar mot att fabricera. Om ja: vilket facit mäts den mot?

---

*Skapad 2026-11-19 (session med DeepSeek V4.1 Flash) efter ägardialog om L2.
Alla siffror är mätta i denna arbetskatalog med `code-index --coverage` och
`gnawdriver db-surf` samma dag — inga påståenden utan facit (M1–M14).*

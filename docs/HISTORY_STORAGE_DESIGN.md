# Historiklagring: backuper, store och onion skin

**Datum:** 2026-11-19 · **Status:** mätning + förslag (§1–7). **§8 tillkommit
samma dag: ägarbeslut om målbilden (temporal store, rest-permanens,
ankar-gitter, retention) — §8 ersätter §4 som stegordning.** Ingen kodändring
gjord utöver de som redan redovisas.
**Föregångare:** `docs/BACKUP_STORAGE_AND_TIME_TRAVEL.md` (2026-11-12) — den mätte
backuperna och genomförde steg 1 (`tree` bort). Det här dokumentet täcker det
som *inte* täcktes där: **storens historik, `before_hash`-gapet och vad onion
skin faktiskt kan leverera.**

---

## 0. Utgångspunkten: ett ifrågasatt påstående

Ägarens invändning (2026-11-19):

> "jag ser inte hur code index verkligen kan uppfylla kravet på onion skin om
> inte historik fullt upp sparas"

Och vidare:

> "backupen borde vara knuten till code index db i stället"
> "som git, med förändringar, men ast-baserat … allt i db med viss decay, prune"

Alla tre invändningarna prövas nedan mot mätningar. Kortversionen:

1. Onion **kan inte** rekonstruera idag — inte som approximation, utan
   **dataexistentiellt**. `before_hash` är `NULL` i 104 287 av 104 287 rader.
2. Backup och store är **två separata historier** som delar ordet "historik".
   Att knyta dem ihop är rätt, men kräver först en gemensam hash.
3. En git-liknande modell är rätt — men **utan** AST-deltan. Siffrorna visar att
   det inte behövs.

---

## 1. Mätningen

Körbar dokumentation: `examples/measure_history.rs`,
`examples/measure_history2.rs`, `examples/measure_backup_content.rs`,
`examples/measure_parse_cost.rs` (alla read-only).

### 1.1 Backuperna: v1 vs v2

`create_backup` (`src/core/mod.rs`) skriver `gtw-backup-v2` sedan 2026-11-12 —
utan `tree`. Mätt i arbetskatalogen 2026-11-19:

| Format | Antal | Summa | Snitt per backup |
|---|---|---|---|
| **v1** (med `tree`) | 980 | **16 586 227 350 B (16,6 GB)** | 16,9 MB |
| **v2** (utan `tree`) | 167 | **13 584 942 B (13,6 MB)** | **81 KB** |
| | | **1 221× mindre per backup** | |

**Steg 1 är alltså redan levererat.** Det som återstår för backuperna är
**arvet**: 980 v1-filer på 16,6 GB som aldrig migrerats.

### 1.2 Vad en v1-backup innehöll

Mätt på `cli.rs_backup_20261111_084515_713.json` (46 MB):

| Del | Storlek | Andel |
|---|---|---|
| `source_code` | **277 845 B** | 0,6 % |
| `tree` (AST som JSON) | **14 208 398 B** | 30 % |
| summa av nodernas `content` | 3 072 577 B | 6 % |
| JSON-struktur/escaping runt omkring | ~33 MB | 69 % |
| **filen totalt** | **47 902 739 B** | 100 % |

AST:n var **51× större än källkoden den beskrev**, och nodernas `content` bar
**11,1× källkoden** i sig själv (varje nod bär sin delträdstext).

### 1.3 Är AST:n värd att lagra? — mätt

`examples/measure_parse_cost.rs` på `src/cli.rs` (309 348 B, 68 026 noder):

```
parse time: best 195.19 ms, avg 207.71 ms over 10 runs
```

| | Lagra AST | Parsa om |
|---|---|---|
| Disk per version | 14 MB | **0 B** |
| Tid per restore | ~0 ms | **195 ms** |
| 483 versioner | **6,8 GB** | **0 B** |

195 ms är under uppfattningströskeln. Att betala gigabyte för att spara den
tiden är att lagra en **cache som om den vore data**.

Dessutom: en sparad AST är **fryst**. `node_path` (`0.1.3`) är positionsberoende
och kan peka fel om grammatikversionen ändras. En AST i historiken påstår vad en
*dåtidig* parser sa medan läsaren är dagens — samma felklass som `mamba3`-katalogen
utan loader (data som ser ut som kunskap men inte är det).

### 1.4 Storens historik: det nya redundansproblemet

Backuperna är fixade. **Storen är inte.** Mätt mot `.gnawtreewriter_ai/code_index.db`:

| Fynd | Värde |
|---|---|
| `index_revisions` | **559** (alla `completed`) |
| `file_versions` rader | **104 287** |
| Distinkta `after_hash` | **479** |
| **`before_hash` satt** | **0 av 104 287** |
| Samma `after_hash` inspelad | upp till **1 118 gånger** |
| Distinkta versioner per riktig sökväg | **483** |

Per fil (riktig sökväg, inte basename):

| Fil | Distinkta versioner |
|---|---|
| `src/cli.rs` | 42 |
| `src/mcp/mod.rs` | 37 |
| `src/core/mod.rs` | 23 |
| `src/core/index_relations.rs` | 21 |

**99,5 % av `file_versions` är redundant**: 104 287 rader lagrar 479 unika
tillstånd. Varje revision skriver om samma `after_hash` för varje orörd fil
(559 rader för en fil som aldrig ändrats).

### 1.5 Onion skin: kan den rekonstruera?

`code_graph.rs:onion` rekonstruerar **bakåt från deltan**:

```rust
let mut count = now;
for (rid, rev, m) in by_rev.iter() {
    if let Some((_, _, after)) = m.get(&key) { count = *after; }
    walk.push((*rid, rev.clone(), count, kind));
}
```

Den börjar på live-värdet och rullar bakåt genom `count_after` per revision.
Giltigheten hänger på att varje delta bär sitt före-tillstånd. Mätt i storen:

| `relation_changes` | Värde |
|---|---|
| rader | 1 954 |
| `change_kind = added` | 439 |
| `change_kind = removed` | 155 |
| **`change_kind = modified`** | **0** |
| `count_before >= 1` | **439** (22 %) |
| `count_before = 0` | **1 515** (78 %) |
| `before_json` satt | 1 954 (fältet finns) |
| `entity_changes` | **0 rader** |

**Slutsats: onion kan inte uppfylla sitt namn.** Den kan visa *att* en kant dök
upp vid revision N, men inte *hur många* den hade före N+1 — för `count_before`
är 0 i 78 % av fallen och `modified` existerar inte som klass. 155 `removed`-rader
bär heller ingen historik om vad som fanns innan de försvann.

Detta är **inte en bugg i `onion`** — koden räknar rätt utifrån det den får. Det
är en **databrist**: det finns ingen baslinja att räkna bakåt från.

---

## 2. Slutsats per invändning

### 2.1 "Backupen borde vara knuten till code index db"

**Ja, men de är i dag oberoende system** som inte kan knytas utan förarbete:

| System | Var | Innehåll | Kan återställa kod? |
|---|---|---|---|
| Undo/redo | `.gnawtreewriter_backups/` | källkod (v2) / källkod+AST (v1) | **Ja** |
| Transaktionslogg | `.gnawtreewriter_session.json` | metadata + `before_hash`/`after_hash` | Nej (pekar på backup) |
| Code-index-historik | `.gnawtreewriter_ai/code_index.db` | **bara hashar** + aggregat | **Nej** |

**Blockerare 1 — två olika hashfunktioner:**

```rust
// transaction_log.rs:660 (transaktioner, backuper)
pub fn calculate_content_hash(content: &str) -> String {
    let mut hasher = DefaultHasher::new();   // SipHash-1-3, 16 hex
    ...
}

// code_index_db.rs:1317 (storen)
format!("{:x}", md5::compute(&bytes))        // MD5, 32 hex
```

Uppmätt i loggen: `"before_hash": "73cf152d801cac9c"` (16 hex).
Storen: `"after_hash": "f0c5281d576578fc27d5b8eafd148da1"` (32 hex).

`DefaultHasher` är enligt Rust-dokumentationen **inte stabil mellan
Rust-versioner**. Den dag verktygskedjan uppgraderas slutar alla `before_hash`
matcha backuperna — **tyst**. Detta måste unifieras till MD5 (samma som
`files.hash`) innan någon sammankoppling är meningsfull.

**Blockerare 2 — `before_hash` är `NULL` i `file_versions`.** Fältet finns i
schemat men skrivs aldrig. Det är därför onion är tom på före-sidan.

**Blockerare 3 — backup-filnamnen använder basename.** `mod.rs` har 460
backuper, men `src/core/mod.rs` och `src/mcp/mod.rs` heter båda `mod.rs` och
ligger i samma namnrymd. Uppslagningen kan i princip para ihop fel fil. Det är
en **korrekthetsrisk**, inte bara prestanda.

### 2.2 "Som git, men AST-baserat, med decay och prune"

**Rätt modell, fel detalj.** Mätt: 483 distinkta versioner × ~278 KB källkod
= **~134 MB okomprimerat**. AST-deltan på nodnivå behövs inte — det finns
nästan inget att spara på när basen redan är 134 MB, och git självt lagrar hela
blobbar och deltifierar först i packfiles när det lönar sig.

Modellen:

```sql
-- Content-addressed blobs: EN rad per unikt innehåll, aldrig AST.
blobs(
    hash    TEXT PRIMARY KEY,   -- MD5 av bytes (samma som files.hash)
    content BLOB NOT NULL,      -- zstd-komprimerad källkod
    size    INTEGER NOT NULL,   -- okomprimerad storlek (för doctor-måttet)
    refs    INTEGER NOT NULL    -- refcount; 0 = oanvänd, prunable
);

-- Ersätter file_versions redundans: bara faktiska ÄNDRINGAR.
file_states(
    revision_id INTEGER NOT NULL REFERENCES index_revisions(id),
    path        TEXT NOT NULL,
    blob_hash   TEXT NOT NULL REFERENCES blobs(hash),
    parse_status TEXT NOT NULL,
    PRIMARY KEY (revision_id, path)
);
```

**Migreringen är enkelriktad och producerar mindre:** `file_versions` har 479
distinkta hashar — det är 479 rader i `blobs` i stället för 104 287 i
`file_versions`. Men `file_versions` bär bara **hashar**, inte innehåll. Den
faktiska källkoden finns i backuperna.

### 2.3 Decay och prune

Git komprimerar/rensar inte automatiskt (`git gc` är explicit). Utan decay blir
16,6 GB → 160 GB. Nivåerna:

| Nivå | Vad som sparas | Motivation |
|---|---|---|
| Senaste N revisioner (förslag: 50) | full `file_states` per revision | undo, onion, `--at` måste vara exakt |
| Äldre | `file_states` endast för **ändrade** filer | orörda filer ärvs — det är där 99,5 % av redundansen sitter |
| Mycket äldre | bara `index_revisions`-metadata | revisionshistorik, ingen innehållsåterställning |
| Bortom retention | bara `git_rev` | git äger committat innehåll |

`refs = 0` i `blobs` = ingen `file_states` pekar dit = **prunable**. Prune är då
`DELETE FROM blobs WHERE refs = 0`, vilket är deterministiskt och säkert.

---

## 3. Vad som krävs för att onion ska bli sann

Onion kräver en **baslinja per revision**. Två vägar:

| Väg | Vad som sparas | Onion blir |
|---|---|---|
| **Delta (idag)** | aggregat, `count_before = 0` i 78 % | approximation som presenteras som state-serie |
| **Full `file_states` per rev** | vilken blob var gällande per fil och rev | **exakt** — varje frame är en sann state |
| **Hybrid** | full var N:te rev + deltan emellan | exakt vid snapshot, delta mellan |

Med 198 filer × 559 revisioner blir full `file_states` 110 682 rader — men med
`refs`-dedup mot 479 blobs kostar det bara **479 blobs + 110 682 små rader**
(≈ 5 MB metadata). Det är överkomligt och ger onion en sann baslinja.

**Viktigt:** att spara `count_before` korrekt i `relation_changes` är en
**förutsättning** — utan det kan inte ens en full `file_states` rädda
delta-kedjan, eftersom onion räknar kanter, inte filinnehåll.

---

## 4. Föreslagen ordning

> **Ersatt av §8 (2026-11-19, ägarbeslut).** Behållen som underlag.
> Steg 2 och steg 3 hör inte längre till ordningen — se §8.1 korrigeringarna.

Ingen del är gjord. Ordningen är vald så att varje steg är självständigt
levererbart och mätbart.

1. **Unifiera hashen.** `calculate_content_hash` → MD5 av bytes (samma som
   `files.hash`). Kräver hantering av befintliga `DefaultHasher`-värden:
   gamla 16-hex-hashar kan inte konverteras, så en `hash_scheme`-markör per
   transaktion behövs. **Förutsättning för allt annat.**

2. **`before_hash` i `file_versions`.** Skriv före-tillståndet. Det är
   ensamt nog för att onion ska kunna räkna bakåt på det som spelas in
   framöver (historiken kan inte räddas, men kan sluta ljuga).

3. **`count_before` i `relation_changes`.** Skriv det faktiska antalet före
   varje delta, samt inför `modified` som klass. Mät: andel rader med
   `count_before > 0` ska gå från 22 % → ~100 % för nya revisioner.

4. **`blobs` + `file_states` (additivt).** Nya skrivningar går till det nya
   lagret, gamla läsningar faller tillbaka på backuperna. Ingen radering.

5. **`--at <txn|rev>` som läsande resolver.** Per fil redovisas proveniens:
   `after_blob` | `before_blob` | `git | absent | not_in_store`. Aldrig tyst
   sammanblandning (jfr `from_symbol: null`-regeln).

6. **Migrera de 980 v1-backuperna** med manifest: verifiera `source_code` →
   hash → blob → **läs tillbaka och jämför** → först därefter radera
   originalet. Aldrig radering före verifierad återläsning.

7. **Decay + prune** enligt §2.3, med `refs = 0` som enda raderingsvillkor.

8. **Kontinuerligt mått** i `doctor`: `stored_bytes_per_unique_source_byte`
   med nämnaren utskriven (M7). Larm över 3. Då kan problemet inte smyga
   tillbaka.

---

## 5. Vad som INTE föreslås

- **AST i historiken.** Mätt: 51× källkoden, 195 ms att återskapa. Fryst data
  som dessutom är versionsberoende.
- **Nodnivå-deltan.** Basen är 134 MB; deltan köper lite och gör
  återställningen känsligare. (Samma slutsats som 2026-11-12-dokumentet §"Delta-lagring: nej".)
- **Att röra de 980 v1-backuperna utan manifest.** De är läsbara som de är;
  en migration utan verifierad återläsning är en risk utan vinst.

---

## 6. Öppna frågor (kräver ägarbeslut)

### 6.0 Mätfälla som kostade tid (läs denna först)

`gnawdriver db-surf --where 'before_hash exists true'` **matchar alla rader**,
även de där värdet är `null`. Verktygets egen dokumentation säger det: *"JSON-null
är ett VÄRDE"*. Ett `exists`-filter svarar alltså "fältet finns i svaret", inte
"fältet har ett värde".

Följden: proben ser ut att motbevisa gapet (`matched: 104287`). Sanningen syns
bara genom att **läsa kolumnen** (`--fields path,before_hash` → `"before_hash": null`)
eller räkna NULL-medvetet (`SUM(before_hash IS NOT NULL)` = 0, i `measure_history2`).

Detta är M7-klassen: två tal för samma sak med olika nämnare. Skriv ut
nämnaren — "exists" och "har värde" är olika frågor.

### 6.1 Sakfrågorna

1. **Hash-migreringen:** ska gamla `DefaultHasher`-transaktioner behållas som
   de är (med `hash_scheme: "siphash"`) och nya skrivas som MD5? Det gör
   historiken blandad men bevarar allt.
2. **Retention:** hur många revisioner ska ha full `file_states`? 50 är en
   gissning, inte en mätning.
3. **`git` som källa:** ska `--at` för committade revisioner läsa från git
   (`git show rev:path`) i stället för att GTW lagrar sin egen kopia? Det sparar
   disk men kräver att arbetskopian är en git-checkout.
4. **Zstd-nivå:** 2026-11-12 mätte 25–31× på `source_code`. Nivå 3 är snabb och
   ger ~4×; nivå 19 ger ~30× men är långsam vid skrivning. Vilken väger tyngst?
5. **Hur länge ska v1-backuperna finnas kvar** om migrationen inte genomförs?

---

## 7. Verifieringskommandon

```bash
# Mätningarna i detta dokument (read-only)
cargo run --example measure_history  -- .gnawtreewriter_ai/code_index.db
cargo run --example measure_history2 -- .gnawtreewriter_ai/code_index.db
cargo run --example measure_backup_content
cargo run --example measure_parse_cost -- src/cli.rs

# Läget just nu
ls -l .gnawtreewriter_backups/*.json | awk '{s+=$5} END {print s" bytes"}'
grep -l '"gtw-backup-v2"' .gnawtreewriter_backups/*.json | wc -l

# Onion-gapet — OBS: `--where 'before_hash exists true'` DUGER INTE.
# db-surf räknar exists som "fältet finns i svaret", och JSON-null är ett
# värde: den matchar alla 104 287 rader. Läs kolumnen och se null i stället:
gnawdriver db-surf --path .gnawtreewriter_ai/code_index.db \
  --table file_versions --fields path,before_hash --limit 2
#   → "before_hash": null  = gapet bekräftat (alla rader)

# Den exakta räkningen (NULL-medveten) ligger i measure_history2:
cargo run --example measure_history2 -- .gnawtreewriter_ai/code_index.db
#   → "with before_hash : 0"
```

---

## 8. Målbild: temporal store, rest-permanens och ankar-gitter (2026-11-19, ägarbeslut)

**Detta avsnitt är huvudspåret och ersätter §4 som stegordning.** Utgångspunkten
är ägarens beslut: **GTW:s historik flyttas i helhet till Code Index — det
handlar inte bara om onion.** `code_index.db` blir projektets temporal store
(graf + historik + undo), och övergången sker genom bevisgrinder, aldrig som en
switch-punkt.

### 8.1 Korrigeringar mot tidigare avsnitt (läs först)

1. **§4 steg 3 är delvis byggt — och klassnamnet stavas annorlunda.**
   `66fa00f` (2026-11-12) levererade `relation_changes`-skrivaren: riktiga
   `count_before`/`count_after`, `before_json`/`after_json` med
   resolved-räknare, och klassen **`count_changed`** (`code_index_db.rs:598`)
   — inte "modified" som §3/§4 kallar den. Mätt 2026-11-19: revision 565/566
   bär riktiga före-värden (`count_changed 3→4, 19→20`; `removed` med
   `count_before=1`). Gapet i §1.5 (78 % nollor) är **gamla rader**, inte
   avsaknad av skrivare.
2. **"Historiken kan inte räddas" (§3) gäller endast observed resolution.**
   Räknare för filnivå-historik går att återfå: backuperna ÄR fullupplösta
   punkter utspridda i mörka åldrarna, på edit-granularitet. Materialiserade
   på begäran blir de räkne-ankare, märkta `derived(current_extractor)`.
   Det som är obevekligt är resolution *som observerades då* (store + resolver
   state vid tidpunkten).
3. **§4 steg 2 (`before_hash` i `file_versions`) ströks.** Tabellen ersätts av
   `file_states` i målbilden; att skriva before-hash där är att investera i en
   deprecierad yta. (Nuvarande skrivare lägger dessutom NULL explicit och en
   rad per fil per revision — `code_index_db.rs:855` — redundansen fortgår.)

### 8.2 Grundmodellen: två oberoende axlar

Data degraderas längs två axlar. De blandas inte.

| Axel | Vad som varierar | Styr |
|---|---|---|
| **Lösningsdjup** | aggregat/räknare → kant-identitet → innehåll | vilka frågor som kan svaras |
| **Ålder** | färskt rått → gammalt degraderat | bara innehållslagret |

Rest-lagret — lösningsdjupets permanenta grund:

- `relation_changes`: count_before/after, before/after_json (count + resolved),
  resolution_status, relation_key (identiteten på kanter som *ändrades*)
- journal-tabeller: edit-händelser + undo/redo-reactions
- **Invariant: rest prunas aldrig.** Motiverad av mätning, inte sentiment:
  2 265 rader i denna store (2026-11-19) — trivialt liten för alltid.

Onion arbetar enbart på resten. Pruna allt innehåll hur gammalt som helst —
onion räknar fortfarande exakt, givet hel kedja.

### 8.3 Ankar-gittret: upplösning är fläckig, inte en åldersfunktion

Ankare = punkter med full eller hög upplösning, var som helst på tidslinjen —
inte bara vid HEAD. De förstärker båda grannskapen: en fråga rullar från
*närmaste upplösbara ankar* (bakåt via `count_before`, framåt via
`count_after`), och ett brutet span begränsas till spannet mellan ankare.

| Grad | Innehåll | Kan ge |
|---|---|---|
| A | innehåll (blob) | allt — re-extrahera → B → C |
| B | kanlista (identitet per kant) | C direkt |
| C | aggregat `{key → (count, resolved)}` | onion-aritmetik + resolution-aggregat (kB) |

Ankare uppstår **organiskt** (write-behind: varje `--at`-materialisering
lämnas kvar; backup-migreringen sådder grad-A-ankare i mörka åldrarna) och kan
**schemaläggas som beredskap med utlösningsvillkor** (se 8.6 config).
Retention: ankare är cache med refs — tas ett ankar bort degraderas djupet i
dess grannskap, aldrig aritmetiken. Undantag: ankare utan git-bakgrund (dirty)
tillhör den råa kärnan (8.6).

### 8.4 Onion: ankarbaserad med proveniens per frame

Varje frame i ett onion-`--at`-svar bär sin proveniens:

```
exact        — rest-kedjan intakt till ankar
anchored     — materialiserad via ankar M, rullat ±k
derived      — återlösad via git-innehåll, med aktuell extraktor (märkt)
unresolvable — varken kedja eller fungerande pekare; rapporteras, aldrig
               tyst approximerat ("from_symbol: null"-regeln)
```

Regler: bruten kedja → `chain_broken`, aldrig tyst rullning.
`{git_rev}`-pekare verifieras vid anrop (`git cat-file`-kontroll); en dangle
→ `unresolvable`, aldrig "0 kanter" som ser friskt ut (M14-klass).

**Git:s plats:** kompletterande källa för committat innehåll — det GTW medvetet
inte äger (hierarkin i 2026-11-12-dokumentet §3). Dirty-innehåll kan git inte
täcka: dirty-blobbar är den råa kärnan.

### 8.5 Konsolidering: fyra ytor → en store

| Idag | Målbild i store |
|---|---|
| Transaktionsjournal (`.gnawtreewriter_session.json`) | rest-tabeller (edit-nivå, permanent) |
| Backup-filer (v3-namn, `137c1d4`) | `blobs` + `file_states` (content-addressed) |
| Head (`.gnawtreewriter_head.json`) | härledd projektion (bevisat derivbar — test raderar filen, identisk projektion) |
| `relation_changes` / `entity_changes` | redan i store |

Bron finns redan i schemat: `revisions_tx_links (revision_id, transaction_id)`
(`code_index_db.rs:151`, schema v4 "phase 2"). Målbildens tidslinje: **edits =
fin kedja, revisioner = ankare/sammanfattningar**, bundna av länken.

### 8.6 Retention — ägarbeslut tagna (2026-11-19)

| Yta | Beslut |
|---|---|
| Rest (deltas, journal) | permanent — prunas aldrig |
| Committat innehåll | git-pekare räcker; inget eget lagras |
| **Dirty-innehåll** | **behåll tills tillståndet committas + golv; golv = 30 dagar (default), användarkonfigurerbart; därefter prune** |
| Ankare (materialiseringar) | cache med storlekstak; densitet konfigurerbar |

Schemalagd ankarsådd: **organisk först; schemalagd som beredskap med
utlösningsvillkor** — aktiveras endast om `max_roll_distance` överstiger
tröskeln. Startvärden i config (kod/JSON på engelska enligt språkregeln):

```yaml
history:
  dirty_retention_days: 30       # floor; dirty blobs prunable after this
  anchor_seeding_interval: null  # null = organic only; set N = every Nth revision
  max_roll_distance_alarm: 200   # doctor alarm: longest anchor-free span
```

Värdena är startpunkter som justeras utifrån hur systemet mår (8.10).

### 8.7 Utväg (export)

Historiken får inte bli en svart låda. **Tidigt:** dokumenterad
materialiseringsväg — varje rad går att skriva tillbaka till filer (det är
migreringen baklänges) — plus integrity-check i doctor. **Senare:**
`history export`/`import`-kommando vid behov. SQLite är öppet format; utvägen
ska vara dokumenterad innan målbilden kallas trovärdig.

### 8.8 Övergången: bevisgrinder, inga switch-punkter (ägarprincip)

Ägarens regel: **nya funktioner och Code Index-bitar bevisar sig bitvis; den
gamla metoden stängs av först efter bevis och tas bort ännu senare.** Mönstret
upprepas per steg:

| Fas | Vad händer | Grind till nästa fas |
|---|---|---|
| Skugga | nya vägen skriver; gamla äger läsning | data finns i båda |
| Paritetsbevis | samma operation genom båda vägarna, jämför resultat | identiskt över N verkliga operationer |
| Läs-flip | läsning byter källa; båda skriver fortfarande | flip-back = flagga, inte kodändring |
| Stäng av | gamla skrivvägen av | legacy-läsning kvar (mönster: backup-v3) |
| Rensa | gamla filer/kod tas bort | senare release, aldrig samma |

### 8.9 Stegordning (ersätter §4)

1. **Hash-unifiering** (oförändrad från §4 steg 1) — MD5 av bytes +
   `hash_scheme`-markör; `parse_backup_file` räknar båda hasharna under
   övergången. Självbevisande: att gamla undo:n fortsätter fungera ÄR
   paritetsprovet.
2. **Rest-kompletthet** — `entity_changes`-skrivare (D1-aggregat, som
   relationerna), bevisad med känt facit; invarianten "rest prunas aldrig"
   inbyggd i prune-designen.
3. **Innehåll + edits i store (skugga)** — `blobs` + `file_states` +
   journal-tabeller; dual-write; läsning styrs av flagga (DB är inte källa än).
4. **Paritetsbevis + läs-flip** — `undo` läser DB först när pariteten varit
   grön över verkligt arbete (bevisat via migrerade v1- och v2-blobbar i båda
   riktningarna). Flip-back = flagga.
5. **`--at` + ankarbaserad onion** — bevisad mot känt facit: rekonstruera ett
   känt tillstånd och jämför mot git, med proveniens per frame (8.4).
6. **Backup-migrering = ankarsådd** — samma disciplin som §4 steg 6 (manifest,
   verifiera-läs-tillbaka före radering), ny betydelse: grad-A-ankare i mörka
   åldrarna.
7. **Decay/prune** enligt 8.6 + refs=0-regeln (avd #2.3), dirty-golvet som
   enda retention-talet utöver cachetaket.
8. **doctor-mått** enligt 8.10.

### 8.10 doctor-mått (M7: nämnaren skrivs ut)

- `stored_bytes_per_unique_source_byte` (oförändrad från §4 steg 8; larm > 3)
- `anchor_density` — ankare per kedjespan + andel revisioner inom ±k från ett
  ankar
- `max_roll_distance` — längsta kedjespan utan ankar (ärlighets-horisonten: 
  längst bak som svaret fortfarande kan vara exakt);
  larmtröskel i config, startvärde 200

### 8.11 Kvar öppet (små)

- zstd-nivå (§6.1 #4) — opåverkad av §8, väntar mätning
- v1-backupernas livslängd om migration uteblir (§6.1 #5) — besvaras av när
  steg 6 körs
- paritets-N i steg 4 (föreslaget: 2 veckors verkligt arbete i detta repo)
- exakt gränssnitt för flip-flaggan (namn, default, placering)

---

*Skapad 2026-11-19 (session med DeepSeek V4.1 Flash); ägare: Tuulikk.
Alla siffror är mätta i denna arbetskatalog med kommandona i §7 — inga
påståenden utan facit (M1–M14). §8 tillkommen samma dag (session med
GLM 5.3 Flash) efter ägardialog: målbilden, retentionstalen och
bevisgrinder-principen är ägarbeslut, inte agentens förslag.*

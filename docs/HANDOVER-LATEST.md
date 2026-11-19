# Handover — 2026-11-19 (X): island_scan, insert-guard, backup-v3 — och det öppna arkitekturspåret

**Läget:** träd rent, 4 leveranser idag (`e2b0f5a` → `9210f18`), 373 lib- +
26 mcp_integration-tester gröna, fmt/clippy `-D warnings`/`code-index --check`
gröna. Ingen release taggad (0.26.1 ute). **Viktigt:** installerad binär
`~/.cargo/bin/gnawtreewriter` är 0.26.1 från i morse — **rebuilda
(`cargo install --path .`) + daemon-omstart** innan fixen från `137c1d4`
når MCP-vägen. De 1 147 gamla backuperna (16,6 GB) är orörda.

Läsordning: denna fil → `docs/HISTORY_STORAGE_DESIGN.md` (idag, mätt) →
`docs/BACKUP_STORAGE_AND_TIME_TRAVEL.md` (2026-11-12, backuperna) →
`uppslag.md` → `docs/CODE_INDEX_SCAN_TOOLS_PROPOSAL.md` §3.2.

---

## 1. Denna session — tre leveranser

### a) `island_scan` — T4 #2 (SCAN_TOOLS §3.2), verktyg #40 (`79ac0bc`)

Pub-ytor utan extern koppling. Population = pub-deklarationer under prefixet
(default `src/`); kopplingsbevis på två kanaler — namngiven edge från annan
produktionsfil ELLER verbatim ordnamnande i annan fil (subsumerar importer:
storen slår flerradiga `use`-importer och sparar aldrig `use` i funktionskropp).
Test-filer och `#[cfg(test)]` exkluderade som bevis; `cfg(test)`-verdikten läst
från **källan på storens radankare**, inte heuristik. Rankning: severity →
metod-yta (metoder hänger på impl-itemet, inte structen — mätt miss i fixturen)
→ namn+fil.

Verklig körning: 546 pub-ytor, 301 kopplade, 245 kandidater. Toppen:
`AnchorDetector` (verifierad ö med grep — bara nämnd i egen fil).

**M14 bevisat:** hermetiskt test med känd ö och känd kopplad; 4 tester; fel
population = högljudande fel, aldrig tomt-friskt. Paritet: 39 → 40 verktyg,
`integration_mcp_instructions_listed` grön.

### b) `insert_node`-guard: annoteringsblock delades aldrig (`137c1d4`, del 1)

Position 0/1/2 räknade en offset **inuti** en kommentars-/attributnod. Matt på
fyra språk, samma operation: Rust → E0753, Bash → koden ovanför shebang,
Go → build-taggen ignorerad, **JS → koden uppslukad av blockkommentaren
(valid syntax, exit 0, edit gör ingenting)**. Fix: annoteringsnod har ingen
insida → insert efter blocket; insert som landar i filens inledande
`//!`/`#![` flyttas efter det; vakterna säger det på stderr. 2 regressionstester
i `mod annotation_insert_regression`; bevisat båda riktningarna (med fixen
avstängd fångar testet shebang-brottet).

### c) Backup-v3: hash + molad sökväg i filnamnet (`137c1d4`, del 2)

| Mätning | Före | Efter |
|---|---|---|
| 2 undo, 1 108 backuper i katalogen | **3m21s** (vid 1 147) | **57 ms** |
| Två `mod.rs` i samma millisekund | andra **skrev över** första | två distinkta namn |
| Uppslagning | hashar alla filer (16,6 GB) | läser **noll** innehåll |

Format: `<molad_sökväg>_backup_<hash>_<ts>.json`, ts utan understreck
(`%Y%m%dT%H%M%S%3f`) så namnparsern kan splitta. Skrivs av **båda**
skrivvägarna (`create_backup` och `write_after_blob` — efter-blobber hade
samma kollision). Legacy-scan kvar för pre-v3; bevisat av test
`legacy_names_still_resolve_through_the_content_scan`.

**Två förbehåll, ärliga:** hashen är fortfarande `DefaultHasher` (instabil
mellan Rust-versioner — se §3 steg 1); onion förblir trasigt tills
code-index-historiken byggs (§3 steg 2–4).

---

## 2. Process — vad agenten gjorde fel och rätt

**Fel:** snubblade in i undo-problemet från island_scan-arbetet och fortsatte
in i det i stället för att återrapportera; beskrev sedan v2-formatet som
"steg 1 redan gjort" utan att säga att **uppslagningen fortfarande är O(n)**.
Agarens tillrättavisning: *"hur hamnade du ens på island scan och undo när vi
går igenom förändringar av strukturen?"* — rätt. Två gånger dessförinnan
antog jag fel sak (att `db-surf exists` betyder "har värde"; att `0` är
filroten när rotens path är `""`). M6/M7/M11 fångade alla tre — men de kostade
omgångar som en plan med första-probe-sanning hade sluppit.

**Rätt (ägarens ordning följd):** island_scan först (ordern), insert-fixen
som egen commit, undo-fixen som egen commit med båda förbehållen, och
arkitekturspåret **som dokument, inte kod** — formatändringar för historik
skrivs innan de byggs.

**Mätfälla för nästa agent (M7-klass):**
`gnawdriver db-surf --where 'before_hash exists true'` matchar **alla** rader —
`exists` betyder "fältet finns i svaret", och JSON-null är ett värde. Gapet
syns bara genom att läsa kolumnen (`--fields path,before_hash` → `"before_hash": null`)
eller NULL-medvetet (`SUM(before_hash IS NOT NULL)` = 0). Dokumenterat i
design-dokumentet §6.0.

---

## 3. Det öppna arkitekturspåret — detta är nästa sessions stora fråga

Agaren lyfte det, och mätningen bekräftar: **onion skin kan inte
rekonstruera**, inte som approximation utan dataexistentiellt:

| Gap | Bevis |
|---|---|
| `file_versions.before_hash` | NULL i **104 287 av 104 287** |
| `relation_changes.count_before ≥ 1` | 439 av 1 954 (22 %) |
| `change_kind = modified` | **0 rader** (klassen existerar inte) |
| `entity_changes` | 0 rader |
| `file_versions` redundans | 104 287 rader → **479** distinkta versioner |

**Ingång:** `docs/HISTORY_STORAGE_DESIGN.md` — mätningarna, `blobs` +
`file_states`-formen, migreringsvägen och **§6.1 öppna frågor (ägarbeslut)**
är första agendapunkten. Rekommendation från agenten: starta med **steg 1
(hash-unifiering)** eftersom den är förutsättning för allt annat och är liten —

> **Lösning (förslagen, inte genomförd):** byt
> `transaction_log::calculate_content_hash` → MD5 av bytes (storen använder
> redan `md5 = "0.8.1"`, samma som `files.hash`). Övergången utan migrering:
> `parse_backup_file` räknar **båda** hasharna (MD5 + SipHash) och matchar
> vilken transaktionen har — gamla undo:n fortsätter fungera, nya skrivs med
> MD5. Lägg `hash_scheme`-markör i transaktionen. Detta vore också steget som
> gör backup-namnets hash och framtida blob-hash till **samma** hash.

Agarens egen sortering (ordagrant): *"hash som en del av filnamn skulle kunna
vara tillfällig fix … men sen är frågan om Code Index ska vara framtiden, men
det är ett större arbete."* — och den tillfälliga fixen är **inte kastad**:
MD5-namnen är samma hash som framtida `blobs` behöver.

---

## 4. Kvar (ärligt, inget tyst)

| Post | Läge |
|---|---|
| **Binär-ålder** | installerad binär är äldre än källan → MCP får inte dagens fixar förrän rebuild + omstart |
| **1 147 v1/v2-backuper (16,6 GB)** | orörda; migrering med manifest är design-dok steg 6, **ej påbörjad** |
| **island_scan rest** | fält-nivå ("skrivet men aldrig läst") och trait-metod-klasserna är §3.2-item storen inte kan bevisa — noterade i rapportens `note`, inte byggda |
| **`undo` är processlokalt** | förut dokumenterad (2026-11-12 §7 fynd 1): två separata `undo`-anrop kan ångra samma transaktion två gånger — fortfarande olöst |
| **Onion/delta-kedjan** | `count_before` + `modified` + `entity_changes` är design-dok steg 2–3, ej påbörjade |
| **980 v1-backuper** | innehåller fortfarande AST (14 MB/st); läsbara, men är det som håller 16,6 GB på disk |
| **GTW_INSTRUCTIONS.md raden** | `island_scan` tillagd + 39→40 verktyg; språkregeln följdes (kod/JSON engelska, prosa svenska) |

---

## 5. Verifieringskommandon (M11: kör, lita inte på minnet)

```bash
git status --porcelain && git log --oneline -6   # rent, 4 leveranser
cargo test --lib 2>&1 | grep "test result"        # 373 grön
cargo test --test mcp_integration 2>&1 | grep "test result"  # 26 grön
cargo run --example measure_history2 -- .gnawtreewriter_ai/code_index.db
#   → "with before_hash : 0" = onion-gapet replikerat
bash scripts/claim-scan-gate.sh .                                  # exit 0
gnawtreewriter code-index . --check                                # grön
# undo-fixens skalning (hermetiskt):
#   cd /tmp/opencode/gtw-undo-timing && time gnawtreewriter undo --steps 2
#   → 57 ms vid 1 108 backuper (före: 3m21s vid 1147)
```

*Skapad 2026-11-19 av sessionens agent (GLM 5.3 Flash); ägare: Tuulikk.
Allt ovan är verifierat med kommandon denna session — inga påståenden utan
facit (M1–M14).*

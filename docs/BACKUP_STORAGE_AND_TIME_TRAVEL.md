# Backup-lagringen och tidsresor: mätning före migration

**Datum:** 2026-11-12 · **Status:** mätning + gap-analys. Ingen migration gjord.

Frågan som styr allt: *vad kan återskapas av någon annan, och vad kan bara GTW
veta?* Historiken över förändringar är viktigare än de exakta versionerna — git
äger innehållet, och AST:n kan parsas fram igen. Det som **inte** kan återskapas
är relationernas historia.

---

## 1. Mätningen (15,45 GiB → vad?)

Profil av alla 980 backuper i `.gnawtreewriter_backups/`:

| Post | Storlek | Andel |
|---|---|---|
| **`tree` (serialiserad AST)** | **15,31 GiB** | **99,1 %** |
| `source_code` | 0,13 GiB | 0,9 % |
| metadata + JSON-escapes | ~0,00 GiB | 0,0 % |
| **totalt** | **15,45 GiB** | |

| Mått | Värde |
|---|---|
| backuper | 980 |
| **unika källkodstillstånd** | **945 av 980** (dubbletter: 2 %) |
| unik källkod (rå) | 132,4 MiB |
| **förstoring mot unik källkod** | **119×** |
| värsta fil | `mod.rs`: 440 backuper = 10,9 GiB (99 % AST) |
| näst värsta | `cli.rs`: 75 backuper = 2,9 GiB för en 286 KB fil (**~130× per tillstånd**) |

Kompression, mätt på 4 MB AST-prov ur en `cli.rs`-backup:

| Komprimering | Ut | Ratio |
|---|---|---|
| zstd -19 | 128 663 B | **31×** |
| xz -9 | 133 064 B | 30× |
| gzip -9 | 311 769 B | 12,8× |
| zlib -9 på `source_code` (28 302 KiB) | 1 145 KiB | **25×** |

### Slutsatsen som vänder på prioriteringen

Content-addressing och dedup köper **2 %** — det finns nästan inga dubblerade
tillstånd. Problemet är inte dubbletter och inte okomprimerad JSON i första hand:

> **99,1 % av lagringen är ett fält som ingen kod läser.**

`"tree": &self.tree` skrivs på exakt ett ställe
(`GnawTreeWriter::create_backup`, `src/core/mod.rs:506`) och läses **ingenstans**
— `restore_from_backup` (`undo_redo.rs:953`) och `backup.rs:93/169` läser bara
`source_code`. Alla andra förekomster i repot är testfixtures med `{}`.

Att sluta skriva fältet är därför en **enradig** ändring med ~100× effekt, inte
en migration.

---

## 2. Vad loggen faktiskt innehåller

`Transaction` = `id, timestamp, operation, file_path, node_path, before_hash,
after_hash, description, session_id, metadata`. Alltså **identitet och metadata,
inte innehåll**. Innehållet ligger i backup-blobben:

```json
{ "file_path": …, "timestamp": …, "tree": …, "source_code": … }
```

Mätt på `.gnawtreewriter_session.json`: 983 rader = 962 `Edit` + 17 `Insert` +
1 `Delete` + 3 sessionsmarkörer, över **62 filer**. Antal innehållstransaktioner
(980) = antal backuper (980) — **exakt en backup per skrivning**.

Hasharna är innehållshashar av hela filtexten (`before_hash` från
`self.source_code`, `after_hash` från den modifierade texten), men
`calculate_content_hash` använder `DefaultHasher` (SipHash-1-3): innehållsbaserad,
men **inte stabil mellan Rust-versioner** och inte kryptografisk.

### Följden: en lucka, inte en bugg

N edit för en fil ger N before-blobar: S0 … S(N−1). Den nyaste state som *inte*
finns i blob-lagret är alltid **S_N** — den ligger på disk. Det är därför
`redo` har felvägen `"no suitable backup or previous state found"`
(`undo_redo.rs:402`): after-tillståndet materialiseras först som *nästa* editerings
before-blob.

Konsekvens för destruktiva experiment: så länge GTW själv rör filen igen finns
tillståndet. Rör ett annat verktyg filen — eller raderas blobben — är det borta,
och git har det aldrig sett (`dirty = 1`).

---

## 3. Lagringshierarkin (principen, skärpt)

| Data | Kan återskapas av | Ska GTW lagra? |
|---|---|---|
| **Relationshistorik** (vem/vad/när, resolution, count) | **ingen** | **Ja.** Den enda unika datan — och den är liten (metadata) |
| Filinnehåll i ett **commit** | git | **Nej** → pekare `{git_rev, path}` |
| Filinnehåll i **dirty**-tillstånd | ingen | **Ja**, men bara dessa |
| **AST** | parser + källtext | **Aldrig** — 99,1 % av lagringen, och versionsberoende |

Sista raden är viktigare än den ser ut: en sparad AST är inte bara överflödig,
den är **fryst**. Den skulle påstå vad en *dåtidig* parser sa medan extraktorn som
läser den är dagens. Det är samma klass av fel som `mamba3`-katalogen utan
loader: data som ser ut som kunskap men inte är det.

Mål: **15,45 GiB → < 150 MiB** bara genom att sluta skriva `tree`, och
**~10–20 MiB** med zstd + git-pekare för committade tillstånd.

---

## 4. Svar på de fem kraven

1. **Storleksfördelning** — mätt ovan: AST 99,1 %, källkod 0,9 %, dubbletter 2 %.
2. **Avbrottssäker migration** — nytt lager är *additivt*: gamla JSON lämnas
   orörda och är läsbara hela tiden (bara `source_code` läses). Migrering sker
   fil för fil med manifest: verifiera `source_code` → hash → objekt → **läs
   tillbaka och jämför** → först därefter får originalet raderas. Aldrig radering
   före verifierad återläsning. Avbruten körning = manifestet säger var den stannade.
3. **Enskild fil vs projekttillstånd** — två separata operationer:
   `resolve_file(file, at)` och `resolve_project(at)`. Den senare måste **per fil
   redovisa proveniens**: `after_blob` | `before_blob` | `git` | `absent` | `not_in_store`.
   Ingen tyst sammanblandning — det är samma regel som `from_symbol: null`.
4. **Läsande resolver** — returnerar text, skriver aldrig. Bevis: arbetsfilernas
   mtime oförändrade efter anropet.
5. **Extraktorversion** — lagras **inte** i blobben (innehåll är versionsfritt)
   utan i *svaret*: varje rekonstruerad graf bär `extractor_version` +
   `git_rev`/`txn`. Dagens extraktor som läser dåtidens text är inte dåtidens
   extraktor, och det ska stå.

---

## 5. Räck(er) transaktionerna för ett helt projekttillstånd?

**Ja, för filer GTW har rört — med fem deklarerade luckor:**

| # | Lucka | Åtgärd |
|---|---|---|
| 1 | **Nyaste tillståndet per fil saknas** som blob (bara on-disk) | skriv after-tillståndet vid skrivning när lagringen är billig (efter §3), eller deklarera `absent` |
| 2 | Filer GTW **aldrig rört** har ingen historik alls (bara git) | `resolve_project` måste lista dem som `not_in_store`, inte utelämna dem |
| 3 | `restore-project` faller tillbaka på **tidsstämpel** ("backup efter txn, annars senaste före") → kan ge before-state utan att säga det | proveniens i svaret, även i fallbacken |
| 4 | `EXTRACTOR_VERSION` finns bara i koden, inte i `index_revisions` | kolumn i `index_revisions` (samma fält som redan styr omparsning) |
| 5 | `CreateFile` loggar transaktion **utan** backup (before = frånvaro) | representera som `absent`-tillstånd, inte som saknad data |

`session-start`/`session-end` saknar hashar (3 rader) — det är korrekt, inte en
lucka: de ändrar inget innehåll.

---

## 6. Ordning (föreslagen, ingen del gjord)

1. **Sluta skriva `tree`.** ~100× effekt, enradig, gamla backuper orörda.
   Mät före/efter med `du -sh .gnawtreewriter_backups` + ett prov på en ny edit.
2. **Content-addressing + zstd** (`objects/<sha256>.zst`), dubbel läsning under
   övergången (JSON först, objekt sedan).
3. **Migrera de 980** med manifest och verifiera-läs-tillbaka, radera sist.
4. **`--at <txn|rev>`** som läsande resolver ovanpå (3), med proveniens per fil
   och `extractor_version` i svaret.
5. **Kontinuerligt mått**: `stored_bytes_per_unique_source_byte` i `doctor`
   (nämnaren utskriven, M7). Larm över 3. Då kan problemet inte smyga tillbaka.

**Delta-lagring: nej, inte nu.** Med 31× kompression och `tree` borta är vi på
tiotalet MiB; delta köper lite och gör återställningen känsligare.



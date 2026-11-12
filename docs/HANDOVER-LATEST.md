# Handover — 2026-11-12 (V): spårbarhet, tillstånd, två Motor2-fynd

**Läget just nu:** allt committat och pushat (`b5a3c11`), träd rent,
`## master...origin/master`. **466 tester**, `clippy --all-targets -D warnings`
= 0, `scripts/commit-build.sh` → `RESULT ok`. Ingen release taggad (0.26.1 ute,
0.27.0 ej påbörjad).

Läsordning för den som tar över: den här filen → `uppslag.md` (nyast först) →
`CHANGELOG.md` `[Unreleased]` → `docs/BACKUP_STORAGE_AND_TIME_TRAVEL.md`.

---

## 1. Denna session — KLART

Sju sammanhängande saker, alla med mätning före ändring:

**a) Semantiska indexet ljög om färskhet.** `file_hash` var en **path**-hash →
ingenting kunde någonsin upptäcka en ändring. Nu hashas innehåll;
`has_content_hash()` är enda skip-frågan. `ai index <PATH>` skrev till **cwd**:s
store i stället för målets rot (fixat, skriver `📦 Store:`). Hermetiskt bevis:
2 inbäddade → 0/2 → en `printf`-ändring → 1/2.

**b) Ärliga nämnare.** `local_resolution` per relationstyp (resolved /
module_path / needs_receiver / ambiguous / external / macros) + `reading_note` i
`--coverage`; `--scope <prefix>` filtrerar **täljare och nämnare tillsammans**
(mätt spridning: hela projektet 43 %, `src/core/` 47 %, `src/llm/` 35 %,
`src/mcp/` 53 %). `file_scope_by_design` (döpt om från `..._unmatched`).

**c) Filnivå:** `--file-graph [--focus]`, `--cycles --by-file` (SCC),
`--touches <SYMBOL>` (båda riktningarna, `from_symbol: null` = okänt),
`--changed` (git-diff + untracked, `triple_in_store`).

**d) Relationshistorik i DB:n** — tabellen fanns med **noll skrivare**.
`RelationDelta` + `agg_relations` + `diff_relations` + `record_revision`;
before-image fångas före per-fil-upsert. Aggregerad nyckel
`from_id|relation_type|to_name` (`to_id` utesluten — resolution är härledd),
`count_changed` som egen kind. Mätt rev 311: 55 added, 6 count_changed,
1 removed; omkörning → 0. `--history` + `--onion` (frame = tillstånd **vid**
revisionen, off-by-one rättad).

**e) `--scope` på alla fem** (`coverage`, `touches`, `file_graph`, `history`,
`onion`, `changed`) med semantiken namngiven per kommando: motpart för
`touches`, incident-par för `file_graph`, prefix på relation key för
history/onion. Nya fältet `matches_in_store` gör att en tom sida inte kan
förväxlas med "historiken börjar nu".

**f) Backuper: AST:n bort.** `tree` var **99,1 %** av 15,45 GiB (15,31 GiB) och
lästes **ingenstans**. v2 lagrar bara `source_code`: samma fil 49 002 423 B →
304 735 B = **160,8×**. Projektion för de 980: ~139 MiB (och ~5–6 MiB med zstd
senare). Gamla blobbar orörda och läsbara.

**g) Tillstånd (A+B).** `undo`/`redo` skriver nu **reaktioner**
(`OperationType::Undo`/`Redo` med `metadata.reverted_txn` / `restored_txn`) och
den applicerade mängden **härleds** (`TransactionLog::applied_transactions`,
`redo_candidates`). Journalen är append-only. `core::state_head` projicerar
"vilket innehåll gäller per fil" + `content_source`; `.gnawtreewriter_head.json`
är en **cache med vattenstämpel** — raderar man den blir projektionen identisk
(testat). `head [--verify] [--trace]` — `--verify` rapporterar
`no_content_source` / `source_unreadable` / `source_hash_mismatch` /
`working_tree_mismatch` / `file_missing` / `cache_stale` och **lagar aldrig**.
Efter-blobbar ger det nyaste tillståndet en verifierbar källa (den gamla
before-blobben kunde inte).

**h) Attribution (deklarerad, frivillig).** `core::attribution`: bidragsgivare
med **roller som lista** (en identitet med flera roller skrivs en gång),
`mechanism` (cli/mcp/batch/plugin — satt av **kodvägen**, kan inte påstås),
`client`/`client_version` ur MCP-handskakningen, `change_id` (en logisk ändring
över filer) och `task_id` ovanför. Lagras i `Transaction.metadata` → gamla
journaler läses oförändrat, inget lib-API brutet. `--author`, `--co-designer`,
`--reviewer`, `--initiator`, `--model`, `--change-id`, `--task-id`.
`head --trace` visar nämnare + `by client` / `by mechanism`.

**i) Två Motor2-fynd** (se §4).

## 2. Den röda tråden — klassen av fel som återkommit fyra gånger

**Ett värde som ser ut som data men inte är det.** Förekomster i den här
sessionen: path-hash i `file_hash`; fantom-store i fel katalog; `mamba3`-katalog
(6,4 GB) utan loader; `resolution_rate` utan nämnare; `tree` i varje backup;
`file_versions.before_hash` alltid NULL; `revisions_tx_links` med noll rader;
`prose_annotations` med noll anropare. **Fixen är alltid "gör talet till det det
påstår", aldrig "förbättra talet".**

Invarianter som inte får raseras (de är dyrt köpta):

1. **Journalen äger tillståndshistoriken; indexet härleder.** Append-only —
   ingenting skrivs om, reaktioner är nya rader.
2. **Head är en projektion, aldrig en auktoritet.** `rm .gnawtreewriter_head.json`
   förlorar ingenting.
3. **Frånvaro är inte noll.** `graph: Option<f32>` — "absence is not agreement".
   `unknown` är en riktig hink i varje räkning.
4. **Varje tal skriver ut sin nämnare.** `--scope`, `local_resolution`,
   `head --trace` (0/980), `--coverage`.
5. **Lagra aldrig utvärderat tillstånd.** Blender-läxan: keys, inte states.
   En sparad AST är dessutom *fryst* — den påstår vad en dåtida parser sa.
6. **`mechanism` sätts av kodvägen**, aldrig av anroparen. `client` är
   självrapporterad och får inte presenteras som verifierad.
7. **Tester är hermetiska:** `tempfile::tempdir()` + **`.git`-markör** (annars
   kapar en sessionsfil i `/tmp` backuparna — mätt i den här sessionen).
8. **`end_line` betyder "raden med sista byten"**, inte "sista raden". För text
   som slutar med `\n` är den en rad för hög. Använd den aldrig som slice-gräns
   utan att kompensera (se §4).

## 3. Vad som mättes och var siffrorna finns

| Mätning | Värde | Var |
|---|---|---|
| AST:ns andel av backuperna | 15,31 / 15,45 GiB = 99,1 % | `docs/BACKUP_STORAGE_AND_TIME_TRAVEL.md` |
| v1 → v2 för samma fil | 49 002 423 B → 304 735 B (160,8×) | samma doc §7 |
| lokal resolutionsgrad | 43 % totalt; 47/35/53 per scope | `--coverage` |
| relationer som externa/makro/mottagarberoende/ambigusa | 12 284 / 3 659 / 2 056 / 1 428 | `local_resolution` |
| `head --verify` på repot | 61 `no_content_source`, 41 `working_tree_mismatch`, 2 `file_missing` | `head --verify --json` |
| `head --trace` på repot | **0/980** med deklarerad bidragsgivare | `head --trace` |
| attributionsspridning (temp) | by client `opencode=1 unknown=2`; by mechanism `cli=2 mcp=1` | CHANGELOG `[Unreleased]` |

**Misstagen jag gjorde, så nästa inte upprepar dem:**

- *"Prosa-filen äger körningen"* — det var **debug-binären** (10–50× långsammare
  för ONNX). Mät med release/installerad binär när det gäller prestanda.
- *"Import-scope ger +1 000 resolvade kanter"* — det gav **+105**. Realistiskt
  tak för heuristik är ~45–50 %; 43 % ligger nära det. Compiler-orakel (SCIP) är
  enda vägen till 90 %+, opt-in, aldrig ett krav.
- **Verktygsanrop blev text i stället för att köras** upprepade gånger i
  Cline-miljön. Inget skadades (fallet inträffade före exekvering), men
  motmedlet som fungerade: **ett anrop per tur**, inga nästlade citattecken,
  skriv skript med `editor` och kör `bash /tmp/x.sh` (naket, utan flaggor).
  Användaren överväger att flytta till OpenCode av samma skäl.

## 4. De två Motor2-fynden (reproducerade, åtgärdade, svar skrivet)

Fullt svar i `docs/SVAR-TILL-MOTOR2-2026-11-12.md` §6.

**1) `delete` svalde nästa rad.** `/// doc` ovanför `use std::fmt;` → båda
försvann, och filen parsade fortfarande, så syntaxvakten såg inget (tyst
kodförlust). Orsak: `end_line` = raden med sista byten; för text som slutar med
`\n` är den en rad för hög, och `delete` använde den som exklusiv slice-gräns.
En funktion slutar inte med `\n` → därför var funktionsradering rätt och buggen
kunde leva. Fix: räkna om slutraden. Test
`delete_of_a_newline_terminated_comment_keeps_the_next_line`.

**2) `insert_node` med position utanför intervallet.** Två varianter, båda med
"Content inserted": klammerförälder → innehåll **efter** `}` (utanför modulen);
**lövnod** som förälder → `children.is_empty()` → `start_line - 1` = **överst i
filen**. Fix: klampning till förälderns eget slut **plus varning på stderr**.
Test `insert_beyond_the_child_count_clamps_inside_the_parent`.

`end_line`-semantiken är **orörd** (många konsumenter läser den). De två
ställena som använde den som grind är fixade; att normalisera fältet (eller lägga
till `start_byte`/`end_byte`) är en öppen tråd.

## 5. Öppna trådar, i prioritetsordning

1. **Bron (C): `revisions_tx_links` + `file_versions.before_hash`.** Tabellen är
   deklarerad och har **noll skrivare**; `file_versions` skrivs men läses aldrig
   och `before_hash` är alltid NULL. En `INSERT` vid indexering + en kolumn räcker
   för att koppla head till indexet och därmed låsa upp `--at` och onion över
   transaktioner. **Nästa naturliga steg.**
2. **Claims (D):** `prose_annotations` (anchor_kind/key/hash, status,
   invalid_reason) har schema + `upsert_prose` + statusuppdaterare men **noll
   anropare** och noll rader. Koppla in + invalidiering när `anchor_hash` inte
   längre matchar. ALF ska bli den korta, LLM-fria ytan (dryg, parsebar —
   `tuulikk[unknown:author] via cli change=c1`).
3. **Lagring:** formatmigration av de 980 v1-blobbarna (verifiera-läs-tillbaka
   före radering, manifest), sedan zstd/content-addressing och **GC** — GC kräver
   referensmodellen från (1).
4. **Döda-ledningar-kontrollen:** lista strukturer som skrivs men aldrig läses,
   **med belägg och osäkerhet** — inte döma ut kod. Hade fångat `tree`,
   `file_versions`, `revisions_tx_links`, `prose_annotations`.
5. **`end_line`-normalisering** (eller byte-offsets) — nu med två mätta symptom.
6. **Attributionens kanter:** batch får `change_id` `auto:batch-…` när ingen
   anges (mönstret är etablerat); `kind` är alltid `unknown` från CLI (avsiktligt
   — CLI kan inte veta); `--trace` saknar tidsfönster (kosmetika).
7. **Onion/vyer:** `docs/ONION_SKIN_MODEL.md` och `docs/VIEWS_MODEL.md` är
   modeller utan kod. Vyer = linser (tröskel/opacitet/ordning), aldrig lagring;
   frånvaro ≠ noll; varje vy redovisar egen nämnare.
8. **`--at`:** kräver (1) + en läsande resolver som returnerar **text**, aldrig
   skriver (bevis: mtime oförändrad).

## 6. Fällor i miljön (kostat tid)

- **`/tmp/.gnawtreewriter_session.json` finns** (från en ad-hoc-körning) och gör
  att `find_project_root` kapar temp-katalogers backuper till `/tmp`. Pinnna
  alltid med `.git` i temp-projekt.
- **Debug-binären är 10–50× långsammare** för ONNX. Prestandapåståenden kräver
  release/installerad binär.
- **`--json` är global** på `Cli`; att deklarera en andra `--json` på en
  subcommand är en dubblett som `tests/cli_contract` fångar.
- **clap:s `debug_assert` behöver ~16 MiB stack** (~100 subkommandon); testtråden
  har 2 MiB → SIGABRT. `cli_contract` kör assertionen på en stor tråd.
- **`redo` kräver after-blob.** Finns bara för rader skrivna efter 2026-11-12;
  äldre rader ger `journal:no-blob` i head — det är sant, inte ett fel.
- **`scripts/daemon-hygiene.sh`**: döda aldrig din egen MCP-daemon mitt i en
  session (`GTW_REAP=yes`), klienten återansluter inte själv.
- **Motor2 pinnar en commit**, följer inte master (fyra HEAD-flyttar på en dag).

## 7. Arbetsmetod som fungerade

1. **Mät före ändring** (och mät om efter) — varje påstående i den här filen har
   ett kommando bakom sig. Två observationer innan en bugg rapporteras.
2. **Reproducera i temp-projekt med `.git`** — aldrig mot repots egen state.
3. **Rätt ställe:** GTW `edit`/`quick-replace` → `editor` → aldrig
   `python`/`sed -i`/`dd`/`tee` på projektfiler (M2). En avvikelse i den här
   sessionen (två `python3`-heredoc för mekaniska flersidiga byten) är redovisad i
   `uppslag.md` med vad som kompenserade — godkända tester ersätter inte regeln.
4. **Skriv in fällan i koden** där den gäller, och i `uppslag.md` med belägg,
   status och "om det är fel".
5. **Avgränsa:** en ändring = en commit med mätning i meddelandet, `RESULT ok`
   som sista rad.

## Pekare

- Detta är session V. Föregående: `docs/HANDOVER-LATEST.md` i git-historiken
  (`git log --all -- docs/HANDOVER-LATEST.md`).
- Docs: `BACKUP_STORAGE_AND_TIME_TRAVEL.md` (§7 = mätningen),
  `ONION_SKIN_MODEL.md`, `VIEWS_MODEL.md`, `LOCAL_MODEL_BAKEOFF.md`,
  `SVAR-TILL-MOTOR2-2026-11-12.md` (§6 = Motor2-fynden).
- Fyndlista: `uppslag.md` (nyast först, med status och nämnare).
- Motor2:s rapporter: `FEEDBACK_FRAN_MOTOR2_2026-11-12.md`,
  `GTW_FYND_FRAN_MOTOR2_2026-11-12.md`.
- Nyckelmoduler: `src/core/state_head.rs`, `src/core/attribution.rs`,
  `src/core/transaction_log.rs` (`applied_transactions`, `redo_candidates`),
  `src/core/capabilities.rs`, `src/core/index_levels.rs`.
- Modeller installerade: 418 MB `bge-base` (embeddings, L3) + 697 MB `lfm25`
  (L4, kräver `mamba`-feature). 7,5 GB felaktiga modeller raderade 2026-11-12.



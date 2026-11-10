# GnawTreeWriter — Release Notes (v0.25.0)

**Datum:** 2026-11-10
**Typ:** Minor Release (nya funktioner + beteendeändringar; ingen API-brytning i lib)

## Sammanfattning

Två spår i en release: **språk- och edit-korrekthet** (markdown/css/yaml/toml
flyttade till tree-sitter, fyra nya grammatiker) och **planerings­trovärdighet**
(P2-graf­frågor, ärlig resolution, falsifierbara svar). Dessutom stängdes sex
CLI-hål som lurade agenter — bland dem `--version` som såg ut som en krasch och
`insert` som panikade på varje anrop i debug-bygge.

Den viktigaste ändringen är inte en funktion utan ett **avstående**: verktyget
slutar påstå saker det inte vet. Kors-fil-metodanrop resolvas inte längre på
namnunikum, och varje grafsvar bär nu sin egen färskhet och osäkerhet.

## Changes

### Språk
- **Fyra nya grammatiker:** PowerShell (`.ps1/.psm1/.psd1`), CMake
  (`.cmake` + `CMakeLists.txt`), Makefile (filnamn), Ruby (`.rb` — tidigare en
  *tyst* fallback till GenericParser via ett alias utan parser).
- **Fyra parsrar bytta till tree-sitter:** markdown, css, yaml, toml. De var
  position-blinda (`start_col`/`end_col` = 0) och `edit_node` föll tillbaka på
  helrads-ersättning: en rubrik tappade sitt `#`, en CSS-regel ersattes av sitt
  värde, yaml/toml gick inte att redigera alls. Netto −1 249/+835 rader.
- **Filnamnsdetektering** före ändelsematchning (`CMakeLists.txt`, `Makefile`,
  `.bashrc`/`.zshrc`/`.profile`), `.pyi` → Python, `.mts`/`.cts` → TypeScript,
  och `code_index`-defaulten 15 → 32 ändelser.

### Planering
- **`code_graph`** (P2): `cycles`, `impact`, `chain`, `reachable` — MCP
  `code_index kind=cycles|impact`, CLI `code-index --cycles|--impact|--why`.
- **`--why <namn>`** visar per kant varför den finns (`unique_in_index` /
  `ambiguous_pick` / `unresolved`) — ett grafsvar går att falsifiera.
- **`meta`-kuvert** på varje grafsvar: `store_rev`, `worktree_rev`,
  `index_stale`, `static_only`.

### Korrekthet
- **Edit-vägen:** `edit_node` tappade filens avslutande newline i **alla**
  format (även Rust) — fixat; `generate_sparkline` delade med noll när en
  **leaf** redigerades (krasch *efter* lyckad skrivning) — fixat.
- **Sex CLI-hål:** exit-koder för `--version`/`--help` (1 → 0, stdout),
  användningsfel 2, `analyze` non-zero vid noll parsade filer, `insert`
  (panik → fungerar), dubbletter `-m`/`-i`, `restore`-flaggan, `quick-replace`
  med `-`-värden, samt **`edit --expect-name`** som avvisar en inaktuell path i
  stället för att tyst skriva fel nod.
- **Extraktor v3:** kors-fil-metodmål resolvas inte längre. Resolvade
  call-kanter 3 898 → 2 064, kors-fil metodmål 1 848 → 0, fler-nods­cykler
  338 → 29 (mätt på GTW:s egen store).
- **Hooken** kan inte längre nedgradera storen med en äldre binär.

## Upgrade

```bash
cargo install --path . --features modernbert,mcp
gnawtreewriter code-index . --db     # full ombyggnad (extraktor v3)
```

**Konsumenter av indexet (Motor2 m.fl.):** `to_id`-täckningen blir lägre. Det
som försvinner är fabricerade kanter; `coverage` redovisar dem som `unresolved`.
Räkna om eventuella trösklar som vilar på den gamla siffran (70,7 %).

## Known

- **JSON** går kvar på serde + `json_semantic_edit` (J1): paths är nyckelnamn,
  inte numeriska. `--preview` visar den råa splitten medan skarp edit tar den
  semantiska vägen (preview och apply är oense för JSON).
- **`.h` → C-parsern** även för C++-headers (heuristik ej införd).
- **73 self-loops** kvar i cykeldetektionen (samma-fil metod-self-edges).
- `cargo fmt --check` avviker på ett antal filer (befintlig drift, ej i denna
  release).

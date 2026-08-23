# Repomix-jämförelse & Åtgärdsplan

**Datum:** 2026-08-23
**Syfte:** Analysera Repomix (https://repomix.com/) och identifiera funktioner som GnawTreeWriter kan emulera för att bli mer AI-vänligt.

---

## Sammanfattning

Repomix packar hela en kodbas till ett AI-optimiserat format. GnawTreeWriter fokuserar på AST-nivå-editering med säkerhetsgarantier. De kompletterar varandra — men GTW kan adoptera flera Repomix-koncept för att förbättra AI-agenters upplevelse.

| Område | Repomix | GTW (nuvarande) | Gap |
|--------|---------|-----------------|-----|
| Kodkompression | `--compress` via Tree-sitter (~70% reduktion) | `get_skeleton` (struktur) | Saknar komprimerings-läge för hela filer |
| Token-räkning | Per-fil och totalt | Ingen | Stort gap — agenter vet inte hur mycket kontext de använder |
| Paketering av repo | `repomix` → XML/MD/JSON | Saknas | Ingen `pack`-kommando |
| Säkerhet/Secrets | Secretlint-integration | Ingen | Risk för att känsliga nycklar läcks via LLM |
| MCP-server | `repomix --mcp` | `gnawtreewriter mcp stdio` | Redan bra — kan förbättra sandbox-läge |
| Custom instructions | `--instructions` flagga | Saknas | Agenter kan inte lägga till kontext |
| Agent skills | Auto-genererad MCP-config | Manuellt `AGENTS.md` | Saknar auto-generering |
| Watch-läge | Filövervakning | Session-system (manuellt) | Saknar real-time övervakning |
| Output-format | XML, MD, JSON, Plain | JSON (analyze), MD (skeleton) | Färre alternativ |

---

## Åtgärdsförslag

### Prio 1: Hög impact, rimlig insats

#### 1.1 `pack`-kommando — Paketera hela projektet för AI

**Syfte:** Exportera ett helt projekt till ett AI-optimiserat format som kan skickas till LLM-kontext.

**Funktionalitet:**
```bash
gnawtreewriter pack . --style markdown --output project-context.md
gnawtreewriter pack . --style json --compress
gnawtreewriter pack . --style xml --include "src/**/*.rs,**/*.toml"
```

**Implementation:**
- Ny modul: `src/core/pack.rs`
- Använder `walkdir` + befintlig parser-pipeline
- Stöd för `--include`/`--ignore` glob-mönster (redan finns `regex` + `walkdir`)
- Output-format: markdown (med trädstruktur + komprimerad kod), JSON (AST-per-fil), XML
- Inkludera: trädstruktur, token-count per fil, komprimerad/innehålls-kod

**Varför viktigt:**
GTW:s styrka är AST-kunskap. Ett `pack`-kommando kan ge **semantiskt packning** — t.ex. "här är alla publik-funktioner i lib.rs" snarare än råtext.

#### 1.2 Token-räkning — Estimera kontextanvändning

**Syfte:** Beräkna uppskattat token-antal per fil och för hela exporten.

**Funktionalitet:**
```bash
gnawtreewriter analyze src/main.rs --tokens     # token-count per nod
gnawtreewriter pack . --tokens                   # totalt token-count
gnawtreewriter list src/main.rs --tokens         # token-count per definition
```

**Implementation:**
- Enkel tokenizer: `text.split_whitespace().count() * 1.3` (ca tokens per ord)
- Eller bättre: `tiktoken-rs` crate (OpenAI-kompatibel) — ~200 tokens per fil overhead
- Lägg till i JSON-output: `"estimated_tokens": 342`
- Varningsgränser: >4000 tokens per fil (de flesta LLM-ar med 8k-32k kontext)

#### 1.3 Git-aware filtrering — Ersätt hårdkodade skip-logor

**Syfte:** Alla moduler som traverserar filer (find, inspect, blast, refactor, indexer) ignorerar projektets `.gitignore` och har inkonsistenta hårdkodade listor.

**Nuvarande problem:**

| Modul | Hoppar över | Saknar |
|-------|-------------|--------|
| `gnaw_find.rs:44` | `target`, `node_modules`, `.git`, `.`-prefix | `.gitignore` |
| `blast.rs:206` | `target`, `node_modules`, `.git` | `.gitignore`, `.`-prefix |
| `inspect.rs:160` | `target`, `node_modules`, `.git`, `.`-prefix | `.gitignore` |
| `gnaw_refactor.rs:158` | `target`, `node_modules`, `.git` | `.gitignore` |
| `project_indexer.rs:45` | Alla `.`-startade kataloger | `.gitignore` |

**Lösning:** Lägg till `ignore`-crate (av ripgrep-skaparna) och ersätt alla manuela skip-logor.

```toml
# Cargo.toml
ignore = "0.4"
```

```rust
// src/core/file_walker.rs — Gemensam walkdir-funktion
pub fn walk_source_files(root: &Path) -> impl Iterator<Item = PathBuf> {
    ignore::WalkBuilder::new(root)
        .hidden(false)          // Inkludera dolda filer (men respektera .gitignore)
        .git_ignore(true)       // Läs .gitignore
        .git_global(true)       // Läs ~/.gitignore_global
        .parents(true)          // Läs .gitignore i föräldrakataloger
        .build()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|ft| ft.is_file()).unwrap_or(false))
        .map(|e| e.into_path())
}
```

**Fördelar:**
- En enda, korrekt implementation istället för 5 inkonsistenta
- Respekterar projektets `.gitignore` automatiskt
- Stöd för global gitignore (`~/.gitignore_global`)
- Stöd för `.git/info/exclude`
- Traverserar inte `target/`, `node_modules/`, `.git/` osv. (redan i `.gitignore`)
- Frigör ~80 rader duplicerad kod

**Risk:** Låg — `ignore`-cratet är standard inom Rust-ekosystemet (ripgrep, bat, delta).

---

#### 1.4 Kodkompression — `compress`-läge

**Syfte:** Komprimera filer till bara signatures + struktur, med bevarad semantik.

**Funktionalitet:**
```bash
gnawtreewriter compress src/main.rs             # komprimerad output
gnawtreewriter compress . --output compressed/  # hela projektet
gnawtreewriter pack . --compress                # kombinera med pack
```

**Implementation:**
- Utöka `get_skeleton` till att stöda djupare kompression:
  - Behåll: funktions-signaturer, typ-definitioner, imports, attribut/dekoratörer
  - Ta bort: funktionskroppar (ersätt med `⋮----`), loopar, villkor, interna variabler
  - Behåll doc-kommentarer
- Använd Tree-sitter-noder för att veta exakt vad som är "implementation" vs "kontrakt"
- Reduktion: ~60-70% (mätt på Rust-projekt)

---

### Prio 2: Medium impact

#### 2.1 Secret-detection — Skyddslager mot credential-läckage

**Syfte:** Förhindra att API-nycklar, tokens och lösenord läcks via LLM-arbetsflöden.

**Funktionalitet:**
```bash
gnawtreewriter pack . --scan-secrets            # varna för känsliga mönster
gnawtreewriter validate --scan-secrets          # kör vid validering
```

**Implementation:**
- Regex-mönster för: AWS keys, GitHub tokens, private keys, JWT, base64-encoderade secrets
- Lägg till i `pack`-kommandot som default-flagga
- Output: `⚠ Secret detected in src/config.rs:42 — pattern: AKIA...`
- Ersätt känsliga rader med `<REDACTED>` i output
- Crate: `secrecy` eller egna regex (GTW behöver inte Secretlint-fullversionen)

#### 2.2 `--instructions` flagga — Lägg till kontext till output

**Syfte:** låt användare/agenter lägga till kontext-specifika instruktioner.

**Funktionalitet:**
```bash
gnawtreewriter pack . --instructions "Fokus på auth-modulen, ignorerar tester"
gnawtreewriter analyze src/main.rs --instructions "Jag behöver förstå flödet"
```

**Implementation:**
- Lägg till en `instructions: Option<String>` i output-metadata
- I markdown-output: lägg till som första sektionen
- I JSON-output: `"instructions": "..."`
- MCP-verktyg: lägg till `instructions` parameter på `analyze`, `pack`, `get_skeleton`

#### 2.3 Förbättrad `get_skeleton` — Bättre strukturöversikt

**Syfte:** Göra skeleton mer som Repomix's komprimering — med mer kontext.

**Befintligt:** Returnerar funktions-/klass-signaturer
**Förbättring:**
- Lägg till import-sektion
- Lägg till modulär struktur (vilka moduler anropar vilka)
- Lägg till token-count per node
- Stöd för djupkontroll per nodtyp

---

### Prio 3: Lägre prioritet, långsiktig

#### 3.1 Agent skills auto-generering

**Syfte:** Auto-generera MCP-konfiguration och instruktionsfiler.

```bash
gnawtreewriter agent-config --target copilot    # .github/copilot-instructions.md
gnawtreewriter agent-config --target mcp        # mcp-config.json
gnawtreewriter agent-config --target cursor     # .cursorrules
```

**Varför:** Repomix's `agent_skills_generation` gör liknande. GTW har redan `AGENTS.md` men den är manuellt underhållen.

#### 3.2 Sandbox-läge för MCP

**Syfte:** Begränsa filåtkomst till workspace-root (som Repomix `--sandbox`).

GTW:s MCP-server har redan implicit begränsning, men explicit `--sandbox` flagga vore tydligare för säkerhet.

#### 3.3 Watch-läge

**Syfte:** Real-time filövervakning med om-parsning.

Repomix har `watch-mode` som re-pakar vid ändringar. GTW:s `session-system` hanterar historik men saknar real-time triggrar. Långsiktigt: `gnawtreewriter watch` som triggar om-analys vid filändringar.

#### 3.4 Fler output-format

**Syfte:** XML-output (kompatibel med Repomix) och Plain-text.

GTW har redan JSON och markdown. XML vore nyttigt för kompatibilitet med verktyg som konsumerar Repomix-format.

---

## Rekommenderad implementeringsordning

```
Vecka 1:   Git-aware filtrering (1.3) — fundamentet för pack/find/inspect
Vecka 2:   Token-räkning (1.2) — snabb vinst, låg risk
Vecka 3-4: Kodkompression (1.4) — bygg på skeleton
Vecka 4-5: Pack-kommando (1.1) — kombinera med kompression + tokens + git-aware
Vecka 6:   Secret-detection (2.1) — säkerhetsfönster
Vecka 7:   Custom instructions (2.2) — enkel att lägga till
Framtid:   Agent skills (3.1), Sandbox (3.2), Watch (3.3)
```

**Varför git-aware först (vecka 1):**
Alla framtida funktioner (`pack`, `compress`, `inspect --orphans`, etc.) behöver
veta vilka filer som faktiskt tillhör projektet. Utan korrekt `.gitignore`-stöd
kommer de traversera `target/`, `node_modules/`, och andra generateda filer —
vilket ger felaktig token-räkning, felaktig kompression, och onödig prestanda-
förvärring. `ignore`-cratet är en ~2 timmars uppgift som sparar veckors debugging
senare.

---

## Relaterade befintliga funktioner

| GTW-funktion | Repomix-motsvarighet | Notering |
|---|---|---|
| `get_skeleton` | Kodkompression | Redan finns — utöka med kompression-läge |
| `analyze` | `pack_codebase` | Redan JSON-output — lägg till tokens |
| `search_nodes` | `grep_repomix_output` | Redan bättre (AST-medveten) |
| `sense` | — | Inget motsvarande i Repomix (GTW har semantisk sökning) |
| `batch` | — | Inget motsvarande i Repomix |
| `mcp stdio` | `--mcp` | Redan implementerat |
| `session-*` | — | GTW-unikt (temporal editing) |

---

---

## Testtäckning — Befintligt & Planerat

### Befintlig teststatus

**Totalt: 62 tester (alla passerar)**

| Kategori | Antal | Moduler |
|----------|-------|---------|
| Core unit-tests | 28 | batch (2), backup (2), transaction_log (4), undo_redo (6), tag_manager (4), diff_parser (5), macro_dispatcher (5), restoration_engine (2), scaffold (1) |
| Parser unit-tests | 4 | xml (2), generic (1), markdown (1) |
| CLI unit-tests | 3 | restore, quick_replace_preview, quick_replace_apply |
| LLM unit-tests | 1 | semantic_index (cosine similarity) |
| Integration tests | 26 | insert_position (11), mcp_integration (9), gnaw_sense (1), relational (1), ai_modernbert (1), gnaw_sense_integration (1), insert_position (1) |

### Befintliga klyftor — ICKE-testade moduler

| Modul | Prio för test | Notering |
|-------|---------------|----------|
| `find_node_by_path` / `find_node_by_name` | 🔴 Hög | Kärnfunktionalitet — alla edit-operationer beror på denna |
| `edit_node_at_path` | 🔴 Hög | Inga direkta tester, endast indirekt via CLI |
| `insert_node_at_path` | 🔴 Hög | Testas av insert_position.rs men ingen unit-test |
| `delete_node_at_path` | 🔴 Hög | Inga tester alls |
| `preview_edit` | 🟡 Medel | Testas indirekt via integration-tests |
| `guardian` | 🟡 Medel | Inga unit-tests |
| `healer` | 🟡 Medel | Inga unit-tests |
| `gnaw_find` | 🟡 Medel | Inga tester |
| `gnaw_diff` | 🟡 Medel | Diff-parser testas men inte gnaw_diff |
| `gnaw_refactor` | 🟡 Medel | Inga tester |
| `gnaw_graph` | 🟢 Låg | Nytillkommen modul |
| `alf` | 🟢 Låg | Loggmodul — kräver整合-test |
| `diagnostics` | 🟢 Låg | |
| `inspect` / `blast` | 🟢 Låg | |
| `label_manager` | 🟢 Låg | |
| `anchor` | 🟢 Låg | |
| `visualizer` | 🟢 Låg | |
| `blueprint` | 🟢 Låg | |

### Planerade tester — Nya Repomix-inspirerade funktioner

#### Prio 1: Git-aware filtrering (1.3)

```
tests/gitignore.rs — Integration-tester
├── test_respects_local_gitignore         # Fil i .gitignore → ej traverserad
├── test_respects_global_gitignore        # ~/.gitignore_global
├── test_respects_parent_gitignore        # Föräldramappars .gitignore
├── test_includes_untracked_files         # Fil som ej är commitad men ej ignorerad → med
├── test_excludes_target_dir              # target/ ingår i .gitignore → ej med
├── test_excludes_node_modules            # node_modules/ → ej med
├── test_hidden_files_respected           # Dolda filer (men ej .gitignore) → med om ej ignorerade
├── test_empty_gitignore                  # Tom .gitignore → allt med
├── test_negation_in_gitignore            # !pattern fungerar
└── test_multiple_repos                   # Monorepo med flera .gitignore

src/core/file_walker.rs — Unit-tester
├── test_walk_source_files_basic          # Enkel traversal
├── test_walk_source_files_with_ignore    # Med .gitignore
├── test_walk_source_files_no_git         # Utan .git-rep → fallback
└── test_walk_source_files_symlinks       # Symlinks hanteras korrekt
```

#### Prio 1: Token-räkning (1.2)

```
tests/token_count.rs — Integration-tester
├── test_count_tokens_rust_file         # Känd fil → förväntat token-antal
├── test_count_tokens_python_file       # Python-specifik
├── test_count_tokens_json_output       # --json-flagga
├── test_count_tokens_zero_file         # Tom fil → 0
├── test_count_tokens_large_file        # Stor fil (>4000 tokens) → warning
└── test_count_tokens_per_node          # Per-nod-räkning

src/core/token_count.rs — Unit-tester
├── test_estimate_tokens_basic          # "hello world" → ~2 tokens
├── test_estimate_tokens_code           # Kod med symboler
├── test_estimate_tokens_empty          # Tom sträng → 0
└── test_token_threshold_warning        # Gränsöverstigning
```

#### Prio 1: Kodkompression (1.3)

```
tests/compress.rs — Integration-tester
├── test_compress_rust_function         # Behåller signatur, tar bort kropp
├── test_compress_preserves_imports     # Imports bevaras
├── test_compress_preserves_types       # Typ-definitioner bevaras
├── test_compress_removes_body          # Funktionskropp → ⋮----
├── test_compress_preserves_doc_comments # Doc-kommentarer bevaras
├── test_compress_reduction_ratio       # ≥60% reduktion
├── test_compress_python                # Python-specifik
├── test_compress_multifile             # Hela katalogen
└── test_compress_output_markdown       # Markdown-format

src/core/compress.rs — Unit-tester
├── test_should_compress_node           # Funktioner ja, imports nej
├── test_compress_signature_only        # Bara signatur
├── test_compress_block_statement       # Block → ⋮----
└── test_compress_preserves_attributes  # Attribut/dekoratörer
```

#### Prio 1: Pack-kommando (1.1)

```
tests/pack.rs — Integration-tester
├── test_pack_markdown_output           # Markdown med trädstruktur
├── test_pack_json_output               # JSON per fil
├── test_pack_with_include_pattern      # --include filter
├── test_pack_with_ignore_pattern       # --ignore filter
├── test_pack_with_compress             # --compress + pack
├── test_pack_with_tokens               # --tokens + pack
├── test_pack_with_instructions         # --instructions i output
├── test_pack_empty_project             # Tom katalog
├── test_pack_single_file               # Enskild fil
└── test_pack_output_to_file            # --output flagga

src/core/pack.rs — Unit-tester
├── test_collect_files_glob             # Glob-matchning
├── test_respects_gitignore             # .gitignore respekteras
├── test_tree_structure_markdown        # Träd i markdown-format
├── test_metadata_json                  # JSON-metadata (tokens, filer)
└── test_secret_detection_in_pack       # Secrets ersätts med <REDACTED>
```

#### Prio 2: Secret-detection (2.1)

```
tests/secrets.rs — Integration-tester
├── test_detect_aws_key                 # AKIA...
├── test_detect_github_token            # ghp_...
├── test_detect_private_key             # -----BEGIN RSA
├── test_detect_jwt                     # eyJ...
├── test_detect_generic_secret          # password=, api_key=
├── test_redact_in_pack_output          # <REDACTED> i output
├── test_no_false_positives             # Vanlig kod triggar inte
└── test_secret_count_in_metadata       # Antal secrets i metadata

src/core/secrets.rs — Unit-tester
├── test_aws_key_pattern                # Regex-mönster
├── test_github_token_pattern
├── test_private_key_pattern
├── test_jwt_pattern
├── test_base64_secret_pattern
├── test_line_is_secret                 # Hjälpfunktion
└── test_redact_line                    # Ersättning
```

#### Prio 2: Custom instructions (2.2)

```
tests/instructions.rs — Integration-tester
├── test_instructions_in_markdown       # Första sektionen
├── test_instructions_in_json           # "instructions"-nyckel
├── test_instructions_mcp_tool          # MCP-parameter
└── test_no_instructions_default        # Ingen flagga → ingen instructions-sektion
```

### Rekommenderad testordning

```
Fas 0 (vecka 1): Git-aware filtrering + Befintliga klyftor
  ├── src/core/file_walker.rs unit-tests
  ├── tests/gitignore.rs
  ├── find_node_by_path/unit-tests
  ├── edit_node_at_path/unit-tests
  ├── preview_edit/unit-tests
  └── guardian/unit-tests

Fas 1 (vecka 2): Token-räkning
  ├── tests/token_count.rs
  └── src/core/token_count.rs unit-tests

Fas 2 (vecka 3-4): Kodkompression
  ├── tests/compress.rs
  └── src/core/compress.rs unit-tests

Fas 3 (vecka 4-5): Pack + Secrets
  ├── tests/pack.rs
  ├── tests/secrets.rs
  └── src/core/pack.rs unit-tests

Fas 4 (vecka 6): Custom instructions + MCP
  ├── tests/instructions.rs
  └── MCP-verktyg integration
```

---

## Källor

- Repomix webbsida: https://repomix.com/
- Repomix code compression: https://repomix.com/guide/code-compress
- Repomix MCP server: https://repomix.com/guide/mcp-server
- GTW Roadmap: docs/ROADMAP.md
- GTW LLM Integration: docs/LLM_INTEGRATION.md
- GTW Future Concepts: docs/FUTURE_CONCEPTS.md

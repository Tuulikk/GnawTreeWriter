# GTW ↔ Agent Memory Integration Specification

**Version:** 1.0
**Status:** Draft
**Datum:** 2026-08-24

---

## 1. Bakgrund

Graphlit's Memory Index beskriver en multi-dimensional lookup-lager för agent-minne:
semantisk, entitet, relationell, temporal, och keyword-sökning. GTW har redan
de flesta av dessa dimensioner för **enstaka projekt** via sitt MCP-api.

Denna specifikation beskriver hur GTW kan agera som en **kodintelligens-lager**
som andra agent-memory-system konsumerar — utan att GTW behöver ändra sin
grundläggande arkitektur.

---

## 2. Arkitektur

```
┌─────────────────────────────────────────────────────────┐
│                    Agent Memory System                    │
│  (persistens, multi-projekt, agent-historik, användare)  │
└───────────┬──────────────────────────┬──────────────────┘
            │  MCP-verktyg             │  MCP-verktyg
            ▼                          ▼
┌───────────────────────┐  ┌───────────────────────┐
│   GTW (projekt A)     │  │   GTW (projekt B)     │
│   • AST-index         │  │   • AST-index         │
│   • Semantisk sökning │  │   • Semantisk sökning │
│   • Relations-karta   │  │   • Relations-karta   │
│   • Token-count       │  │   • Token-count       │
│   • Kompression       │  │   • Kompression       │
│   • Secret-detection  │  │   • Secret-detection  │
└───────────────────────┘  └───────────────────────┘
```

**Princip:** GTW projektinstanser är **stateless intelligens-tjänster**.
De lagrar inget persistent — de analyserar och returnerar. Det externa
systemet ansvarar för lagring, historik och multi-projekt-anslutning.

---

## 3. Designöverväganden

### 3.1 Deterministiska entitets-ID:n

**Problem:** Om entitets-ID bygger på radnummer (t.ex. `gtw:project:file.rs:12`)
skapas spöknoder eller dubbletter när kod flyttas.

**Lösning:** ID:n ska vara deterministiska, baserade på modul + namn:

```
gtw:{project}:{module_path}:{entity_kind}:{entity_name}
```

Exempel:
```
gtw:project_a:src/auth/login.rs:function:validate_password
gtw:project_a:src/auth/login.rs:struct:AuthConfig
```

Detta möjliggör **upsert** (ersätt) per entitet utan att bry sig om
radnummer. Memory System kan säkert radera gamla entiteter för en fil
och lägga till nya utan risk för dubbletter.

### 3.2 Paginering och filtrering

**Problem:** Stora projekt kan producera enorma JSON-responser som äter
upp agentens kontextfönster.

**Lösning:** Alla nya verktyg ska ha inbyggd paginering:

```json
{
  "tool": "index_entities",
  "args": {
    "file_path": "src/main.rs",
    "visibility": "public",
    "entity_types": ["function", "struct"],
    "limit": 50,
    "offset": 0
  }
}
```

Filtreringsalternativ:
- `visibility`: `"public"` | `"private"` | `"all"` (default: `"all"`)
- `entity_types`: lista av typer att inkludera
- `limit` / `offset`: paginering
- `min_tokens`: filer med färre tokens än denna inkluderas ej

### 3.3 Caching av semantisk sökning

**Problem:** ModernBERT (571 MB) laddas vid kallstart (~3.3s i release).
Varje MCP-anrop som trigar `sense` skulle betala denna kostnad.

**Lösning:** GTW behöver en långgående process (daemon eller connection pool)
som håller modellen i minnet. Förslag:

1. **Daemon-läge:** `gnawtreewriter daemon` som lyssnar på MCP-anrop
   och håller ModernBERT + BPE-cachen i minnet. Startas av Memory System
   vid projektanslutning.
2. **LazyLock:** Befintligt för tiktoken BPE. Utöka till ModernBERT-modellen
   (redan delvis gjort i `ai_manager.rs` med `Arc<Mutex<>>`).
3. **Preferens:** Memory System startar GTW-demon per projekt och håller
   anslutningen öppen.

### 3.4 Temporalt index: ocommittade ändringar

**Problem:** `temporal_index` baserat på git-logg missar lokala ändringar
som ej är committade.

**Lösning:** Komplettera med `git status` / `git diff` för att fånga
dirty working tree:

```
temporal_index:
  committed:    git log --since="2026-08-17" --until="2026-08-24"
  uncommitted:  git diff --name-only + git status --porcelain
```

Memory System kan då fråga: "vilka ändringar finns det inklusive
ej committade?"

### 3.5 Filövervakning

**Fråga:** Ska GTW ha inbyggd file-watcher?

**Svar:** GTW behöver inte driva övervakningen själv, men kan erbjuda
stöd om plattformen har det:

| Plattform | Mekanism | GTW-stöd |
|-----------|----------|----------|
| Linux | inotify / fanotify | `inotifywait` via CLI |
| macOS | FSEvents | `fswatch` via CLI |
| Cross-platform | `notify` crate (Rust) | `gnawtreewriter watch` |

Förslag: `gnawtreewriter watch --format json` som emitterar
filändringar som JSON-events:

```json
{"event": "modified", "path": "src/auth/login.rs", "timestamp": "2026-08-24T13:00:00Z"}
{"event": "created", "path": "src/auth/mfa.rs", "timestamp": "2026-08-24T13:00:01Z"}
```

Memory System kan prenumerera på denna ström och trigga
om-indexering automatiskt. Filövervakningens kärna bör ligga
i `notify`-cratet (Rust, cross-platform), inte i GTW:s kärna.

### 3.6 Inkrementell tracking och delta-rapportering

**Problem:** Utan stateмежду sessioner måste Memory System fråga GTW om
hela projektet vid varje anrop — onödig prestandakostnad.

**Lösning:** GTW sparar ett lättviktigt state-fil i projektroten:

```
.gnawtreewriter_state.json
{
  "last_analyzed": "2026-08-24T13:00:00Z",
  "git_head": "abc123def",
  "file_hashes": {
    "src/auth/login.rs": "a1b2c3...",
    "src/auth/session.rs": "d4e5f6...",
    "src/main.rs": "789abc..."
  }
}
```

**Nytt MCP-verktyg: `diff_since`**

```json
{
  "tool": "diff_since",
  "args": {
    "since_commit": "abc123def",
    "include_uncommitted": true
  },
  "result": {
    "since_commit": "abc123def",
    "current_commit": "xyz789abc",
    "changed_files": [
      {
        "path": "src/auth/login.rs",
        "status": "modified",
        "old_hash": "a1b2c3...",
        "new_hash": "x9y8z7...",
        "diff_stat": "+12 -5"
      },
      {
        "path": "src/auth/mfa.rs",
        "status": "added",
        "new_hash": "m1n2o3...",
        "diff_stat": "+87 -0"
      },
      {
        "path": "src/auth/legacy.rs",
        "status": "deleted",
        "old_hash": "p4q5r6..."
      }
    ],
    "uncommitted": [
      {
        "path": "src/auth/login.rs",
        "status": "modified",
        "diff_stat": "+3 -1"
      }
    ],
    "stats": {
      "files_added": 1,
      "files_modified": 1,
      "files_deleted": 1,
      "uncommitted_changes": 1
    }
  }
}
```

**Användningsflöde:**

```
Memory System: "Vad har ändrats sen index X?"
  → GTW diff_since(since_commit: "abc123")
  → GTW: "3 filer ändrades, 1 ocommittad"
  → Memory System: Uppdatera bara de filerna
  → GTW: analyze/endex_entities för de specifika filerna
```

**Fördelar:**
- Memory System betalar bara för analysering av ändrade filer
- Ocommittade ändringar fångas utan extra anrop
- Deterministiska fil-hasher (SHA256) gör att samma innehåll
  alltid får samma hash — inga dubbletter vid om-indexering
- State-filen är liten (~1KB) och kan versionhanteras

**Implementation i GTW:**
- Spara state efter varje `analyze`/`pack`-anrop
- `diff_since`: jämför HEAD, beräkna SHA256 per ändrad fil
- State-filen skapas automatiskt vid första körningen

---

## 3. GTW-exponerade MCP-verktyg

Dessa verktyg finns redan i GTW och behöver inga ändringar:

| Verktyg | Exponerar | Användning av Memory System |
|---------|-----------|----------------------------|
| `analyze` | Full AST-trädstuktur + token-count | Entitet-extraktion (funktioner, klasser, moduler) |
| `sense` | Semantisk sökning (vektor) | Semantisk index-frågor |
| `search_nodes` | Text-/namnsökning i AST | Keyword-index |
| `gnaw_find` | Mönster-sökning över projekt | Cross-fil entitetssökning |
| `inspect` | Call graph, callers/callees | Relations-index |
| `pack` | Komprimerad projekt-paketering | Kontext-leverans till agenter |
| `curate` | Intelligent filval | Relevansbaserad kontext |
| `compress` | AST-kompression (~70%) | Lagringseffektivitet |
| `stats` | Projektstatistik | Metadata och kontextfönster |
| `batch` | Atomära multi-fil-ändringar | Minnesuppdatering |

### Nya MCP-verktyg som behövs

Dessa behöver läggas till för att stödja full memory-index-integration:

#### `index_entities` — Extrahera entiteter från filer

```json
{
  "tool": "index_entities",
  "args": {
    "file_path": "src/auth/login.rs",
    "include_private": false
  },
  "result": {
    "entities": [
      {
        "type": "function",
        "name": "validate_password",
        "signature": "pub fn validate_password(password: &str) -> bool",
        "path": "0.1",
        "line": 12,
        "visibility": "public",
        "doc_comment": "Validates a password against policy requirements."
      },
      {
        "type": "struct",
        "name": "AuthConfig",
        "path": "0.5",
        "line": 45,
        "fields": ["max_attempts", "lockout_duration"]
      }
    ],
    "imports": ["crate::crypto", "std::time"],
    "exports": ["validate_password", "AuthConfig"]
  }
}
```

**Syfte:** Entitet-extraktion för knowledge graph. Memory-systemet lagrar
dessa entiter och kan sedan fråga "vilka funktioner använder `validate_password`?"

#### `index_relations` — Extrahera beroenden mellan entiteter

```json
{
  "tool": "index_relations",
  "args": {
    "file_path": "src/auth/login.rs",
    "direction": "both"
  },
  "result": {
    "relations": [
      {"from": "validate_password", "to": "check_password_hash", "type": "calls", "line": 18},
      {"from": "validate_password", "to": "AuthConfig", "type": "uses", "line": 15},
      {"from": "login_handler", "to": "validate_password", "type": "calls", "line": 5}
    ]
  }
}
```

**Syfte:** Bygga relations-grafer för knowledge graph. Motsvarar Graphlits
"Relationship Index".

#### `search_semantic` — Semantisk sökning (utökat sense)

```json
{
  "tool": "search_semantic",
  "args": {
    "query": "hur hanteras sessioner",
    "scope": "project",
    "include_compressed": true
  },
  "result": {
    "matches": [
      {"file": "src/auth/session.rs", "score": 0.89, "context": "..."},
      {"file": "src/auth/login.rs", "score": 0.72, "context": "..."}
    ]
  }
}
```

**Syfte:** Erbjuda semantisk sökning utan att behöva ange filnamn.
Motsvarar Graphlits "Semantic Index".

#### `temporal_index` — Git-baserat tidsindex

```json
{
  "tool": "temporal_index",
  "args": {
    "since": "2026-08-17",
    "until": "2026-08-24",
    "include_commits": true,
    "file_pattern": "src/auth/**"
  },
  "result": {
    "changed_files": [
      {"path": "src/auth/login.rs", "commits": 3, "last_modified": "2026-08-22", "authors": ["tuulikk"]},
      {"path": "src/auth/session.rs", "commits": 1, "last_modified": "2026-08-20", "authors": ["tuulikk"]}
    ],
    "summary": {
      "total_commits": 8,
      "files_changed": 5,
      "lines_added": 120,
      "lines_removed": 45
    }
  }
}
```

**Syfte:** Temporalt index baserat på git-historik. Motsvarar Graphlits
"Temporal Index". Kan svara på "vilka filer ändrades senaste veckan?"

---

## 4. Integrationsprotokoll

### 4.1 Handshake

När Memory System ansluter till en GTW-instans:

```
Memory System → GTW: {"tool": "stats", "args": {"path": "."}}
GTW → Memory System: { "total_tokens": 169252, "languages": [...], "file_count": 79 }
```

Memory System skapar en "project profile" baserat på stats.

### 4.2 Initial Indexering

Första gången ett projekt ansluts:

```
Memory System → GTW: {"tool": "analyze", "args": {"file_path": "src/main.rs"}}
Memory System → GTW: {"tool": "index_entities", "args": {"file_path": "src/main.rs"}}
Memory System → GTW: {"tool": "index_relations", "args": {"file_path": "src/main.rs"}}
Memory System → GTW: {"tool": "search_semantic", "args": {"query": "..."}}
```

### 4.3 Inkrementell Uppdatering

Vid filändringar (via file-watcher eller pollning):

```
Memory System → GTW: {"tool": "analyze", "args": {"file_path": "src/auth/login.rs"}}
Memory System: Uppdatera entitet/relations-index för den filen
```

### 4.4 Frågeflöde

När en agent frågar "vilka funktioner hanterar autentisering?":

```
Agent → Memory System: "vilka funktioner hanterar autentisering?"
Memory System → GTW: {"tool": "search_semantic", "args": {"query": "autentisering"}}
Memory System → GTW: {"tool": "search_nodes", "args": {"pattern": "auth", "type_filter": "function"}}
Memory System: Sammanställ, deduplicera, rangordna svar
Memory System → Agent: [lista med funktioner + kontext]
```

---

## 5. Dataformat för Memory System

### 5.1 Entitet

```json
{
  "id": "gtw:project_a:src/auth/login.rs:function:validate_password",
  "type": "function",
  "name": "validate_password",
  "signature": "pub fn validate_password(password: &str) -> bool",
  "project": "project_a",
  "file": "src/auth/login.rs",
  "line": 12,
  "visibility": "public",
  "doc": "Validates a password against policy requirements.",
  "tokens": 45,
  "last_indexed": "2026-08-24T13:00:00Z"
}
```

**ID-format:** `gtw:{project}:{file}:{entity_type}:{entity_name}`
- Deterministiskt — identiskt oavsett radnummer
- Möjliggör säker upsert utan dubbletter
- Filen i ID:t är relativ sökväg (ej absolut)

### 5.2 Relation

```json
{
  "id": "gtw:project_a:validate_password->check_password_hash",
  "type": "calls",
  "source": "gtw:project_a:src/auth/login.rs:validate_password",
  "target": "gtw:project_a:src/crypto/hash.rs:check_password_hash",
  "project": "project_a",
  "file": "src/auth/login.rs",
  "line": 18
}
```

### 5.3 Temporal Entry

```json
{
  "file": "src/auth/login.rs",
  "date": "2026-08-22",
  "commits": ["abc123", "def456"],
  "authors": ["tuulikk"],
  "lines_added": 45,
  "lines_removed": 12,
  "summary": "Added rate limiting to validate_password"
}
```

---

## 6. Användningsfall

### 6.1 Agentic Code Review

```
Agent: "Granska ändringar i autentiseringen senaste veckan"

Memory System:
  1. GTW temporal_index → hitta ändrade filer
  2. GTW index_entities → extrahera påverkade funktioner
  3. GTW index_relations → hitta anropande funktioner
  4. GTW compress → komprimera relevant kod
  5. Agent: Sammanställ granskningsrapport
```

### 6.2 Cross-Project Pattern Search

```
Agent: "Finns det redan en session-hantering i något av våra projekt?"

Memory System:
  1. GTW(search_semantic, project=A) → "session.rs"
  2. GTW(search_semantic, project=B) → "session_manager.rs"
  3. Memory System: Sammanställ mönster, visa likheter
```

### 6.3 Impact Analysis

```
Agent: "Vad händer om vi ändrar AuthConfig?"

Memory System:
  1. GTW(inspect, file=auth/config.rs) → callers
  2. GTW(index_relations) → deep caller graph
  3. GTW(stats) → token-count för påverkade filer
  4. Memory System: Rapport med påverkans-analys
```

---

## 7. Roadmap

### Fas 1: Befintliga verktyg (inga GTW-ändringar)
- Memory System ansluter till GTW via MCP
- Använder `analyze`, `sense`, `search_nodes`, `inspect`, `pack`, `stats`
- **Status:** Redan möjligt

### Fas 2: Nya MCP-verktyg (GTW-ändringar)
- `index_entities` — entitetsextraktion
- `index_relations` — beroendekartläggning
- `search_semantic` — utökad semantisk sökning utan filnamn
- `temporal_index` — git-baserat tidsindex
- `diff_since` — inkrementell delta-rapportering
- State-fil (`.gnawtreewriter_state.json`) för session-övergående tracking
- **Estimat:** 2-3 veckors utveckling

### Fas 3: Optimering
- Inkrementell indexering (endast ändrade filer)
- Caching av index-resultat
- Batch-frågor (flera filer i en anrop)
- **Estimat:** 1-2 veckor

### Fas 4: Distribution
- GTW som embedded library (inte bara CLI/MCP)
- FFI-bindningar för andra språk (Python, Node.js)
- **Estimat:** 2-4 veckor

---

## 8. Begränsningar

| Begränsning | Förklaring | Potentiell lösning |
|-------------|-----------|-------------------|
| Per-projekt | GTW hanterar ett projekt i taget | Memory System aggregerar |
| Session-baserat | GTW lagrar inget persistent | Memory System lagrar |
| ModernBERT-storlek | 571 MB modell laddas vid behov | Modell-caching, eller externt API |
| Inga cross-fil-anrop | GTW ser bara anrop inom ett projekt | Memory System bygger graffritt |
| Inget real-time | File-watcher saknas (ännu) | Pollning eller OS-notifikationer |

---

## 9. Rekommenderad Teknisk Stack

### För Memory System (externt):
- **Lagring:** PostgreSQL + pgvector (vektorer)
- **Graf:** Neo4j eller pg_graph (relationer)
- **API:** GraphQL eller REST
- **Agent:** LLM med MCP-stöd (Claude, GPT)

### För GTW (befintligt):
- **Parser:** Tree-sitter (redan)
- **Semantik:** ModernBERT (redan)
- **Sökning:** AST-traversering (redan)
- **Paketering:** pack/compress (redan)
- **Säkerhet:** secrets_scanner (redan)

---

## 10. Referenser

- Graphlit Memory Index: https://www.graphlit.com/glossary/memory-index
- GTW Roadmap: docs/ROADMAP.md
- GTW MCP: docs/MCP.md
- GTW AI-Friendly: docs/AI_FRIENDLY.md

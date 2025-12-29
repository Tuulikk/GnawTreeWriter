# GnawTreeWriter — Release Notes (v0.3.0) (DRAFT)

**Datum:** 2025-12-28  
**Status:** Draft — skapa en utkastrelease för taggen `v0.3.0` och använd innehållet i den här filen som release-beskrivning. Publicera inte paketet på crates.io ännu om du inte är redo.

---

## Sammanfattning

Release `v0.3.0` fokuserar på robusthet och användbarhet: sessionshantering gör arbetsflödet säkrare (ingen manuell `session-start` krävs längre), en inbyggd diff-vy gör förändringar lätta att granska, och generisk filstöd gör att *alla* filer i ett projekt kan backas upp och historieföras. Dessutom introduceras Named References ("tags") som stabiliserar automatiserade redigeringar genom att ge namn åt träd-paths.

---

## Höjdpunkter

### Implicit Sessions
- Sessions startas automatiskt vid första edit — inga fler "glömde session-start".
- Session-ID sparas mellan kommandon i `.gnawtreewriter_session_id`.
- `history`, `restore-session` och session-baserade operationer fungerar som tidigare, men UX är enklare för snabba redigeringar.

Användningsexempel:
```bash
# Första edit startar en default session automatiskt
gnawtreewriter edit test.py "0" 'def f(): pass'

# Kontrollera historik
gnawtreewriter history
```

---

### Built-in Diff View
- Nytt kommando: `gnawtreewriter diff`
  - `gnawtreewriter diff --last N` — visa diffs för de senaste N transaktionerna
  - `gnawtreewriter diff <txn_id>` — visa diff för en specifik transaktion
- Diffen baseras på backup-filer (före/efter) och använder content-hashes för pålitlighet.
- Om en transaktion saknar "after backup" (nyaste transaktionen), visas jämförelse mot aktuell fil istället.

Exempel:
```bash
# Visa diff för de senaste 3 transaktionerna
gnawtreewriter diff --last 3

# Visa diff för en specifik transaktion
gnawtreewriter diff txn_1766948638667822835
```

---

### Generic Parser (backup & editing för alla filer)
- `GenericParser` behandlar okända filtyper som en enda nod (path = `0`) så att:
  - Alla filer i projektet kan analyseras (`analyze`),
  - Alla filer backas upp i `.gnawtreewriter_backups/`,
  - Och de kan även redigeras som text-blobs när det är lämpligt.
- Detta gör GnawTreeWriter till ett fullständigt projektverktyg (inte bara för kända språk).

Exempel:
```bash
# Analysera och redigera en Dockerfile eller README
gnawtreewriter analyze Dockerfile
gnawtreewriter edit Dockerfile "0" 'FROM python:3.11' --preview
```

---

### Named References (Tags)
- Nya kommandon: `tag add/list/remove`
  - `gnawtreewriter tag add <file> "<path>" "<name>" [--force]`
  - `gnawtreewriter tag list <file>`
  - `gnawtreewriter tag remove <file> "<name>"`
- `edit/insert/delete` stöder shorthand `tag:<name>` i den positionella node-path-argumentet:
  - Exempel: `gnawtreewriter edit main.rs tag:my_function '...'`
- `tag add` validerar att den angivna pathen finns i filens AST.
- Taggar lagras i projektroten: `.gnawtreewriter-tags.toml`

Exempel:
```bash
# Skapa en tag
gnawtreewriter tag add test.py "0" "my_function"

# Redigera via inline-tag
gnawtreewriter edit test.py tag:my_function 'def my_function():\n  pass' --preview

# Lista taggar
gnawtreewriter tag list test.py

# Ta bort en tag
gnawtreewriter tag remove test.py "my_function"
---

### Batch Operations (Atomiska Multi-Fil Edits)
- Nytt kommando: `gnawtreewriter batch <json_file> [--preview]`
- JSON-baserad batch-specifikation för koordinerade ändringar över flera filer
- Stöder tre operationstyper: `edit`, `insert`, `delete`
- **Atomic Validation**: Alla operationer valideras i minnet innan någon skrivning sker
- **Unified Preview**: Visa alla ändringar över alla filer i en gemensam diff
- **Automatic Rollback**: Om en operation misslyckas, återställs alla skrivna filer från sina backups
- **Transaction Logging**: Varje filoperation loggas individuellt för undo-kapabilitet

Batch JSON-format:
```json
{
  "description": "Mänsklig läsbar beskrivning",
  "operations": [
    {
      "type": "edit",
      "file": "path/to/file.ext",
      "path": "node.path.here",
      "content": "nytt innehåll"
    },
    {
      "type": "insert",
      "file": "path/to/file.ext",
      "parent_path": "parent.node.path",
      "position": 1,
      "content": "innehåll att infoga"
    },
    {
      "type": "delete",
      "file": "path/to/file.ext",
      "path": "node.to.delete"
    }
  ]
}
```

Position-värden för insert:
- `0` = Topp (efter öppnande klammerparentes om det finns)
- `1` = Botten
- `2` = Efter egenskaper (QML-specifikt)

Exempel:
```bash
# Skapa batch-fil
cat > update.json << 'EOF'
{
  "description": "UI tema uppdatering",
  "operations": [
    {"type": "edit", "file": "main.qml", "path": "1.1.3.2.0.1", "content": "darkblue"},
    {"type": "insert", "file": "main.qml", "parent_path": "1.1", "position": 2, "content": "radius: 8"}
  ]
}
EOF

# Förhandsgranska (rekommenderas först)
gnawtreewriter batch update.json --preview

# Applicera atomiskt
gnawtreewriter batch update.json
```

**Nytta för AI Agenter:**
- ✅ Perfekt för multi-fil refaktorering och koordinerade ändringar
- ✅ Inbyggd säkerhet genom validation och rollback
- ✅ Enkel integration med befintlig undo/history-infrastruktur
- ✅ Se [BATCH_USAGE.md](BATCH_USAGE.md) för komplett dokumentation

---

## Uppdateringar i dokumentation & CLI-hjälp
- README: exempel och snabbkommandon uppdaterade med `tag`- och `batch`-exempel.
- AGENTS.md: rekommendationer om add-ons (LSP & MCP) som valfria utbyggnader; nytt batch-workflow exempel tillagt.
- ROADMAP.md: status uppdaterad till `v0.3.0` och Batch Operations markerat som implementerad.
- BATCH_USAGE.md: ny dokumentation för batch operations med komplett användarguide och exempel.
- CLI-hjälp uppdaterad: `gnawtreewriter tag --help` och `gnawtreewriter batch --help` beskriver nya kommandon.

---

## Kompatibilitet & Migration
- Ingen breaking change för kända arbetsflöden.
- Automationsskript som förlitar sig på numeriska paths kan göras mer robusta genom att byta till Named References (tags). Rekommendation:
  - Skapa taggar för viktiga målpunkter (`tag add`) och använd `tag:<name>` i skript.
- Genom att använda taggar får skript stabilare mål trots mindre AST-ändringar (insert/delete ovanför).
- Batch operations tillhandahåller ett säkert och kraftfullt alternativ för multi-fil automation med inbyggd rollback-kapacitet.

---

## Tester & Kvalitetssäkring
- Enhetstester för TagManager, Batch och övriga kärnfunktioner finns under `src` (`cargo test` kör alla tester — 19 tester passerade lokalt).
- Manual QA: grundläggande arbetsflöden testade (analys, edit, diff, tag add/list/remove, history/restore, batch operations).

---

## Publiceringsinstruktioner (för dig)
1. Kontrollera innehållet i den här filen.
2. Skapa en draft-release i din Git-host (taggen `v0.3.0` finns redan).
3. Kopiera innehållet i den här filen som release notes och markera releasen som **Draft** tills du är klar att publicera.
4. (OBS: Vi publicerar inte till crates.io ännu enligt din önskan — avvaktande.)

---

## Tack & Acknowledgements
- Tack till testern från Gemini CLI som hjälpte oss identifiera förbättringsområden.
- Tack för samarbetet — dessa förbättringar gör GnawTreeWriter både säkrare och mer användbart i verkliga AI-driven utvecklingsflöden.

---

**NOTERA:** Detta är en release-notes-fil (DRAFT). När du vill kan jag skapa GitHub-release-utkastet åt dig eller publicera det (om du ger klartecken).
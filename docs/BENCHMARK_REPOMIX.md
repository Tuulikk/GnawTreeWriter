# Benchmark: GTW vs Repomix

**Datum:** 2026-08-24
**Test:** Pack av `src/` (85 filer, ~164k tokens) på GTW-repot, 3 körningar.

## Resultat

| Metrik | GTW | Repomix 1.18.0 | GTW-fördel |
|--------|-----|----------------|------------|
| Pack (varm) | 0.26–0.34s | 1.35–1.44s | **~4x snabbare** |
| Pack + compress | 0.67s | 1.78s | **~2.7x snabbare** |
| Minne | ~39 MB | ~200 MB | **~5x mindre** |
| Komprimerad output | 189 KB | 404 KB | **2.1x mindre** |
| Token-reduktion | 69% (164k → 51k) | ~54% (uppskattat) | **GTW komprimerar hårdare** |

*Not: Repomix-siffror inkluderar npx-overhead (~0.3s). Även med avdrag är GTW 3–4x snabbare.*

## Komprimeringskvalitet (avgörande skillnad)

Sidovid-sida på `src/core/alf.rs` visar att Repomix **förlorar viktig struktur**:

| Element | GTW | Repomix |
|---------|-----|---------|
| `use`-statements | ✅ Bevaras intakta | ❌ Ersätts med `⋮----` |
| Struct-fält | ✅ Komplet definitioner | ❌ `pub struct X { ⋮---- }` |
| Indentation | ✅ Bevaras | ❌ Strips mellanslag |
| `#[derive(...)]` | ✅ Bevaras | ❌ Faller bort |
| Doc-kommentarer | ✅ Bevaras | ✅ Bevaras |
| Funktionskroppar | `⋮----` | `⋮----` (men partiell/bevarad omixat) |

**Exempel — samma struct:**

GTW:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlfEntry {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub actor: String,
    pub transaction_id: Option<String>,
    ...
}
```

Repomix:
```rust
pub struct AlfEntry {
⋮----
```

**Slutsats:** Repomix komprimerar radvis utan full AST-förståelse — tappar
fältdefinitioner och attribut som LLM:er behöver för att förstå API:er.
GTW:s Tree-sitter-baserade kompression bevarar kontrakt (signaturer, typer,
fält) och tar bara bort implementation.

## Metod

```bash
# GTW
gnawtreewriter pack src --format json --output gtw-pack.json
gnawtreewriter pack src --compress --format json --output gtw-pack-c.json

# Repomix
npx repomix --stdout --style json --include "src/**"
npx repomix --stdout --style json --compress --include "src/**"
```

Kördes på samma maskin, samma arbetskatalog, 3 iterationer vardera.

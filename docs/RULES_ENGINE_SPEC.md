# Rules Engine: semgrep-liknande mönster → strukturell AST-matchning

**Status:** Implementerad (v0.9.7) — alla 5 steg är byggda och verifierade
**Syfte:** Ge GTW en snabb, deterministisk mönsterbaserad analysmotor (Ruff/semgrep-liknande), och låta den förstärka den lokala LLM:ens edit-förmåga genom att injicera kodmönster-expertis i prompten och edit-loopen.
**Mål:** `lint` blir riktig lint; LLM:en får "gratis expertis" utan att byta modell.

---

## 1. Bakgrund & varför

- GTW bygger redan AST:er (tree-sitter, 26 språk) — samma grund semgrep/Ruff står på.
- `lint` idag är bara en parse-check ("lyckas filen parsas?"). Koden säger själv: *"Future: Add actual linting rules here"*.
- En liten lokal LLM (LFM2.5-1.2B) saknar kodmönsterkunskap i vikterna (vet inte att `except: pass` är en anti-pattern). Regler kan injicera den kunskapen.
- Tree-sitter 0.26 (redan dependency) har inbyggda queries (`Query`, `QueryCursor`) — samma mönsterspråk semgrep bygger på.

**Kärnprincip:** Regel-motorn är helt LLM-oberoende. LLM är bara två tunna lager ovanpå: en *konsument* (får träffar i prompten) och en *producent* (skriver nya mönster som motorn validerar).

---

## 2. Arkitekturöversikt

```
semgrep-liknande mönster ("$X = $X")
        │  kompileras (en gång, cachelagras)
        ▼
tree-sitter query
        │  körs mot varje fils AST (rayon, parallellt)
        ▼
träffar: [rad, kol, mönster, message, severity]
        │
        ├──► lint (standard): rapport till människa/JSON
        ├──► edit-guardian: blockera/varna vid regelträff på ny kod
        ├──► prompt-kontext: annoterad kod in i LLM-prompten
        └──► regel-generering (LLM): modellen skriver mönster → motorn validerar
```

**Nyckel:** Analysen är gratis (ingen modell, inga tokens). LLM:en ser bara färdiga träffar.

---

## 3. Regel-format

Högnivå-format som en liten LLM kan skriva, matchas strukturellt mot källträdet (se avsnitt 4).

```yaml
# examples/rules.yaml (förslag)
rules:
  - id: redundant_return
    language: rust          # krävs i v0.1 (beslut 1)
    severity: warning       # error | warning | info
    message: "Redundant return statement"
    pattern: "return $X;\nreturn $X;"   # semgrep-liknande kodmönster
```

**Fält:**
- `id` — unikt namn (används för filter: `lint --rule redundant_return`)
- `language` — vilket språk regeln gäller för (krävs i v0.1)
- `severity` — `error` | `warning` | `info`
- `message` — vad som rapporteras till användaren
- `pattern` — kodmönster med `$VARIABLE`-platshållare (semgrep-stil)

**Mönster-syntax (semgrep-liknande):**
- `$X` — valfri nod (identifierare, uttryck, sats)
- `$X = $X` — självtilldelning
- `$FUNC(...)` — anropsmönster
- Bokstavlig kod matchar exakt (modulo whitespace)
- Kommentarer efter `#` tillåts i pattern (ignoreras vid kompilering)

**Vad som INTE ingår i v0.1:** `pattern-inside`, `pattern-not`, `metavariable-regex`, `fix`-förslag. Dessa är framtida steg.

---

## 4. Kompilator: mönster → matchning

Den svåraste och viktigaste delen. Mönstret är *kod* (semgrep-stil), inte en query.

**Vald strategi: strukturell matchning i Rust (inte query-generering).**

Att generera en query-sträng per språk (`(assignment left: (identifier) @left ...)`) kräver språkspecifik kunskap om hur varje nodtyp ser ut — bräckligt och svårt att underhålla. Istället:

1. **Parsa mönstret** med samma parser som målspråket → ett mönster-AST.
2. **Fånga platshållare:** `$X`-noder markeras i mönster-AST:en (namn + position).
3. **Matcha strukturellt:** rekursivt jämför mönster-AST:n mot källträdets noder:
   - Samma nodtyp → match
   - `$X` → matchar vilken nod som helst (fångas)
   - Barn matchas i ordning (med whitespace-normalisering)
4. **Bindning av `$X`:** När samma platshållare (`$X`) förekommer flera gånger i ett mönster (`$X = $X`), måste alla förekomster fånga noder med **identiskt textinnehåll** — annars ingen träff. Det fångar "självtilldelning" korrekt och undviker falska positiva (`a = b` matchar inte `$X = $X`).
5. **Träff = (nod, rad, kol, fångade $X:n).**

**Fördelar:**
- Ett matchnings-skelett för alla språk (noder jämförs efter typ + struktur, inte namn).
- Ingen språkspecifik query-generering.
- Lättare att utöka med `pattern-inside`, `pattern-not` senare (samma matcher).

**Nackdel (accepterad):** Långsammare än en kompilerad query per fil — men träd matchas i minnet, mikrosekunder per regel, och vi kör parallellt med rayon.

**Whitespace:** Mönster- och källkod normaliseras före matchning: radbrytningar normaliseras, överflödiga blankrader trimmas. Tree-sitter ignorerar mellanslag i trädet ändå.

**Kompilering misslyckas rent:** Om mönstret inte parsar → tydligt fel vid laddning av regeln, aldrig tyst.

**Cache:** Parsade mönster-AST:er cachas per (rule_id, language) i sessionen.

---

## 5. Standard-körning: `lint`

```
gnawtreewriter lint <path> --rules <rules.yaml> [--severity error] [--format json]
```

- **Utan `--rules`:** inbyggda regler (ett fåtal per språk som start).
- **Utan något LLM:** helt deterministisk, ingen modell, ingen token-kostnad.
- Körs parallellt med rayon över filer (redan befintligt mönster).
- Output: människo-vänlig lista eller JSON (GNAW_JSON).
- `--severity`-filter, `--rule <id>`-filter.

**Inbyggda start-regler (förslag):**
- Rust: `unwrap()` utan kontext, redundant `return`, `let _ = ...`
- Python: `except: pass`, `eval(`, breda `except Exception`
- JS/TS: `console.log` kvar i prod, `==` istället för `===`

---

## 6. LLM-integrationer (ovanpå motorn)

### 6a. Edit-guardian (Duplex Loop 2.0)
- Efter att `edit`/`edit --ask` applicerar en ändring, kör reglerna på den *nya* koden.
- Regelträff → blockera (severity=error) eller varna (warning).
- Från "parsar det?" till "är det bra kod?".

### 6b. Prompt-kontext (LLM ser problemen)
- Innan LLM:en föreslår en edit, annotera berörd kod med regelträffar:
  ```
  ⚠️ redundant_return (warning): return x; return x;
  ```
- Injicerad i prompten — LLM:en kan undvika/fixa problemen.
- **Noll extra token-kostnad för själva analysen** (träffarna är redan beräknade).

### 6c. Regel-generering (det unika)
- `lint --discover`: LLM:en analyserar kodbasen och föreslår projekt-specifika regler.
- GTW validerar varje förslag (kompilerar mönstret, kör det, kollar att det matchar ≥1 gång), avvisar dåliga med felmeddelande.
- Sparas till `gnawtreewriter.rules.yaml` i projektroten.
- "Det här projektet glömmer alltid hantera X" → genererad regel → körs på allt.

---

## 7. Stegindelning (alla ✅ implementerade i v0.9.7)

| Steg | Innehåll | Status |
|---|---|---|
| **1** | Regel-motor: format, kompilator, `lint --rules`, inbyggda regler | ✅ |
| **2** | Edit-guardian: regler på ny kod efter edit | ✅ |
| **3** | Prompt-kontext: regelträffar in i LLM-prompten | ✅ |
| **4** | Regel-generering: `lint --discover` + `rules add`/MCP `add_rule` | ✅ |
| **5** | Rule-guided multi-edit: `edit --ask --all` | ✅ |

**Steg 1-2 är LLM-oberoende.** Steg 3-5 kräver `mamba`-featuren.

---

## 8. Beslut (spikade för v0.1)

1. **`all`-språk:** Kräv explicit `language` i v0.1. `all` tillåts inte — en regel måste ange språk. Det håller matchningen deterministisk och undviker fuzzy-matchning mot okända AST:er. (Framtida steg kan lägga till `all` med tyst skip för språk som inte parsar.)
2. **Whitespace:** Normalisera mönster- och källkod före matchning: radbrytningar normaliseras, överflödiga blankrader trimmas. Tree-sitter ignorerar mellanslag i trädet ändå — vi normaliserar bara texten vi parsar.
3. **Regel-fil-plats:** Hybrid. En kärna av standardregler bakas in i binären (`include_str!` från en `rules/`-katalog) så `lint` fungerar out-of-the-box. Projekt-regler läses från `gnawtreewriter.rules.yaml` i projektroten och adderas/överskriver inbyggda.
4. **Kommentarer i pattern:** Stöd enkelt — rader som börjar med `#` filtreras bort från mönstersträngen innan parsing. Gör YAML-regler trevligare att skriva.
5. **Relation till befintlig `lint`:** Komplettera. Parse-checken ligger kvar som grund (körs först, avbryter filen vid syntaxfel), regelmotorn är det semantiska/strukturella lagret ovanpå.
6. **Severity i edit-guardian:** Konfigurerbart med hård ventil. Default: `error` blockerar en edit, `warning` släpper med varning i prompt-kontexten. `--guardian-severity` sätter gränsen.

---

## 9. Ej i scope (v0.1)

- `pattern-inside` / `pattern-not` / `fix` / metavariable-regex (semgrep-avancerat)
- Autofix (semgrep `fix`-fält)
- Taint/flow-analys
- Import av hela semgrep-rules-biblioteket (för många formatvarianter)

---

*Status: Implementerad i v0.9.7. Framtida utbyggnad: `pattern-inside`/`pattern-not`/`fix` samt `curate`-digest, `review`, `impact`, `query` (se ROADMAP Phase 8).*

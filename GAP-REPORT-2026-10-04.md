# GAP-REPORT 2026-10-04 — vad GTW behöver fixas för att bli agenternas återkommande verktyg

> Källa: Motor2-agent-session 2026-10-04 (användarens uppdrag). Den som
> kör denna rapporten är separat agent i GTW-projektet. Bakgrunden: Motor2-
> agenter använder GTW nästan aldrig (diagnos/sökning/edit) trots §35
> ("GTW först"), medan gnawdriver används dagligen. Användarens bedömning:
> "något är fel på gtw" — den bedömningen stämmer: tre konkreta luckor +
> ett adoptionsfel. Rapporten är prioriterad i den ordning.

---

## Lucka 1 (högst): MCP-verktyget `gtw_lint` saknas — annonsen finns redan

**Bevis (§3-annons utan verktyg):**
- `src/cli.rs:3397` — katalogen reklamerar redan:
  `{"name": "lint", "tool": "gtw_lint", "write": false, "desc": "Find issues in files"}`
- CLI-kommandot finns: `Commands::Lint` (`src/cli.rs:414`, handlar `src/cli.rs:4598ff`
  med `--recursive`, `--rules`, `--severity`, `--rule`, `--discover`).
- `src/mcp/mod.rs` (tool-registry `src/mcp/mod.rs:133-489`) har **ingen**
  `lint`-post. Kontrollerat 2026-10-04: `rg '"lint"' src/mcp/mod.rs` → 0 träffar.

**Varför detta är viktigast:** `$X`-mönstersökning över hela projektet
(ast-grep `run -p`-ekvivalenten) går idag bara via CLI. MCP-agenten måste
falla tillbaka på bash eller egen grep — och greppa gör redan det GTW
ska vara bättre på. Utan `gtw_lint` i MCP finns inget sätt för en agent
att köra GTW:s regler utan att lämna MCP-flödet.

**Bygg:**
1. Lägg tool-post i `src/mcp/mod.rs`-registret + handlar:
   - `paths: string[]` (filer/kataloger), `recursive: bool` (default true),
     `rules_file: Option<string>`, `severity: Option<string>`,
     `rule_id: Option<string>`,
   - **nytt: `pattern: Option<string>` + `language: Option<string>`** —
     ad-hoc-sökning: kompilera ett tillfälligt rule i minnet från
     mönstret (report-only), så agenter kan göra `ast-grep -p`-sökning
     utan att skapa en regel först.
   - Returnera structurerat: `findings: [{file, rule_id, severity,
     message, line?, fix?}]` (fix-text om regeln har `fix:` — se Lucka 2).
2. Delad funktionskärna med CLI:handlern (bygg `handle_lint`-logiken till
   en `run_lint(paths, rules, filters) -> Vec<Finding>` som båda anropar —
   §36: crate, inte subprocess — MCP och CLI får inte divergera).
3. Skriv om annonsen på `cli.rs:3397` om verktgnamnet ändras.

**Tester:** fixture-fil med kända regelträffar → MCP-anrop → assertions på
finding-listan (fil, rule_id, radnummer). Ad-hoc-pattern-söket får minst
ett test.

---

## Lucka 2: regler kan bara RAPPORTERA — `fix:` saknas (ast-grep-paritet)

**Bevis:**
- `src/core/rules.rs:35` — `pub struct Rule { … pub message, pub pattern … }`
  — inget `fix`-fält. (`rg 'fix' src/core/rules.rs` → 0.)
- `src/cli.rs:646` — `RuleSubcommands { Add, List }` — ingen fix-stödspår.

**Vad som saknas:** semgrep/ast-grep-liknande `fix:` — en regel bär sin
ersättning och `lint --fix` applicerar den på alla matcher. GTW:s batch-
motorik (`BATCH_USAGE.md`, `--preview`, undo/transaktionslogg) finns redan;
reglerna kopplas bara aldrig till den.

**Bygg:**
1. `Rule.fix: Option<String>` i `src/core/rules.rs` (+ YAML-nyckel `fix:`).
2. `gnawtreewriter rule add --fix "…"` (`src/cli.rs:646ff`) — validera:
   pattern kompilerar OCH fixen parsar som giltig kod för språket
   (återanvänd `compile_rule`-maskineriet; fixen får `$X`-metavariabler
   som bindas från matchen).
3. `lint --fix` i `handle_lint` (`src/cli.rs:4598ff`):
   - **default = rapport-only (nuvarande beteende oförändrat)**
   - `--fix` utan `--preview` → applicera via batch-motoriken
   - `--fix --preview` → visa diff, skriv inte
   - ALDRIG skriv utan explicit flagga.
4. Updatera `gnawtreewriter.rules.yaml`-dokumentationen + `BATCH_USAGE.md`.

**Tester:** (a) regel med fix + match → förväntad ersättningstext;
(b) `lint --fix --preview` skriver INTE; (c) `lint --fix` skriver + `undo`
återställer; (d) ogiltig fix → `rule add` avvisas.

---

## Lucka 3 (adoption — skälet Motor2-agenter inte använder GTW): tomma
schemas + beskrivningar som inte lär ut

**Bevis:**
- `src/mcp/mod.rs:488` —
  `{ "name": "batch", "description": "Apply batch", "inputSchema": {"type":"object"} }`
  — tom schema. Agenter vet inte JSON-formatet utan att först läsa
  `BATCH_USAGE.md` (extra steg → friktion > upplevd nytta → används inte).
- Övriga tool-beskrivningar = substantiv-rader: "Undo", "Apply batch",
  "Find nodes containing specific text pattern". De säger INTE när man
  ska nå dem jämfört med vanliga Read/Grep/Edit.

**Jämförelsemönster att kopiera (gnawdriver, som Motor2-agenter faktiskt
använder dagligen):** långa imperativa beskrivningar som säger VAD,
NÄR, VAD DEN RETURNERAR och varför det är bättre än alternativet —
t.ex. "Sessions-läget i ETT anrop: turn.state, last_snippet … Beviset på
att en tur levererat — läser detta, gissar inte."

**Bygg (för varje MCP-tool):**
1. Fullt `inputSchema` med `properties` + obligatoriska fält + exempel på
   `batch`, `edit_node`, `insert_node`, `search_nodes`, `sense`,
   `semantic_edit` minst (de konkurrerar direkt med Edit/Grep).
2. Beskrivningsmall: **VAD** (1 mening) + **NÄR att nå den** (konkret
   jämförelse: "före Read+Edit på strukturella ändringar — node-path
   skyddar grannkod; Grep ser inte syntax") + **returnerar** + **liten
   exempelrad**.
3. Kontrakts-test (§42-mönstret): varje tool-post i registret måste ha
   `inputSchema` med minst ett property OCH en beskrivning ≥ 100 tecken —
   annars testet rött.

**Dokumentation:** uppdatera `GTW_INSTRUCTIONS.md` med den korta
diagnoskedjan agenter ska öva: `analyze`/`get_skeleton` (vad är filen)
→ `sense`/`search_nodes` (var bor X) → `read_node` (källkoden) →
`edit_node`/`semantic_edit` (ändra) → `gnaw-diff` (evidens).

---

## Lucka 5: källcitat saknas i `investigate`/`get_semantic_report` (punkt 6
 — "fixat 2026-10-01" ≠ helt fixat)

**Vad 10-01-fixen faktiskt gjorde (3afa725):** stabilitet — shared sense
broker, per-request panic isolation, tydligare satellite-svar. **Inte**
källcitat.

**Vad redan finns:** `sense`-svaret bär provenans i den strukturerade
delen — `FileMatch { file_path, node_path, score }` +
`NodeMatch { path … }` (`src/llm/gnaw_sense.rs:84-92`). Det är tillräckligt
för billig verifiering (agenten kan `read_node` på en nod i stället för
att greppa om).

**Vad som saknas:**
1. `investigate` (`src/cli.rs:2692`) och `get_semantic_report` svarar
   fri-text via lokal modell **utan** strukturerad `sources`-lista —
   exakt den tillitsluckan som gör att agenter verifierar med grep ändå
   (dubblerat arbete). Chunks bär redan `file_path`/`node_path`
   (`src/llm/semantic_index.rs:9`) — ytan finns, den exponeras bara inte.
   Åtgärd: appenda `sources: [{file, node_path, lines?}]` på alla
   LLM-genererade svar; kontrakts-test som assertar att `sources` finns
   OCH pekar på rätt fil i en fixture.
2. `FileMatch` saknar `content_preview` — poäng utan innehåll tvingar ett
   extra `read_node` för att bedöma relevans. Lägg till (nice-to-have).
3. **Live-verifiering av 10-01-fixen saknas från Motor2-sidan** — ingen
   post i issue-loggen bekräftar en lyckad körning efter fixen. Motor2-
   agenten kört ett prov (sense via MCP, känd fråga, rätt fil+nod, ingen
   timeout) och loggar resultatet i issue-loggen. Koordinera: fixen räknas
   inte som stängd förrän det provet finns.

---

## Lucka 4: öppna poster ur GTW_MCP_ISSUE_LOG.md (stäng medan ni ändå
 är där)

1. **2026-10-01 (II): `get_skeleton` tomt svar på 856-raders JS-fil** —
   bedömd som bugg, ej åtgärdad. Tomt svar får ALDRIG vara tyst: returnera
   fel eller begränsat skelett + degenereringssignal (t.ex.
   `truncated: true`) vid stora träd. (`GTW_MCP_ISSUE_LOG.md:188-207`.)
2. **2026-09-30-postens triage-checklist är halvfärdig** — roten fanns
   (2026-10-01: panik/per-anrop-model reload/obegränsad JIT-cache, åtgärdad)
   — stäng checklistorna ordentligt så nästa agent inte gör om grävandet.
3. **"Kvar i registry" ur UTF-8-audit (loggen:236-237):** lägg till
   lint-regeln som flaggar `&X[..N]` byte-slices på strängar — ett
   `rule add` (som dessutom är rökprov på rule-flödet).
4. Fundera på en `status`/`doctor`-MCP-post (CLI har redan `Status`/
   `Doctor`, `src/cli.rs:48ff`) — agenter ska kunna diagnosera "är GTW
   vid liv" på ett anrop innan de ger upp (2026-09-30:s tre timeouts
   kostade GTW användandet i praktiken).

---

## Lucka 6: GTW måste vara MER GUIDANDE och REDOVISANDE (användarens krav
 2026-10-04)

GTW ska inte bara fungera — det ska **hitta till agenten** och **peka på
nästa steg**. Tre mekanismer, i prioritetsordning:

1. **Skill (opencode-instruktionsladdning).**
   - **Registreringen gjordes 2026-10-04 av Motor2-agenten** (användaren
     hade gett samma instruktion ≥1 gång tidigare utan att den utfördes
     eller verifierades — §3-mönstret: "sagt" ≠ "gjort"). Frontmattern
     gjordes triggbar (situationsord på engelska + svenska) och
     `~/.config/opencode/skills/gnawtreewriter/SKILL.md` är en symlink
     till repots `SKILL.md` — innehållet lever i GTW-repot, ingen drift.
     Kräver opencode-omstart; verifieras via att `gnawtreewriter` syns i
     `available_skills`.
   - **Kvar för denna rapport-agent:** innehålls-passet i SKILL.md:diagnoskedjan
     (analyze→sense→read_node→edit_node), situations-tabellen (vilket
     verktyg vid vilken situation), fallback-regeln (timeout → omväg +
     logg i GTW_MCP_ISSUE_LOG.md), och ett fullständigt exempelanrop per
     verktyg. Idag är den ett generikt "prefer GTW"-manifest skrivet för
     Crush — den behöver bli agentens situations-handbok.
2. **Kontextmeddelanden till agenten i given situation.** Varje
   SITUATION och varje FEL ska bära guidance:
   - **Felmeddelanden:** mönstret finns redan i `sense`-satelliten
     ("no matches … build it with `gnawtreewriter ai index`, or pass
     file_path") — det är rätt stil. Gå igenom ALLA `tool_error`-vägar i
     `mcp/mod.rs` och gör samma: vad hände + exakt vad agenten ska göra
     istället. Inga råa felsträngar.
   - **Tool-beskrivningarna är agentens "meddelanden i situationen"** —
     de skrivs om enligt Lucka 3-mallen (VAD + NÄR + RETURNERAR +
     EXEMPEL) så att rätt verktyg syns INNAN agenten väljer verktyg.
   - Överväg `annotations`/title-fält per tool (får agentens tool-lista
     att tala: "Diagnos: var bor X" istället för "sense").
3. **AGENTS.md-bitar (GTW-repets egen + Motor2-repets §35).**
   - GTW: en "för agenter som ANVÄNDER GTW"-sektion: diagnoskedjan,
     noterings-taxonomin (se `GTW_MCP_ISSUE_LOG.md`-mallen — sex
     bedömningsrader: bugg / saknad funktion / sub-funktion saknas /
     svår att nå / inte hittad vid behov / under förmåga), och
     eskalationsregeln (≥3 i samma klass → kö).
   - Motor2: §35 omskrivs av Motor2-agenten efter denna rapport
     (beslutstabell, inte mandat — se rapportens del "b" i
     motor2-sessionens plan).

**Noteringsregeln (användaren 2026-10-04):** när GTW sviker — i
VILKEN kategori som helst ovan — postas det i issue-loggen. Särskilt
de två nya: *sub-funktion saknas för situationen* och *inte hittad
vid behov* (upptäcklighets-brister syns bara om de loggas när de
träffar). Loggen är taxonomin, taxonomin är inköpslistan.

---

## Definition of done (per lucka)

- [ ] `cargo build` + `cargo clippy -- -D warnings` rent
- [ ] Tester gröna inkl. nya (beteendetest, inte existens-test —
      Motor2 AGENTS §4 gäller även här: verifiera att datan flödar
      MCP-in → finding-lista ut, och att fix inte skriver utan flagga)
- [ ] `GTW_INSTRUCTIONS.md` uppdaterad (nya verktyg + diagnoskedjan)
- [ ] `GTW_MCP_ISSUE_LOG.md` uppdaterad (stängda triager + ny mall-taxonomin
      på plats — den ligger redan i filen, ändra inte bort den)
- [ ] Lucka 6: skill registrerad och testbar (finns i `skills.paths` eller
      `~/.config/opencode/skills/`, triggas på situationsord), alla
      `tool_error`-vägar i `mcp/mod.rs` innehåller nästa-steg-guidance
      (genomgång + stickprov: minst sense, batch, edit_node, lint)
- [ ] Motor2-sidan: §35/adoption-utrullning sköts av Motor2-agenten
      separat — GÖR INTE den delen här, rapporten är GTW-sidan.

## Referenser

- `GTW_MCP_ISSUE_LOG.md` (felogg med ofullständig triage)
- `BATCH_USAGE.md` (batch-JSON-format, `quick`, `diff-to-batch`)
- `gnawtreewriter.rules.yaml` + `rules/builtin.yaml` (regelformat)
- Motor2 `AGENTS.md` §35 (filredigeringsdisciplin, "GTW först"),
  §4 (testkvalitet), §20 (inert infrastruktur — annonsen `gtw_lint`
  utan verktyg är exakt det mönstret), §36 (crate, inte subprocess)
- ast-grep som jämförelsepunkt: `run -p MÖNSTER -r ERSÄTTNING
  --update-all`, regelfält `fix:`, `--debug-query`

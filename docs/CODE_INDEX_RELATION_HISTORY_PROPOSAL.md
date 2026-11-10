# Förslag: revisionsbunden relationsgraf och ändringspåverkan i code-index

**Status:** förslag, inte implementerat · **Datum:** 2026-11-09 · Samma dags-granskning: **fas 1 godkänd att starta**; beslut D1–D10 + tillägg infogade (spår: "Granskningsbeslut" sist i dokumentet)

**Ägare:** GnawTreeWriter (GTW) · **Motiv:** Motor2:s återkommande integrationsmissar

**Befintlig grund:** `docs/CODE_INDEX_SQLITE_DESIGN.md`,
`docs/CODE_INDEX_SCAN_TOOLS_PROPOSAL.md` och GTW:s transaktionshistorik.

## Beslut som förslaget söker

Använd samma `.gnawtreewriter_ai/code_index.db` för en revisionsbunden
relationsgraf. Efter en edit ska GTW kunna svara: *vilka relationer ändrades,
vilka anropare/konsumenter kan påverkas, vad är okänt och vilket bevis behövs?*
Svaret ska fungera både för GTW-editer och för ändringar gjorda av andra
verktyg. Det får inte kalla en relation "trasig" enbart för att AST-indexet
inte kan lösa den; kompilator, typkontroll och beteendetest avgör det.

## Nuläge, kontrollerat 2026-11-09

- `src/core/code_index_db.rs` lagrar aktuella `files`, `items`, `entities`,
  `relations`, `symbols` och en `staleness_queue`. `entities.node_path` kopplar
  grafen till GTW:s AST. `relations.to_name` finns alltid, men `to_id` sätts
  bara vid entydig upplösning. Refresh ersätter berörda rader; äldre grafer
  finns inte kvar i databasen. `meta.last_refresh_rev` är en senaste-stämpel,
  inte historik.
- `relations` har bara `idx_relations_file` (`code_index_db.rs:91`) — inga
  index på `to_name`/`to_id`. Dagens `ImpactAnalyzer::analyze_impact`
  (`src/llm/impact_analyzer.rs`) laddar dessutom alla grafer och filtrerar
  i minnet. Nya index gör **ingen** skillnad för den vägen förrän den
  ersätts med en riktad SQL-fråga. Mät båda vägarna på Motor2-storlek
  före ett prestandapåstående; schema v4 kan lägga index som den uppmätta
  frågan faktiskt använder (D3).
- `staleness_queue` har en rad per fil och töms vid lyckad refresh. Den är en
  arbetskö, inte en editlogg. GTW:s separata transaktionslogg
  (`src/core/transaction_log.rs`) har transaktions-ID, operation, fil,
  nodväg och före-/efter-hash. Koppla den som proveniens i stället för att
  duplicera dess innehåll i indexet. Git-revision och dirty-state behövs
  för ändringar utanför GTW.
- `signature_impact_for` i `src/core/mod.rs` ger påverkan vid
  `SignatureChange` med kända anropare. En kroppsedits ändrade anrop,
  borttagna läsare, flyttad state-effekt eller nya kodvägar syns inte som
  ett före-/efter-facit. Tomt impactsvar betyder inte noll påverkan.
- `src/core/index_relations.rs` extraherar `calls`, `uses`, `imports` och
  `implements`. `find_calls_in_scope` registrerar identifierare under
  `call_expression`; dess egen kommentar säger att metodanrop och makron
  är andra nodtyper och inte når grenen. Upplösningen bygger främst på
  namn och samma fil, inte fullständig Rust-modul-/typsemantik.
- Ett representativt Motor2-index innehöll 19 729 relationer, varav 4 818
  hade `to_id` (24,4 %). Per typ: `calls` 2 807/10 361 (27,1 %), `uses`
  1 886/8 234 (22,9 %), `imports` 65/877 (7,4 %), `implements` 60/257
  (23,3 %). Detta är **upplösningsgrad för extraherade relationer**, inte
  recall mot alla verkliga relationer. Metodanrop som inte extraherats finns
  inte ens i nämnaren. Siffrorna är ett startvärde, inte ett kvalitetsbetyg.
- Av 10 361 extraherade `calls` hade 3 832 ett `from_id` utan matchande
  `entities.id`; 460 av dessa var explicita syntetiska callsites. Resten
  behöver klassificeras (t.ex. testfunktioner, JS-funktioner och
  impl-/trait-metoder) före entitetsbaserad deltaidentitet. En okänd källa
  får inte försvinna ur påverkansrapporten (D7).

## Förväntat beteende

För varje indexerad ändring returneras en maskinläsbar rapport med:

1. **Identitet:** före-/efter-filhash, git-HEAD + dirty-state, GTW:s
   transaktions-ID när det finns, index-generation, extraktorversion och
   parserstatus. Rapporten får inte jämföra två grafer byggda med olika
   extraktorversioner utan att märka det.
2. **Delta:** tillagda, borttagna och ändrade entiteter och relationer,
   grupperade per symbol och fil. `node_path` är ett ankare giltigt vid
   angiven filhash; ett senare edit-verktyg måste återvalidera nodens namn.
3. **Påverkan:** inkommande relationer till ändrade mål, nya eller förlorade
   utgående relationer, möjliga konsumenter i andra filer och berörda tester.
   En body-edit utan signaturändring kan alltså också ge påverkan.
4. **Osäkerhet:** olösta/ambigua/externa mål, parser-varningar, saknade
   språk-/nodtyper och avbrott i historiken. Varje kandidat bär orsak och
   tillitsnivå (`exact`, `candidate`, `unknown`), aldrig tyst `0`.
5. **Verifieringsväg:** exakta filer och tester att granska/köra. Rapporten
   skiljer mellan grafisk påverkan, kompilatorfel, testfel och ännu oprövad
   beteendeeffekt. Bara observerad verifiering får kallas "bruten".

## Lagring: nulägesgraf + revisionsdeltan i samma SQLite-fil

Behåll dagens tabeller som snabb **nulägesvy**. Lägg till versionerade,
append-only tabeller i samma DB, exempelvis:

```text
index_revisions(id, parent_id, git_rev, dirty, source, transaction_id,
                generation_key, started_at, completed_at, status)
file_versions(revision_id, path, before_hash, after_hash, parse_status)
entity_changes(revision_id, entity_key, change_kind, before_json, after_json)
relation_changes(revision_id, relation_key, change_kind,
                 before_json, after_json, resolution_status)
```

Detta är en skiss, inte låst SQL-schema. Nycklarna måste tåla radförskjutning,
omdöpning och flera lika anrop på olika platser. Använd fil + ägande entitet
+ relationstyp + mål som identitetsunderlag; behandla osäker matchning som
delete/add, inte som säker rename. När källentiteten saknas, använd ett
explicit `unknown_source`/syntetiskt ankare i stället för att tappa länken.

**Deltats granularitet (D1, låst riktning):** deltaenheten är det
*aggregerade* `(from_entity, relation_type, target)` — med `count_before`,
`count_after` och callsite-ankare på båda sidor. Annars försvinner en
borttagen länk när två anrop från samma entitet till samma mål blir ett.
Callsite-rader är ankaren, inte huvudnyckeln. `node_path` är annotering,
aldrig nyckelkomponent:
B1 visade att paths går stale vid varje edit, så path-identitet skulle göra
varje body-edit till mass-delete/add-brus som dränker signalen. Rapportering
och påverkansfrågor aggregerar till entitetsnivå; råa callsite-deltan kan
följa som appendix. Aggregatet ska rapportera `count_changed` även när
relationens existens före/efter är oförändrad (D8).

**Identitetens egen version (D2):** `identity_scheme_version` ingår i
`generation_key`/`meta` — ett byte av nyckelalgoritmen är historikbrott i
samma klass som extraktorbyte (ny baseline). Idag skyddar generation_key
bara extraktorversionen.

Spara bara deltan och nödvändiga före-/efterfält, inte fulla AST-träd per
revision. En komprimerad checkpoint kan återuppbygga valda historiska lägen
utan att varje edit får en full grafkopia.

En refresh ska läsa föregående rader, parsa ändrade filer och lösa mål mot
den nya helheten. **Publiceringsinvariant (D5):** en läsare ser antingen hela
föregående revisionen eller hela den nya, aldrig en blandning. Två tillåtna
vägar: (a) en SQLite-transaktion för revision + delta + nulägesgraf, eller
(b) per-fil-arbete i staging/versionerade tabeller följt av ett atomiskt
byte av `active_revision_id`. Per-fil-transaktioner direkt mot dagens
publicerade nulägestabeller är inte tillåtna. Benchmarka (a) och (b) på en
Motor2-storlek fixtur; välj den enklaste som håller läsar- och låsbudgeten.
Vid misslyckad parsing/skrivning bevaras senast publicerade graf och
staleness-kön. P5:s befintliga busy-fallback får fortfarande skapa en
direkt Markdown-projektion, men den är **inte** en historikrevision:
`history_gap`/`index_stale` måste synas i CLI, MCP, doctor och varje
påverkansrapport. Den får aldrig ge grön ändringsgrind. Gör omindexering
efter schema-/extraktorbyte till en
**ny baseline**, med ett uttalat historikbrott, så en implementationändring
i extraktorn inte feltolkas som tusentals kodändringar.

**Recovery-kontraktet ändras (D6):** med revisionshistorik i samma fil
gäller inte "fully derivable" längre. Nulägesgrafen och projektionen byggs
om från källkod, men historiken är lokal och förlorbar vid DB-borttapp.
Det ska stå i artifact-headern, och doctor ska redovisa det — aldrig tyst.

GTW-editens transaktions-ID länkas till revisionen när det finns. En extern
editor eller Git-checkout får `source=external_refresh`/`git_hook` och
filhashar som proveniens. Den ignorerade databasen är per klon och ska kunna
återskapas från källkod/Git; lokal edit-historik som bara finns i
transaktionsloggen får redovisas som lokal och förlorbar. Specificera
retention (t.ex. behåll deltan till senaste N commits plus lokala edits)
och ett explicit exportformat om historiken ska delas mellan kloner.

## Täckning före hård grind

Mät **separata dimensioner** per språk, relationstyp och konstrukt:

| Mått | Fråga | Facit |
|---|---|---|
| Extraktions-recall | Hittas verkliga anrop/beroenden alls? | Handmärkt AST-fixtur och språkens egna verktyg |
| Upplösningsprecision | Pekar `to_id` på rätt mål? | Handmärkt facit; fel mål är värre än `NULL` |
| Källtäckning | Kan `from_id` kopplas till en entitet eller en explicit syntetisk/okänd källa? | Handmärkt facit och klassificerade omatchade källor |
| Upplösningsgrad | Hur många extraherade kan lösas? | `resolved / extracted`, uppdelat per typ |
| Delta-precision | Visas rätt tillagda/borttagna länkar efter edit? | Före-/efter-fixtur |
| Multiplicitet | Syns 2→1 anrop till samma mål utan att aggregatet försvinner? | Callsite-fixtur med identisk källa och mål |
| Påverkans-recall | Finns alla kända konsumenter i rapporten? | Mutationer som ska ge utslag |
| Falsklarm | Hur ofta rapporteras säker brytning utan bevis? | Negativa fixturer och riktiga repos |

Prioritera Rust-vägarna som nu har kända hål: metodanrop, `self`/trait-
dispatch, makron, `use`-alias, modulvägar, samma namn i olika moduler,
generics och `cfg(test)` kontra produktionskod. Visa `unresolved` när
dynamisk dispatch eller externa crates gör exakt resolution omöjlig. LSP
eller rust-analyzer kan vara en **valfri precisionstjänst** ovanpå AST;
basläget ska fungera offline och utan LSP.

Täckning gäller också *effekter*, men de ska vara egna verifierbara
relationsklasser, inte gissade `calls`: exempelvis `reads_field`,
`writes_field`, `reads_config`, `writes_db_table` och `emits_event`.
SQL via strängar/query builders och värdeflöde mellan async tasks kräver
särskilda extraktorer eller deklarerade domänkontrakt. Börja med ett litet
Rust/SQLite-fixturset och redovisa `unknown` för dynamisk SQL. Påstå aldrig
att ett `Ok`-resultat bevisar att state ändrades; det kräver beteendetest.
Fas 1–3 får bara rapportera effektpåverkan för relationsklasser som faktiskt
har en testad extraktor eller ett deklarerat domänkontrakt. Berörda tester
kräver på samma sätt en separat koppling (t.ex. exakt kontraktsmanifest
eller märkt test-fixtur); en namnlikhet får högst vara kandidat. Tills
dessa leverabler finns ska rapporten säga `effect_unknown` respektive
`tests_unknown`, inte hitta på ett testförslag som låter säkert (D10).

## Faser och acceptans

### 1. Mät och reparera nulägesgrafen

- Lägg till märkt fixtursvit och `code_index coverage --json` med måtten
  ovan. Felaktig `to_id` ska vara en egen allvarlig kategori.
  `coverage` byggs som **permanent acceptans-harness** — återanvänds vid
  varje extraktor-bump och som godkännandeharness för fas 2–3, inte
  engångskod.
- Klassificera omatchade `from_id` per konstrukt och språk. Testa att
  effekten av en ändring i en sådan källa blir `unknown_source` med
  filankare, aldrig en tyst utebliven träff. Lägg till 2→1-callsite-fixtur
  för deltaaggregatet (D7–D8).
- Utöka AST-extraktionen och Rust-upplösningen konstrukt för konstrukt.
  Prioritering (billiga vinster först): **metodanrop** — `field_expression`
  inuti `call_expression`, namnmatchning mot impl-entities på
  `candidate`-nivå — är största enskilda lyften: metodanrop finns idag inte
  ens i nämnaren på 24,4 %-baslinjen. Sedan `macro_invocation` som
  unresolved-med-orsak samt import-alias/modulvägar. Djup typsemantik
  (trait-dispatch-inferens, generics) tillhör den valfria LSP-tjänsten —
  inget mini-rust-analyzer i basen.
  Baslinjen i Motor2 ska redovisas före/efter; sätt inget godtyckligt
  procenttal som enda godkännandekriterium.
- Testa ett projekt med samma funktionsnamn i flera moduler: en relation
  får inte kopplas till fel mål bara för att namnet är unikt i samma fil.

### 2. Revisionsdeltan och historik

- **Förutsättning:** D1, D2, D5 och D7–D9 låses innan revisions-tabellerna
  skrivs. D3, D4 och D10 styr impact-/grindfasen. Fas 1 kan börja
  oberoende — coverage och extraktionsreparation påverkas inte.
- Inför migration och atomisk **publicering** av före-/efter-refresh; query för
  `changes --from <revision> --to <revision>` och `history <symbol>`.
- Prova GTW-edit, extern edit, borttagen fil, rename, trasig parse,
  avbruten refresh, busy-fallback, extraktorbyte, Git-branchbyte och
  DB-återuppbyggnad. Testa samtidigt att en läsare under hela refreshen
  aldrig ser en blandning av revisionsrader (D5, D9).
  **+ GTW-undo:** en undo är en förändring som alla andra — revision med
  `source=gtw_undo` och transaktionslänk.
- **Delta-sammansättningsinvariant pinnas som test:**
  `delta(A→B) + delta(B→C)` ≡ `delta(A→C)` modulo delete/add-normalisering.
- Fixturer hermetiska (läxa #6): tempdir + `.git`-markör, aldrig
  repo-root-state — transaktionsreplay har tidigare korrumperat ett eget
  repo.
- Schema v4 = migrations-callout i CHANGELOG i samma ändring (additivt, ej
  breaking, men path-dep:ar som Motor2 litar på att kontraktsändringar
  ropas ut — §36-läxan).
- Ingen ändring får rapporteras som "inga relationer påverkade" när indexet
  är stale, historiken saknas eller målet är olöst.

### 3. Påverkansrapport som agent- och CI-yta

- `impact --since <revision|transaction>` i CLI och motsvarande MCP-fråga
  returnerar samma JSON-kontrakt; textvyn anger säkerhet och nästa kontroll.
  **Pariteten i samma ändring:** verktygstabellen i `GTW_INSTRUCTIONS.md`
  uppdateras samtidigt (`integration_mcp_instructions_listed`-mönstret) —
  instruktionstabellen får inte ligga efter verktyget.
- Implementera en riktad DB-fråga för inkommande relationer innan
  `idx_relations_to_name`/`idx_relations_to_id` tillskrivs en effekt.
  Benchmarka den mot dagens `load_all_graphs` med kall och varm cache;
  låt resultatet avgöra indexval och prestandakrav (D3).
- GTW `preview_edit` och `edit_node` kan använda samma kärna: preview
  räknar diff utan att skriva, editen länkar revision till transaktions-ID.
  **Edit-receiptet utökas** med `revision_id` + impact-sammanfattning
  (B1-kvittot med `new_node_path`/`node_content_hash` finns redan).
  **Preview-scope, explicit:** preview extraherar kandidatinnehållet i
  minnet (`extract_relations_from_tree` tar ett träd) och frågar inkommande
  påverkan mot nuvarande store; resolution mot hypotetisk helhet är UR
  preview-scope.
- Nivåer: **information** för möjliga konsumenter; **varning** för olösta
  mål, misstänkt rename eller saknad verifiering; **hård grind** bara när
  ett maskinläsbart, på förhand registrerat kontrakt bryts eller när
  kompilator/typkontroll/test ger ett reproducerbart fel. En borttagen
  relation kan vara en avsiktlig refaktorering och är inte i sig ett fel.
  **Rename-regeln (D4):** remove + add i samma caller-entity är endast en
  kandidat för granskning, aldrig bevis för säker rename eller automatisk
  dispens från ett kontrakt. En grön graf är inte en grön integration.
- Mutationstest: ta bort en anropad funktion, byt metodsignatur, ändra
  2→1 anrop till samma mål, lägg till ny anropsväg och lämna ett mål
  ambiguöst. Flytt av `state-write` testas när `writes_field` eller ett
  deklarerat kontrakt faktiskt finns; fram till dess krävs `effect_unknown`.
  Rapporten ska namnge berörd söm och krävd kontroll.

**Slutfacit:** en agent får före edit en påverkanlista och efter edit en
före-/efterrapport med källankare, osäkerhet och verifieringsstatus. En
oberoende testsvit fångar kända regressionsmutationer utan att hävda att
statisk analys ensam bevisar runtime-beteende.

### 4. Införande som standard i agenternas arbetsgång

Den här fasen börjar **efter** att faserna 1–3 har fungerande CLI/MCP-yta,
testad täckning och ärlig osäkerhetsrapportering. Skriv inte in ett krav
på ett verktygsanrop som ännu inte finns i AGENTS-regler eller skills.

1. **Före kodning:** uppdatera eller kontrollera indexets färskhet, fråga
   efter berörda entiteter, anropare, sidoeffekter och tidigare ändringar.
   Agenten använder rapporten för att välja filer och tester samt skriva en
   konkret ändringsplan. `unknown` ska bli en granskningspunkt, inte
   försvinna ur planen.
2. **Vid felsökning:** börja från ett observerat fel och följ inkommande
   och utgående relationer, tidigare deltan och berörda kontrakt. Jämför
   reparerad söm med nya anropsvägar. Rapporten ska hjälpa agenten att
   hitta fler nödvändiga ändringar, inte bara bekräfta första hypotesen.
3. **Efter edit:** kör samma påverkanfråga mot före-/efter-revisionerna,
   granska tillagda/borttagna/olösta länkar och kör föreslagna
   kompilator-, test- och driftfacit. Agenten redovisar vad som blev
   verifierat och vad som fortfarande är okänt innan den säger "klart".
4. **Drift i flera projekt:** börja med GTW:s eget repo och Motor2 som
   dogfood. När falsklarm och missar är mätta, dokumentera det korta
   receptet i respektive projekts AGENTS.md och i en GTW-skill. Andra
   projekt kan sedan aktivera samma rutin utan projektspecifika
   shellskript. CLI och MCP ska ge samma beslut för samma indexrevision.
5. **Automatisk kontroll:** pre-commit/CI ska själva räkna om färskhet,
   revisionsdelta och påverkan från den ändrade koden och jämföra mot
   rapportens filhash/revision. En inlämnad rapport är aldrig ensam ett
   facit; saknad DB, busy-fallback eller historikgap ger `unknown`, inte
   grönt. Hård blockering
   begränsas till verifierade deterministiska fel; olösta relationer
   blir synliga varningar med ansvarig uppföljning. En agent ska inte
   kunna få grönt bara genom att inte anropa verktyget.

**Införandefacit:** i en kontrollerad Motor2-mutation som återinför en
gammal integrationsmiss får agenten en namngiven påverkad relation före
edit, en delta-/riskrapport efter edit och ett relevant testförslag.
Upprepa samma prov med en relation som inte kan lösas: rapporten måste
säga `unknown`, aldrig "ingen påverkan". Följ användningen över tid:
andel kodändringar med före- och efterkontroll, träffade regressionsfel,
falsklarm och missade följdändringar. Först när dessa mått och tester
stödjer det blir relationskontroll en bindande standardrutin.

## Förhållande till befintliga planer

Detta utökar `CODE_INDEX_SQLITE_DESIGN.md` fas 3–4; det skapar ingen andra
graf eller separat databas. `CODE_INDEX_SCAN_TOOLS_PROPOSAL.md` innehåller
redan `drift_report` och andra scans. De ska läsa samma revisionsdeltan
och kan komma efter att täckning och historik är mätbara. GTW:s
transaktionshistorik fortsätter bära edit/undo; code-index bär grafens
förändring och pekar tillbaka till transaktionen.

## Granskningsbeslut 2026-11-09

Samma dags-granskning (agent) fann D1–D6 och T1–T5. En andra granskning
skärpte D1, D3–D5 och lade till D7–D10; allt är infogat ovan. Tabellen är
beslutsspåret, i
`CODE_INDEX_SQLITE_DESIGN.md`:s "13 findings"-mönster:

| # | Fynd | Beslut | Landade i |
|---|---|---|---|
| D1 | Path-baserad identitet → delete/add-brus vid varje edit | Deltaenhet = aggregerat `(from_entity, relation_type, target)` med count och callsite-ankare; `node_path` bara annotering | Lagring |
| D2 | generation_key bevakar inte nyckelalgoritmen | `identity_scheme_version` i generation_key/meta; byte = ny baseline | Lagring |
| D3 | `relations` saknar to-index, men dagens impact-väg laddar hela grafen | Riktad DB-query + benchmark först; lägg bara index som frågan använder | Nuläge, fas 3 |
| D4 | Rename-heuristik kan dölja ett verkligt bortfall | remove + add = kandidat; hård grind kräver deklarerat kontrakt eller reproducerat fel | Fas 3 |
| D5 | Atomisk revision vs P5 busy-degradering | En tx eller staging + atomiskt aktiv-revisionsbyte; benchmarka båda, publicera aldrig per-fil direkt | Lagring |
| D6 | "Fully derivable" gäller inte längre | Recovery-kontrakt uppdateras: historik lokal + förlorbar; artifact-header + doctor | Lagring |
| D7 | Många `calls.from_id` saknar entitetsmatch | Mät källtäckning; använd explicit syntetisk/okänd källa och behåll påverkan | Nuläge, fas 1 |
| D8 | Aggregerad relation kan dölja 2→1 anrop | Count före/efter + callsite-ankare; delta på count även när relationen lever | Lagring, fas 1 |
| D9 | P5 direkt fallback kan missa en historikrevision | Redovisa `history_gap`/`index_stale`; aldrig grön grind på fallback | Lagring, fas 2 |
| D10 | Effekt- och testpåverkan utlovas före extraktorer/kopplingar | Separata leverabler; `effect_unknown`/`tests_unknown` tills de är testade | Täckning, fas 3 |
| T1 | undo saknades i testmatrisen | revision med `source=gtw_undo` + tx-länk | Fas 2 |
| T2 | Delta-komposition otestad | sammansättningsinvariant som pinnat test | Fas 2 |
| T3 | Fixturer måste vara hermetiska | tempdir + `.git`-markör (läxa #6) | Fas 2 |
| T4 | Schema-migration utan callout-risk | CHANGELOG-callout i samma ändring | Fas 2 |
| T5 | MCP-paritet riskerar eftersläpning | verktygstabellen i `GTW_INSTRUCTIONS.md` i SAMMA ändring | Fas 3 |

Största möjligheten: **metodanrops-extraktion på `candidate`-nivå** (fas 1)
— metodanrop saknas i nämnaren idag, så nuvarande 24,4 %-baslinje säger
inget om deras täckning. `coverage` byggs som permanent harness; edit-receiptet får
`revision_id`; preview-scope anges explicit (resolution mot hypotetisk
helhet är UR scopet).

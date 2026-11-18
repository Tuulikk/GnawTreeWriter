# Bindnings- och operationsobservationer

Status 2026-11-17: första fil-lokala Rust/JavaScript/TypeScript-analysen är implementerad som ett publikt Rust-API i `src/core/binding_observations.rs`. Ingen ny CLI/MCP-metod eller persistent lagring ingår.

## Implementerat

`analyze_bindings_from_tree(tree, file, language)` återanvänder GTW:s `TreeNode` och dess sökvägar/teckenkolumner. En `Binding` har ett versionslokalt ID (`file#node_path`), namn, deklarationsankare och bindningsslag. En `OperationObservation` beskriver läsning, omtilldelning, sammansatt tilldelning, uppdatering, medlemsskrivning, muterbart lån eller indirekt skrivning. Den innehåller operationsankare, valfri koppling till deklarationen, möjlig effekt och separat krav på bindningens muterbarhet. `verified_effect` är alltid falsk: syntaxen bevisar inte att operationen kördes.

Scope-stacken kopplar lokala användningar till närmast synliga deklaration och hanterar block-shadowing. Initieringsuttrycket besöks före den nya deklarationen, så `let x = &mut x` kopplar lånet till yttre `x`. Rust `mut` på bindningen skiljs från `&mut T`; en skrivning via `*x` observeras som indirekt och får okänt krav på själva bindningens `mut`. JavaScript/TypeScript `obj.field = ...` och `obj.field++` kräver inte `let` på `obj`. Enkel destrukturering stöds. Olösta namn förblir olösta.

`BindingFinding` härleder `NeedsMutableBinding` eller `CouldBeImmutable` ur filens observationer. Det senare är en indikation med begränsad täckning, inte en autofix eller ett verifierat kompilatorutslag. Ingen procentsats för konfidens anges utan kalibreringsdata. `CallObservation`, `RelationIndex` och deras nuvarande JSON påverkas inte.

## Förberett, men inte implementerat

Samma parsade träd som `code_index_db::refresh` använder kan matas till analysen utan extra parsning. `SourceAnchor` kommer från `index_relations` och ger källfil, nodsökväg och exakt spann. Konsumenten bör knyta varje svar till filens innehållshash i tabellen `files`; en nodsökväg ensam är inte stabil över redigeringar. Bindnings-ID:n är lokala och ska inte förväxlas med `Entity.id` för funktioner/typer. Den omgivande funktionen kan senare kopplas via `Entity.path` och `Relation.from`, men inget sådant mål gissas i första versionen.

En framtida lagringsversion kan lägga observationer i samma SQLite-databas som `entities` och `relations`, med `(file, file_hash, node_path, kind)` som proveniens och valfritt bindnings-ID. `refresh` ska då ersätta observationer för ändrade filer och ta bort dem för raderade filer i samma transaktion; ändrad extraktorsemantik måste höja `EXTRACTOR_VERSION`. Lagra inte härledda kandidater när de kan räknas från färska observationer. Mät antal rader och uppdateringskostnad före schemabeslut.

## Föreslaget analyslager

Ett senare lager kan skapa en hypotes, exempelvis ”bindningen behöver vara muterbar”, och låta observationer peka till den med riktning stödjande, motsägande eller neutral. Operation, möjlig effekt, krav och verifierad effekt ska förbli olika begrepp. Flera observationer från samma AST-operation delar en proveniensnyckel och får inte räknas som oberoende evidens. Kvalitativa nivåer bör grundas i täckning (parser, makroexpansion, typer, alias, kontrollflöde) och kalibreras mot kompilator/typsystem innan numeriska konfidensvärden införs.

`index_revisions`, `file_versions`, `relation_changes` och Code Index onion-vyn ger befintliga revisionsankare. Ett senare observationsdelta bör beräknas mot innehållshash och identitet, så att ändrade ankare inte misstas för ändrat beteende. En möjlig effekts väg kan därefter följas via relationer, med explicit stopp vid olöst relation eller saknad kontrollflödesinformation. Det kräver typ-/alias- och flödesanalys och byggs först när en verklig konsument behöver det.

Motor2 och andra agenter kan i dag konsumera Rust-API:t i process eller via egen wrapper: skicka ett parsat träd och läs bindningar, observationer, fynd och begränsningar. En ny MCP/CLI-yta bör dela exakt denna funktion och serialisera proveniens samt täckning. Nästa praktiska steg är språkfixturer och kompilatorfacit för fler mönster, därefter en gemensam CLI/MCP-läsyta; persistent indexering är senare arbete.

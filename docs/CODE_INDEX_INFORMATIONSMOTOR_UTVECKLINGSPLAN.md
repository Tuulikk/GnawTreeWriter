# Code Index och informationsmotorvisionen – reviderad utvecklingsplan

**Status:** Beslutsunderlag, inte godkänd arbetsorder. Ingen implementation påbörjad.
**Datum:** 2026-11-14
**Visionsunderlag:** [code-index-visions.md](../code-index-visions.md)
**Avgränsning:** GTW som kodverktyg först; återanvändbar Code Index-kärna och en generell informationsmotor är möjliga, separata fortsättningar.

## Rekommendation i korthet

Flytta fram två arbeten: deterministisk klassificering av det GTW redan observerar, och korrekt omprövning av upplösta relationer vid inkrementell indexering. Båda ger direkt nytta för Code Index, beroendekartor och undersökningar. Utveckla parallellt ett tunt gemensamt kontrakt för **resultat och undersökningsomfattning**. Crate-extraktion är ett separat val, inte ett tekniskt nästa steg som dessa förbättringar måste vänta på.

Planen bygger på kodgranskning och GTW:s läsande `get_skeleton`, `search_nodes` och `code_index`. **Observation** avser kod, tester eller verktygssvar; **slutsats** är en tolkning av dem; **rekommendation** är ett föreslaget arbete. Arbetsinsatser är uppskattningar, inte uppmätta tider. Inga tester eller bygggrindar kördes för denna plan eftersom ingen kod ändrades.

## 1. Bekräftad nulägesbild – tillägg till första inventeringen

Den tidigare inventeringen står i huvudsak kvar: GTW har ett inkrementellt SQLite-index, AST-extraherade entiteter och relationer, grafoperationer, relationshistorik, färskhets- och täckningsmått samt flera separata sök- och agentytor. Följande preciseringar ändrar prioriteringen:

1. **Klassificering kan börja nu.** Extraktorn ser redan AST-former för identifieraranrop, kvalificerade anrop, fält-/metodanrop, makroanrop, importer och typreferenser. Flera former reduceras därefter till en enkel relationsrad. Syntaxformen kan bevaras och visas utan AI, språkserver eller ny parser. [Extraktion](../src/core/index_relations.rs), [relationsschema](../src/core/code_index_db.rs).
2. **Inkrementell upplösning har ett omprövningsgap.** `resolve_unresolved_in_db` läser `WHERE to_id IS NULL`. En tidigare upplöst relation omprövas därför inte generellt när en konkurrerande symbol tillkommer i en annan fil. När måldefinitionens egen fil ändras nollställs däremot referenser till dess entiteter. De två fallen behöver skilda regressionstester. [Resolver och refresh](../src/core/code_index_db.rs).
3. **Historiken fångar inte varje kunskapsändring.** Relationsdeltan grupperas efter `from_id`, relationstyp och `to_name`. En ren ändring av `to_id` eller upplösningsstatus hoppas över när antal observerade kanter är oförändrat. `history` och `onion` beskriver främst kantantal, inte alla omprövade slutsatser. [Deltafunktionerna](../src/core/code_index_db.rs), [historikläsningen](../src/core/code_graph.rs).
4. **En förmågekarta finns delvis.** `capabilities` anger modellbehov, `commands` listar CLI-kommandon, MCP har `tools/list`, och `guide` ger exempel. De är ännu inte en sammanhängande, verifierbar karta. `commands` är manuellt underhållen; kodkommentaren dokumenterar tidigare drift från 28 till 54 kommandon. [capabilities](../src/core/capabilities.rs), [CLI-manifest](../src/cli.rs), [MCP-guide](../src/mcp/mod.rs).
5. **Relationsmåtten behöver fortsatt exakta namn.** `coverage` skiljer upplösningsgrad från recall och redovisar lokala kandidater. Dess klass `external` betyder dock i praktiken att ingen entitet med samma namn hittats i indexet; det bevisar inte att målet är externt. [Coverage](../src/core/code_index_coverage.rs).

Ett läsande `code_index mode=coverage` rapporterade vid granskningen 192 filer och 25 956 extraherade relationer. För `calls` var 2 955 av 23 443 extraherade relationer upplösta; med lokala kandidater som nämnare var det 2 955 av 6 931. Inget av talen är recall mot alla verkliga anrop. Rapporten angav `index_stale=false` och `history_gap=false` för den befintliga storen; det styrker inte att varje möjlig relation har extraherats.

Äldre planstatus måste kontrolleras mot kod och senare dokumentation. [ROADMAP.md](../ROADMAP.md) varnar själv för äldre text. [Relations- och historikförslaget](CODE_INDEX_RELATION_HISTORY_PROPOSAL.md) är delvis genomfört, även om dess inledning säger ”inte implementerat”. [Onion-modellen](ONION_SKIN_MODEL.md) anger att `history`/`onion` finns, medan [vymodellen](VIEWS_MODEL.md) uttryckligen är obygd. Äldre rader i [uppslag.md](../uppslag.md) om en tom `relation_changes`-tabell motsägs av nuvarande skrivkod och [handovern](HANDOVER-LATEST.md).

## 2. Tidig klassificering av kod och relationer

Håll minst fyra dimensioner åtskilda:

- **Vad:** objekt- eller relationstyp och observerad syntaxform.
- **Varifrån:** fil, position, hash, parser/extraktor och eventuellt filens roll.
- **Måluppslag:** valt mål, flera kandidater, ingen kandidat eller ännu ej undersökt.
- **Styrka i slutsatsen:** direkt observerat, valt av deterministisk regel, styrkt med starkare kontroll eller förkastat.

Ett befintligt `to_id` betyder att GTW:s regel valde ett mål. Det betyder inte att en kompilator verifierade den faktiska anropsmottagaren.

| Klassificering | Underlag som finns | Litet första steg | Gräns |
|---|---|---|---|
| Identifierar-, kvalificerat, fält-/metod- och makroanrop | AST-typen vid extraktion | Bevara syntaxform och källankare i resultatet. | `obj.metod()` ger inte mottagarens statiska typ eller faktisk dispatch. Kvalificerad väg förloras delvis när `get_name()` ger sista namnet. |
| Typreferens, `implements`, import | Befintliga `uses`, `implements`, `imports` | Beskriv extraktionsregel och täckningsområde per typ. | `uses` täcker ett urval av signaturer/deklarationer, inte alla typberoenden. |
| Lokal eller extern import | Importtext och projektets entiteter | Pröva `local_candidate`, `known_external_prefix`, `undetermined`. | Ingen lokal namnträff bevisar inte extern import; dagens importextraktion använder delvis radtext. |
| Produktions-, test- och genererad kod | Filsökväg, AST-attribut, filväljare | Klassificera explicita `tests/`-filer, kända artefakter och `unknown`. | Sökväg är heuristik; inline `#[cfg(test)]` kräver AST-kontext. `is_generated_artifact` känner uttryckligen igen `code-index.md`. |
| Upplöst, möjlig, olöst | `to_id`, entiteter och coverage-klasser | Exponera använd regel och anledning per relation. | Inga nya falskt ”verifierade” mål får skapas av en etikett. |

**Rekommendation:** Börja med få ortogonala fält och ett uttryckligt okänt tillstånd. Undvik en generell `confidence`-poäng. Definiera också orden ”direkt” och ”indirekt” innan de används som klasser: syntaktiskt direkt anrop och verifierat statiskt mål är olika utsagor.

## 3. Litet kontrakt för kunskap och undersökning

Kontraktet kan först vara ett **läsformat ovanpå befintliga API:er**. Ingen generell kunskapsdatabas behövs för första försöket.

| Del | Minsta användbara fält | Syfte |
|---|---|---|
| Utsaga om informationen | Objekt-/relations-ID; observerad syntax; källfil, position och filhash; relationstyp; valt mål eller kandidater; upplösningsregel; status `observed`, `resolved_by_rule`, `candidate`, `unresolved` eller `rejected`; underlagsreferenser. | Ett anrop kan vara säkert observerat trots okänt mål. Regelupplöst mål beskrivs som just regelupplöst. |
| Utsaga om undersökningen | Fråga och scope; revision/hash; använda metoder och versioner; faktiskt undersökt population; gränser som `top_n`, filantal eller tid; `complete`, `bounded`, `aborted` eller `stale`; stopporsak och möjliga nästa kontroller. | Ett olöst mål kan vara väl undersökt. Ett tomt svar kan vara komplett, begränsat eller avbrutet. |
| Återanvändning | Nyckel över fråge-/regelversion, källhashar, scope och parametrar; resultatets giltighet. | Upprepa inte samma kontroll på samma underlag; ompröva när underlag eller metod ändras. |

GTW har redan delar av underlaget: filhashar och generationsnyckel i [indexdatabasen](../src/core/code_index_db.rs), `index_stale` och `history_gap` i [coverage](../src/core/code_index_coverage.rs) samt `graph_coverage` i [grafresultat](../src/core/code_graph.rs). En synlig sökgräns behövs även för reservvägar: [lexikal reservsökning](../src/core/gnaw_find.rs) tar högst 400 filer och kan stoppa när träffbudgeten nåtts.

**Omprövning:** Behåll observationen när dess tolkning förkastas. Om en ny kandidat gör ett tidigare mål tvetydigt är `resolved_by_rule → unresolved/ambiguous` ett legitimt förbättrat resultat. Först måste indexpassagen återbesöka berörda mål. Därefter kan man besluta om slutsatsändringar ska ingå i `relation_changes` eller få en separat, tydligt benämnd historik. Ändra inte dagens kantantalshistorik tyst.

## 4. Mät relationskvalitet som övergångar

Utöka det handmärkta [relationsfacit](../tests/code_index_fixtures.rs) med **före/efter-sekvenser**: ny konkurrerande definition i annan fil, borttaget mål, ändrad import och löst tvetydighet.

Mät tre led separat:

1. **Extraktion:** Finns den observerade förekomsten? Precision och recall mot handmärkt facit.
2. **Klassificering:** Rätt syntaxform, relationstyp och filkontext? Redovisa okända och ej tillämpliga fall.
3. **Måluppslag:** Rätt mål, korrekt avstående eller felaktigt mål? Redovisa minst följande övergångar:

| Tidigare → nytt utfall | Tolkning |
|---|---|
| Olöst → korrekt upplöst | Vinst |
| Olöst → felaktigt upplöst | Nytt falskt påstående |
| Korrekt → fortsatt korrekt | Bevarad kvalitet |
| Felaktigt mål → korrekt mål | Korrigering |
| Felaktigt mål → korrekt förkastat/olöst | Korrigering med kvarvarande osäkerhet |
| Korrekt → felaktigt ändrat eller förkastat | Regression |
| Olöst → fortsatt olöst med korrekt anledning | Ärligt resultat; avgör om mer kan undersökas |

Räkna även **felaktigt säkra mål per 100 handmärkta relationer**, inte bara upplösningsprocent. Dagens skydd mot att koppla vanliga metodnamn till en godtycklig metod visar varför lägre upplösningsgrad kan ge bättre graf. [Upplösningsregeln](../src/core/index_relations.rs).

**Första hypotes att pröva:** En ändrad entitetspopulation ska omvärdera berörda mål även om anropsfilens hash är oförändrad. Börja med ett hermetiskt test som tillför en konkurrerande fri funktion. Jämför riktad omupplösning per påverkat namn med full omupplösning vid varje refresh. Använd full omupplösning som korrekthetsreferens och mät båda innan optimering väljs.

## 5. Agentanpassad undersökning och verktygskomposition

Skilj **motorns kvalitet** (relationer, omprövning, frågeresultat) från **exponeringen** (CLI, MCP och dokumentation). GTW har redan `get_skeleton`/`search_nodes`, Code Index-frågor, grafoperationer, [curate](../src/core/curator.rs) med tokenbudget och [guide](../src/mcp/mod.rs) för verktygsval. De kan bli mer användbara med ett tunt lager som:

- ger samma ankare, status och scope åt index-, graf- och strukturella resultat;
- länkar ett fynd till en konkret fortsatt kontroll, exempelvis `read_node`, `coverage`, `touches` eller sökning efter konkurrerande definitioner;
- ger sammanfattning, totalantal, sökgräns och utvalda träffar innan en full radmängd hämtas;
- återanvänder delresultat bara när hash, scope och metodversion matchar;
- gör samma maskinläsbara mellanresultat granskningsbart för människa och liten lokal modell.

**Konkret exponeringslucka:** MCP:s `code_index kind=relations` läser alla matchande rader och returnerar en lista; något `limit` eller sidformat syns inte i den granskade vägen. Ett tidigare `kind=items`-anrop gav också ett stort projektsvar, medan `get_skeleton` för en fil var kompakt. Begränsade eller paginerade läsresultat ger därför omedelbar nytta före nya specialverktyg. [MCP-frågevägen](../src/mcp/mod.rs), [databasläsningen](../src/core/code_index_db.rs).

`guide` och verktygsbeskrivningarna bör säga **vad verktygen kan fastställa**: exempelvis ”upplösta kanter enligt GTW:s regel”, ”endast inom vald sökgräns” och ”saknat lokalt mål är inte bevisad extern import”. GTW behöver inget eget agentplaneringssystem för detta.

## 6. Crate-gränser – tre alternativ utan låst ordning

En gemensam frågeyta före extraktion är en **rekommenderad arbetsordning**, inte ett hårt tekniskt beroende. Beteendebevarande extraktion kan provas tidigare, men flyttkostnaden måste motiveras.

| Alternativ | Faktisk gräns och omedelbar nytta | Kostnad och risk | Bedömning |
|---|---|---|---|
| **A. Flytta befintligt Code Index nästan helt** | En crate med dagens indexering, rendering, lagring och frågor. [code_index.rs](../src/core/code_index.rs) använder `get_parser`/`TreeNode`; [code_index_db.rs](../src/core/code_index_db.rs) anropar parser, filväljare och GTW:s extraktorer. | Hög beroende- och regressionsrisk; parser följer med eller beroendeinversion krävs. Ett externt API kan cementera dagens förenklingar. | Pröva bara om en konsument behöver hela GTW-indexeraren som bibliotek. Direkt användning av GTW:s befintliga bibliotek är en enklare första jämförelse. |
| **B. Smal lagrings-, relations- och frågekärna** | GTW producerar kodspecifika objekt från `TreeNode`; en crate äger typade lagrings-/frågekontrakt och möjligen revisionsläsning. Andra Rust-program kan läsa utan egen AST-extraktion. | Medel–hög initial kostnad: refresh och parseranrop är blandade i databasmodulen. Schema, migrationer och kompatibilitet blir delat underhåll. | Mest lovande om en andra konsument faktiskt behöver samma läs-/frågesemantik. Börja med en smal spike. |
| **C. Behåll internt, förbättra gränserna** | Separera extraktion, upplösning och läsfrågor inom GTW; ge typade publika resultat där det behövs. `load_entities`/`load_relations` finns redan, men returnerar `serde_json::Value`. | Lägst omedelbar risk; kräver fortfarande API-disciplin och tester. | Rekommenderat standardval nu. Förbättrar GTW och förbereder en eventuell B-extraktion. |

Ingen väg kräver en ny parser eller att en generell motor duplicerar GTW:s AST-analys. En framtida extra parser eller compiler-/LSP-källa kan komplettera GTW efter uppmätta kvalitetsgränser, men får inte vara ett krav för grundfunktionen.

## 7. Levande och verifierbar förmågekarta

Bygg vidare på `capabilities`, `commands`, MCP `tools/list`, `guide` och befintliga testkontrakt. Undvik en ny stor handskriven katalog.

En minimal förmågepost kan ha: **ID · syfte · CLI/MCP-ingång · nödvändig nivå/store/modell · resultatets omfattning · vad funktionen kan fastställa · begränsningar · källmodul · test/fixtur · senast verifierad version · föreslaget arbete**.

Härled maskinellt det som redan finns i CLI-/MCP-registreringar; underhåll manuellt bara semantiska påståenden och begränsningar. Märk varje post **verifierad**, **dokumenterad men oprövad** eller **föreslagen** och länka verifierade påståenden till namngivna tester eller mätningar. Ett kontraktstest bör upptäcka saknad post för registrerad yta och post som pekar på borttaget test. Den dokumenterade `commands`-driften motiverar en sådan kontroll. Katalogen kan ge `guide` bättre svar och stödja framtida verktygsjämförelser utan obestyrkta konkurrenspåståenden.

## 8. Utvecklingsspår, beroenden och flera samtidiga vinster

| Spår | Direkt nytta | Hårt beroende | Kan göras parallellt |
|---|---|---|---|
| Relationskvalitet, omprövning och tidig klassificering | Bättre Code Index, filgraf, påverkan och agentundersökning. | Facit för ett styrkt kvalitetspåstående; relevant omupplösning för korrigering av gamla mål. | Ja, med förmågekarta och begränsade läsresultat. |
| Kunskaps- och undersökningsstatus | Ärliga tomma, begränsade, avbrutna och olösta resultat. | Samma scope-/versionssemantik för komposition. | Ja, läskontrakt kan prototypas samtidigt med resolverarbete. |
| Tunn frågekomposition och agentexponering | Färre manuella hopfogningar och mindre tokenbrus. | Minsta gemensamma identitet, status och scope. | Kompakta enskilda svar kan göras tidigare. |
| Vyer och avgränsade scan-verktyg | Kontextbaserat brusfilter och kontroll av manifestdrift. | Är enklare efter ett läskontrakt; inget krav på crate eller AI. | Ett snävt scan-experiment kan börja tidigare. |
| Återanvändbar crate | Potentiell nytta för Motor2 och andra Rust-program. | Faktisk gräns och migrationskontrakt; inte ett beroende för spåren ovan. | En read-only gränsspike kan göras oberoende. |

**Hårda beroenden:** kvalitetsanspråk kräver facit; korrekta mängdoperationer kräver gemensam identitets-, versions- och scope-semantik; omprövning kräver att relevanta befintliga relationer återbesöks. **Rekommenderad ordning:** facit före resolverändringar, litet läskontrakt före brett fråge-API, verklig andra konsument före stabil crate.

Högst prioritet får **inkrementell omprövning och förklarad upplösning**: en förbättring kan samtidigt stärka indexet, grafen, påverkananalys, agenters arbete och framtida sökkomposition. Näst högst kommer **scope- och fullständighetskontraktet**, som förbättrar flera ytor på en gång. Generalisering utan uppmätt konsument prioriteras lägre.

## 9. Prioriterad färdplan och små experiment

Storlekar: **S** ≈ dagar, **M** ≈ en till några veckor, **L** ≈ flera etapper. Risk avser regression och felaktig semantik.

| När | Arbete, storlek och risk | Verifiering och resultat som talar emot |
|---|---|---|
| **Nära** | Utöka relationsfacit med före/efter-fall och alla övergångar ovan. **S/låg**, fristående. | Mät extraktion, klassificering och mål separat. Om fallen inte kan märkas entydigt måste frågan preciseras före resolverdesign. |
| **Nära** | Testa ny konkurrerande symbol mot inkrementell refresh; jämför med full ombyggnad och pröva riktad mot full omupplösning. **S–M/medel**. | Samma slutgraf i båda körsätten och felaktigt gammalt mål förkastat. Om riktad metod kostar nästan lika mycket eller missar fall: välj enklare full omupplösning tills mätning motiverar mer. |
| **Nära** | Bevara och exponera några AST-baserade klasser och upplösningsanledningar; börja gärna utan beständig schemaändring. **S–M/medel**. | Fixtur per klass och `unknown`; inga nya falskt verifierade mål. Om klassen inte ändrar ett användbart beslut: behåll som diagnostik. |
| **Nära** | Ge Code Index-läsningar totalantal, begränsning och kompakt första svar; utöka `guide` med indexfrågor och begränsningar. **S/medel**. | Full data ska fortsatt kunna hämtas; mät svarsstorlek/tokenkostnad och förlorade träffar. Om paginering försvårar konsistens: börja med sammanfattning och explicit filtrering. |
| **Mellan** | Prototypa gemensamt läsresultat med separat informations- och undersökningsstatus; komponera strukturell sökning, index och graf. **M/medel**. | Snitt/union mot facit; tom fullständig fråga skiljs från topp-N, stale och avbruten. Om gemensam objektidentitet inte håller: begränsa första försöket till filer eller kanter. |
| **Mellan** | Prova läsbaserade vyer och ett snävt `manifest_check` som tillämpningar av kontraktet. **M/medel**. | Okända behålls, vybyte kräver ingen omindexering, scan-fynd har källankare och kan motbevisas. Högt falsklarmstal talar emot bred lansering. |
| **Mellan/lång sikt** | Pröva alternativ B mot en verklig andra konsument eller fortsätt med C. **M–L/hög**. | GTW-resultat och migrationstester förblir oförändrade; få GTW-/parserberoenden i crate-API:t; mät konkret konsumentnytta. Växande beroendeyta talar för C. |
| **Lång sikt, valfritt** | Aktiv komplettering och fler informationsdomäner. **L/hög**. | Kräv ett konkret andra domänfall, uppmätt nytta/kostnad per kontroll och tydlig stoppregel. |

Fyra små experiment kan genomföras utan att först besluta hela arkitekturen:

1. **Relationsövergångar:** Hermetisk före/efter-fixtur med ny konkurrent. Mät korrekt mål, falskt mål, rätt förkastande och oförändrade korrekta mål. Ett högre antal falska säkra mål talar emot upplösningsregeln även om upplösningsgraden stiger.
2. **Komponerad läsfråga:** Kombinera `changed`, `touches` och strukturell sökning mot ett litet facit. Mät träffar, bortfall, körtid, utdata/tokens och hur ofta scope eller revision skiljer sig. Otydlig identitet eller ofullständighet talar emot ett brett API i detta skede.
3. **Dynamiskt filter:** Applicera två tillfälliga vyer över samma graf med ett oklassificerat objekt och en oklassificerad relation. Mät determinism, egen nämnare och att okända förblir synliga. Om vyn behöver ändra grundstore är gränsen fel vald.
4. **Crate-gräns:** Kartlägg beroenden mellan `code_index_db`, `code_graph`, `index_relations`, parser och CLI; pröva sedan enbart läsning/frågor mot teststore. Mät vilka GTW-typer och beroenden som följer med. Om parser eller CLI måste in i den smala kärnan är snittet för brett.

## 10. Beslutspunkter som kan vänta

| Fråga | Rimliga vägar | Vad avgör? |
|---|---|---|
| Hur långt ska GTW:s resolver gå? | Stegvis egen AST-/import-/modulanalys; senare valfri compiler-/LSP-källa. | Handmärkt precision/recall och nytta på verkliga ändringsbeslut. Grundgrafen ska fungera självständigt. |
| Var ska klassificeringar leva? | Tillfällig läsvy, projektregel eller beständig härledd uppgift. | Återanvändningsgrad, ogiltigförklaring och förmåga att bevara okända tillstånd. |
| Hur rik kunskapsstatus behövs? | Litet typat läsresultat först; rikare påståendemodell vid påvisat behov. | Om samma kontrakt klarar kodrelationer och ett scan-verktyg utan specialfall. |
| När ska Code Index extraheras? | A, B eller C ovan. | Beroendespikens resultat och en konkret andra konsuments behov. |
| Vilken frågeteknik behövs? | Komposition i Rust över nuvarande frågor; riktad SQLite/FTS; särskilda mängdstrukturer vid uppmätt skala. | Mät korrekthet, tid och minne på representativa projekt. FTS/RoaringBitmap är teknikval, inte mål. |
| När ska komplettering automatiseras? | Först föreslagna manuella kontroller, senare budgeterade automatiska steg. | Andel frågor som löses, kostnad per löst fråga och säker stoppregel. |

## Rekommenderad första arbetsomgång

Efter granskning av denna plan: **(1)** före/efter-facit, **(2)** inkrementellt konkurrensfall och omprövningsmätning, **(3)** litet läskontrakt för klass, underlag, scope och fullständighet, **(4)** kompakt Code Index-svar med bättre vägledning. Uppgifterna kan granskas var för sig. Ingen crate, generell motor, ny parser eller modellinsats krävs.

**Acceptans för omgången:** mått med ägare och nämnare; redovisade falska och okända fall; samma semantik via relevanta CLI-/MCP-ytor när nya gränssnitt väljs; uppmätt frågekostnad; samt ett uttalat beslut om vad som är värt att produktifiera.

## Ändrat jämfört med den första planen

Klassificering kommer tidigare. Kvalitetsmåttet omfattar även bevarade, förkastade och korrigerade slutsatser. Kunskap om informationen skiljs från kunskap om undersökningen. Agenternas exponeringsproblem har ett eget spår. Crate-alternativen bedöms utan låst ordning. Skälen är det verifierade omprövningsgapet och att GTW redan har mer AST-, förmåge- och verktygsunderlag än den första prioriteringen tog till vara.

**Begränsning i granskningen:** GTW:s läsande AST-verktyg hittade exakta funktioner, men Code Index förklarade inte ensamt hur gamla mål omprövas eller hur historiken behandlar rena upplösningsändringar. Det krävde direkt läsning av refresh-, resolver- och deltakoden. Testhänvisningarna visar befintlig testkod, inte en ny grön körning.

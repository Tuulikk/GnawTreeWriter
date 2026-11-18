# Gnaw Unified Interactive Environment (GUIE)

### En gemensam, navigerbar och utvecklingsbar miljö för människor, programvara och artificiellt intellekt

**Status:** Levande visionsdokument – första sammanställda version  
**Omfattning:** GUIE, GnawTreeWriter (GTW), Motor2 och möjliga kopplingar till övriga Gnaw-projekt  
**Syfte:** Bevara och utveckla en långsiktig vision utan att förväxla den med nuvarande implementation eller beslutad utvecklingsplan.

---

## 1. Grundvision

Gnaw Unified Interactive Environment (GUIE) är visionen om en gemensam informations- och arbetsmiljö där människor, deterministiska system, specialiserade verktyg och AI-modeller kan undersöka, förstå, navigera, skapa, förändra och verifiera information tillsammans.

Miljön ska inte begränsas till konversation med en AI-modell eller passiv visualisering av information.

Användaren ska kunna arbeta direkt med informationens struktur, innehåll, relationer och historik. Automatiserade system och språkmodeller ska kunna använda samma underliggande representationer och operationer.

Målet är en miljö där förmåga inte enbart kommer från språkmodeller, utan också från bestående informationsstrukturer, verktyg, regler, verifiering, återkoppling och återanvändbara arbetsflöden.

Den långsiktiga inriktningen kan beskrivas som ett utforskande av **Artificial Intellect**: utvecklingsbara system med samverkande förmågor, snarare än enbart språkmodeller som styr andra verktyg.

GUIE är ett övergripande koncept. GTW, Motor2 och andra Gnaw-projekt kan bidra med olika delar av dess funktionalitet.

Detta dokument beskriver en vision, inte ett påstående om att dagens system redan uppfyller den.

## 2. Intelligence-Aided Workflow (IAW)

**Intelligence-Aided Workflow (IAW)** är en vägledande princip för hur arbetsflöden inom GUIE ska kunna stödjas av olika former av funktionell intelligens.

Syftet är att förbättra människors förmåga att förstå, navigera, skapa, undersöka och förändra information.

Intelligensstödet kan komma från:

- Deterministiska algoritmer och funktioner.
- Strukturerad kunskap och informationsmodeller.
- Regler och analysmotorer.
- Språkmodeller och andra specialiserade system.
- Mänskliga bidrag, bedömningar och korrigeringar.

Ingen enskild teknik ska betraktas som en obligatorisk förutsättning för ett intelligent arbetsflöde.

Systemet ska använda de förmågor som är lämpliga för uppgiften, med hänsyn till korrekthet, resurskostnad, transparens och möjlighet till mänsklig kontroll.

Användaren ska kunna dra nytta av intelligensstödet utan att behöva interagera direkt med en AI-modell. Samtidigt ska relevanta beslut, slutsatser och åtgärder kunna granskas och påverkas.

IAW innebär därför inte att varje arbetsmoment behöver styras av en språkmodell. Ett arbetsflöde kan vara intelligensstött genom att flera mindre, specialiserade funktioner samverkar.

**GUIE beskriver miljön där människor och system arbetar. IAW beskriver principen för hur olika förmågor stödjer arbetet inom miljön.**

## 3. Gemensam informationsmodell

Information ska kunna representeras som identifierbara objekt med egenskaper, relationer, ursprung, tillstånd och historik.

Exempel på informationsobjekt är:

- Kodfiler, funktioner, klasser och uttryck.
- AST-element och strukturella relationer.
- Commits, kodändringar och versioner.
- Arbets- och agentsessioner.
- Tester, observationer och analysresultat.
- Noteringar, hypoteser, beslut och dokumentation.
- Arbetsflöden, funktioner och tillgängliga verktyg.

Samma objekt ska kunna förekomma i flera vyer och sammanhang utan att behöva dupliceras som oberoende informationskopior.

En visuell karta, en editor, en relationsgraf och ett agentgränssnitt kan därför fungera som olika ingångar till samma informationsmodell.

Informationsmodellen ska skilja mellan källdata, observationer, härledningar, användarens bedömningar och systemets arbetsstatus.

## 4. Informationslager och semantik

En kodfil ska inte enbart betraktas som text. Den ska kunna bära flera sammanhängande informationslager.

Samma princip ska kunna tillämpas på mindre enheter, exempelvis funktioner, kodblock och uttryck.

### 4.1 Källkod

Den faktiska koden, dess struktur i filer och dess redigerbara innehåll.

### 4.2 Syntax och AST

Parserns representation av kodens syntaktiska konstruktioner, inklusive definitioner, anrop, uttryck, typer, importer och källpositioner.

### 4.3 Semantisk information

Beskrivningar av kodens ansvar, funktion, beteende, förutsättningar och sidoeffekter.

Semantisk information ska kunna vara skapad av människor, härledd genom analys eller föreslagen av språkmodeller. Dessa ursprung ska inte blandas samman.

### 4.4 Relationer

Anrop, referenser, beroenden, dataflöden och andra kopplingar mellan informationsobjekt.

Relationer ska kunna redovisa både observerad struktur och osäkra eller härledda mål.

### 4.5 Noteringar

Mänskliga och maskinella anteckningar, kommentarer, hypoteser, varningar, motiveringar och frågor.

### 4.6 Historik och sessioner

Kopplingar till commits, ändringar, tidigare tillstånd, undersökningar, agentarbete och beslut.

### 4.7 Statistik och verifiering

Exempelvis komplexitet, storlek, ändringsfrekvens, testresultat, relationstäthet och andra mätbara egenskaper.

### 4.8 Samverkan mellan lager

Informationslagren ska kunna kombineras, filtreras och visas på olika detaljnivåer.

De behöver inte motsvara separata databaser eller dokument. Information kan i stället vara knuten till gemensamma objekt och källankare.

## 5. Tänkt, förväntat, härlett och prövat

En central princip är att skilja mellan olika slags kunskap om ett objekt eller beteende.

**Tänkt:** Vad konstruktören, utvecklaren eller en annan ansvarig avsåg.

**Förväntat:** Vad krav, specifikationer, kontrakt eller uttryckliga antaganden säger bör ske.

**Härlett:** Vad ett analysverktyg, en regel eller en språkmodell drar för slutsats utifrån tillgänglig information.

**Prövat:** Vad tester, körningar eller andra observationer faktiskt har visat under angivna förhållanden.

Dessa kategorier kan överensstämma eller motsäga varandra.

En motsägelse är inte nödvändigtvis ett fel i informationsmodellen. Den kan vara ett viktigt undersökningsresultat.

Prövade resultat ska inte automatiskt behandlas som universella sanningar. De ska kunna knytas till exempelvis version, miljö, testmetod och omfattning.

Information bör där det är relevant bära ursprung, tidpunkt, giltighetsområde, osäkerhet och verifieringsstatus.

## 6. Navigerbar och redigerbar arbetsmiljö

Den visuella miljön ska vara mer än en karta som visar kodens struktur.

Användaren ska kunna navigera mellan kod, AST, relationer, semantiska beskrivningar, historik, commits, sessioner, noteringar och analyser.

En funktion ska exempelvis kunna öppnas från en kodkarta och därifrån leda vidare till:

- Den faktiska källkoden.
- Funktionens anrop och beroenden.
- Dess historiska förändringar.
- Sessioner som arbetat med funktionen.
- Tester och analysresultat.
- Anteckningar, motiveringar och osäkerheter.

### 6.1 Redigering

Arbetsmiljön ska även kunna användas för redigering.

Det kan omfatta direkt kodredigering, strukturella kodoperationer och ändringar i informationsmodellens egna uppgifter.

Det måste vara tydligt om en åtgärd ändrar källkoden eller endast information om koden.

### 6.2 Visualisering

**2D är den preliminära huvudriktningen.**

En tredimensionell stadsliknande representation är inte ett mål i sig. Andra visualiseringskoncept kan undersökas om de ger bättre navigering, förståelse eller interaktion.

Visualisering ska stödja arbete med informationen, inte enbart presentera den.

## 7. Samarbete mellan människor och system

Människor och automatiserade aktörer ska kunna arbeta med samma objekt, relationer och tillstånd.

En människa ska kunna följa en agents undersökning, granska dess underlag, korrigera slutsatser, påverka nästa steg och själv utföra operationer.

En agent ska inte behöva styra ett grafiskt gränssnitt för att arbeta med samma information. Den kan använda motsvarande underliggande operationer genom strukturerade gränssnitt.

Mänskliga och automatiserade åtgärder ska kunna redovisas med relevant ursprung och historik.

Arbetsmiljön ska därmed stödja både självständigt arbete och samarbete mellan människor och system.

## 8. GTW, Motor2 och övriga komponenter

GUIE är ett övergripande koncept som kan byggas upp genom samverkan mellan flera projekt och komponenter.

Den exakta ansvarsfördelningen är inte fastställd i detta dokument.

### 8.1 GnawTreeWriter (GTW)

GTW kan utvecklas från ett verktyg för kodoperationer och Code Index till en mer generell informations- och operationsmotor.

Den ska kunna erbjuda strukturerad information, navigering, kontrollerade ändringar, historik och verifierbara resultat.

Deterministiska funktioner ska användas där de är tillräckliga. Språkmodeller ska kunna tillföra flexibel tolkning, problemlösning och skapande där sådana förmågor behövs.

GTW ska inte behöva förutsätta att en språkmodell alltid är närvarande.

### 8.2 Motor2

Motor2 kan fungera som en samordnande miljö för undersökningar, planering, verktygsanvändning och samarbete mellan modeller och andra funktioner.

Motor2 ska kunna konsumera GTW:s strukturerade information på lämplig detaljnivå.

Exempelvis kan en modell få en översikt, en annan få AST och relevant källkod, medan en verifierande funktion får förväntat beteende och prövade resultat.

Målet är att minska onödigt arbete med att rekonstruera information som redan finns tillgänglig.

Den visuella arbetsmiljön ska kunna visa vad Motor2 gör och låta människor påverka arbetet.

### 8.3 Gnawbone och Gnawdriver

Gnawbone är ett möjligt konceptuellt spår för komponerbara funktionsnoder, arbetsflöden och visuella komponenter.

Gnawdriver kan bidra med navigering och undersökning av externa informationsytor.

Detta innebär inte att projekten måste slås samman eller att deras arkitektur redan är beslutad.

## 9. Komponerbara funktioner och andra informationsdomäner

Den långsiktiga miljön ska inte vara begränsad till programkod.

Liknande representationer och arbetsprinciper kan undersökas för dokument, databaser, webbplatser, arbetsflöden och andra strukturerade eller delvis strukturerade informationskällor.

Komponerbara funktioner kan göra det möjligt att återanvända befintliga förmågor i olika sammanhang utan att varje arbetsflöde behöver byggas från grunden.

Hur dessa funktioner ska organiseras och kombineras är en öppen arkitekturfråga.

## 10. Utvecklingsbar förmåga

En långsiktig möjlighet är att systemet ska kunna identifiera begränsningar i sina egna funktioner och använda tillgängliga resurser för att undersöka hur förmågorna kan förbättras.

Det kan exempelvis innebära att upptäcka återkommande misslyckanden, saknade verktyg eller behov av nya arbetsflöden.

Språkmodeller kan användas som tillfälliga utvecklingsresurser, medan förbättrade deterministiska funktioner och bestående strukturer blir kvar efteråt.

Förmåga ska kunna växa genom återanvändbara operationer och bättre informationshantering, inte enbart genom större språkmodeller.

## 11. Öppen användbarhet och kommersiell inriktning

GTW ska eftersträva hög användbarhet utan konstgjorda funktionsbegränsningar eller uppenbara betalväggar.

En möjlig affärsmodell bygger främst på företagsanpassningar, integrationer, specialiserade funktioner, support och underhåll.

Kundfinansierade förbättringar kan, när avtal och förutsättningar tillåter, bidra till den allmänt tillgängliga produkten.

Den kommersiella modellen ska inte kräva att grundverktyget avsiktligt görs sämre för obetalande användare.

## 12. Arkitekturprinciper

Följande principer bör vägleda framtida diskussioner:

1. Bevara användbara observationer och strukturer i stället för att reducera information för tidigt.
2. Skilj källinformation från tolkningar och slutsatser.
3. Gör osäkerhet, motsägelser och kunskapsluckor synliga.
4. Använd gemensamma informationsobjekt bakom olika vyer och verktyg.
5. Låt människor och agenter arbeta genom kompatibla operationer.
6. Bevara spårbarhet mellan kod, historik, beslut och verifiering.
7. Prioritera fungerande, återanvändbara förmågor framför onödig komplexitet.
8. Undvik hårda beroenden av enskilda språkmodeller, externa tjänster eller presentationsformer.
9. Utveckla funktionaliteten stegvis och mät dess faktiska nytta.
10. Skilj vision, föreslagen arkitektur, beslut och implementerad funktionalitet.

## 13. Öppna frågor

- Hur ska stabila objektidentiteter hanteras när kod flyttas, delas eller byter namn?
- Hur ska semantiska beskrivningar uppdateras när källkoden förändras?
- Hur ska information från olika källor jämföras och motsägelser hanteras?
- Vilka delar av en agents arbetsprocess ska kunna visualiseras och påverkas?
- Hur ska sessionshistorik kopplas till specifika kodändringar?
- Vilka redigeringsoperationer bör finnas direkt i kartan?
- Hur ska olika informationslager presenteras utan att gränssnittet blir överlastat?
- Hur kan en generell informationsmodell växa fram utan att försämra GTW:s befintliga kodfunktioner?
- Hur kan systemets förmågor mätas oberoende av vilken språkmodell som används?

## 14. Dokumentets fortsatta utveckling

Detta dokument är avsett att uppdateras stegvis.

Nya idéer, tidigare diskussioner och konkreta erfarenheter kan föras in utan att automatiskt betraktas som beslutade funktioner.

Vid större uppdateringar bör det framgå vad som är nytt, vad som har omformulerats och om något tidigare antagande har ändrats.

Det befintliga Code Index-visionsdokumentet och utvecklingsplanen kan fortsätta vara mer avgränsade dokument. GUIE beskriver den bredare riktningen och ska inte ersätta dem.
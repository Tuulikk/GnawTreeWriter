# Från Code Index till en användarägd informationsmotor

**Status:** Koncept, målbild och undersökningsunderlag – ingen beslutad arkitektur eller implementation.

## Grundidé

Undersöka hur tekniken bakom GnawTreeWriters (GTW) Code Index, tillsammans med andra befintliga komponenter, kan utvecklas och återanvändas som grund för en generell informations-, relations-, undersöknings- och sökmotor.

Motorn ska inte enbart hitta innehåll som matchar sökord. Den ska hjälpa användaren att hitta, utforska, organisera, jämföra, förstå och återfinna information genom dess egenskaper, sammanhang, klassificeringar och relationer.

Systemet ska kunna arbeta med både befintlig kunskap och information som ännu inte har analyserats tillräckligt. Det ska kunna identifiera vad som är känt, vad som är osäkert, vad som saknas och vilka ytterligare undersökningar som kan förbättra resultatet.

Användaren ska kunna äga och påverka informationsstrukturen, kombinera lokala och externa källor och utveckla den över tid.

AI ska kunna bidra med analys, klassificering och undersökning, men grundläggande funktioner ska inte vara beroende av en LLM.

## 1. Sökning, navigering och undersökning

Tre kompletterande arbetssätt:

- **Sökning:** Hitta identifierade objekt, dokument, text, egenskaper och andra informationsenheter.
- **Relationsnavigering:** Fortsätta från ett objekt till andra genom meningsfulla kopplingar utan att behöva formulera nya sökord.
- **Undersökning:** Utforska ett ämne, följa flera spår, jämföra information, upptäcka samband, identifiera kunskapsluckor och vid behov inhämta eller analysera ytterligare information.

Relationer ska inte bara vara navigeringslänkar. De ska kunna beskriva varför objekt hör ihop och själva vara möjliga att undersöka.

Navigering ska kunna ske både mellan objekt och genom relationer, med möjlighet att begränsa vilka kopplingar som är relevanta för den aktuella undersökningen.

En undersökning ska kunna fortsätta från tidigare resultat och återanvända redan utfört arbete.

## 2. Dynamiska brusfilter och klassificering

Inspirerat av facetterad filtrering hos exempelvis Prisjakt, men utan kravet att alla kategorier måste vara definierade i förväg.

Användaren ska kunna beskriva vilka egenskaper som ska inkluderas eller uteslutas, även när informationen saknar motsvarande metadata.

Systemet ska kunna:

- Återanvända befintliga klassificeringar, taggar, egenskaper och relationer.
- Identifiera objekt och relationer som saknar nödvändig klassificering.
- Generera klassificeringar vid behov genom regler, strukturell analys, parser, AI eller andra tillgängliga metoder.
- Skapa tillfälliga, sparade eller återanvändbara filter.
- Filtrera både objekt och relationer utan att förändra eller förstöra grundinformationen.
- Redovisa varför något inkluderats, uteslutits eller lämnats oavgjort.
- Ompröva klassificeringar när källinformation eller analysmetoder förändras.

Brus är kontextberoende. En relation eller ett objekt kan vara korrekt och värdefullt men ändå irrelevant för den aktuella undersökningen.

Systemet ska därför kunna skapa olika vyer över samma informationsmängd utan att behöva förändra dess grundstruktur.

**Avsaknad av klassificering får inte automatiskt tolkas som en negativ klassificering.**

Det ska finnas en tydlig skillnad mellan att något inte uppfyller ett villkor, att det ännu inte undersökts och att undersökningen inte kunde slutföras.

## 3. Informationsobjekt, relationer och identitet

Motorn ska kunna representera olika slags information, exempelvis kodsymboler, dokumentavsnitt, webbsidor, begrepp, observationer, påståenden och härledda uppgifter.

Viktiga egenskaper:

- Identifierbara objekt med identitet som så långt möjligt kan överleva förändringar.
- Versionsbundna positioner och ankare till källmaterial.
- Typade relationer mellan objekt.
- Relationer som egna identifierbara informationsförekomster när det behövs.
- Ursprung, källhänvisningar och spårbarhet.
- Förändringar, historik, giltighet och inaktualitet.
- Klassificeringar och andra härledda egenskaper.
- Möjlighet att kombinera information från olika domäner.
- Skillnad mellan en observation, ett påstående och en fastställd koppling.

Systemet ska inte förutsätta att alla objekt har samma typ av identitet eller ankare. Kod, dokument, webbinnehåll och andra informationskällor kan kräva olika representationer.

Gemensamma gränssnitt ska möjliggöra samverkan utan att all information måste tvingas in i samma interna format.

## 4. Kunskapsstatus, säkerhet och verifierbarhet

Systemet ska inte bara registrera *vad* det tror att något är, utan även kunna redovisa **vad det faktiskt vet, på vilka grunder och vilka begränsningar som finns**.

Fyra centrala dimensioner:

- **Identifiering:** Vad är objektet, relationen eller observationen?
- **Kunskapsstatus:** Vad är fastställt, härlett, möjligt, motsagt eller fortfarande okänt?
- **Underlag:** Vilken källa, observation, regel eller analys stöder uppgiften?
- **Undersökningsmöjlighet:** Vad kan systemet göra för att minska kvarvarande osäkerhet?

Dessa dimensioner ska hållas isär. En observation kan vara säker även när dess tolkning är osäker.

Exempelvis kan GTW säkert identifiera att kod innehåller ett funktionsanrop utan att ännu veta vilken funktionsdefinition anropet avser.

Systemet bör kunna skilja mellan:

- Direkt observerade egenskaper.
- Deterministiskt fastställda slutsatser.
- Härledda slutsatser med kända förutsättningar.
- Möjliga kandidater.
- Olösta eller otillräckligt undersökta frågor.
- Motstridiga uppgifter och ogiltigförklarade slutsatser.

Ursprung, verifiering, upplösningsstatus och eventuell osäkerhet bör representeras som separata egenskaper när det är relevant, snarare än sammanfattas i en enda generell confidence-poäng.

**Målet är inte att alltid uttrycka osäkerhet, utan att uttrycka rätt grad och typ av säkerhet för varje påstående.**

När något är väl fastställt ska systemet kunna redovisa detta tydligt och återanvända slutsatsen.

## 5. Aktiv kunskapskomplettering och fortsatt undersökning

Att sakna information ska inte automatiskt innebära att en undersökning avslutas.

Systemet ska kunna identifiera när en fråga inte går att besvara med befintliga uppgifter och undersöka vilka tillgängliga metoder som kan föra arbetet vidare.

Ett möjligt arbetsflöde:

1. Formulera ett sök-, analys- eller undersökningsvillkor.
2. Använd befintliga index, observationer, relationer och klassificeringar.
3. Identifiera vilka delar som redan kan besvaras och vilka som fortfarande är oklara.
4. Avgör om ytterligare undersökning är möjlig och motiverad.
5. Använd lämpliga metoder för att komplettera informationen.
6. Uppdatera kunskapsstatus, klassificeringar och eventuella relationer.
7. Fortsätt den ursprungliga frågan med förbättrat underlag.

Kompletteringen kan exempelvis bestå av ny strukturell analys, kontroll av källkod, jämförelse mellan dokument, relationsupplösning eller AI-baserad klassificering.

Systemet ska samtidigt kunna avgöra när det finns tillräckligt underlag för att avsluta undersökningen.

Det ska undvika att upprepa redan utförda kontroller utan anledning.

Fortsatt undersökning bör styras av informationsbehov, förväntad nytta, tillgängliga resurser och kostnad, inte enbart av att osäkerhet existerar.

## 6. Komponerbar sökning och analys

Befintliga och framtida mekanismer bör kunna samverka:

- Textsökning och fulltextsökning.
- Strukturell sökning.
- Metadata-, egenskaps- och kategorifilter.
- Relations- och grafoperationer.
- Historik och förändringsanalys.
- Semantisk sökning.
- Regelbaserad och dynamisk klassificering.
- Mängdoperationer som snitt, union och differens.
- Källkontroll och kompletterande undersökning.

En fråga ska kunna kombinera flera mekanismer utan att varje komponent måste implementera hela frågan själv.

Delresultat bör kunna innehålla identifierade objekt, källversion, underlag och uppgifter om hur fullständig undersökningen är.

Det är särskilt viktigt att skilja mellan:

- Ett fullständigt resultat utan träffar.
- Ett ofullständigt resultat.
- Ett resultat där vissa objekt fortfarande är oklassificerade.
- Ett resultat som bygger på inaktuell information.
- Ett resultat där identifiering eller relationsupplösning är osäker.

Frågeplaneringen bör kunna välja billiga och tillförlitliga operationer först, återanvända tidigare delresultat och begära dyrare analyser endast när de behövs.

Ett begränsat semantiskt topp-N-resultat ska exempelvis inte behandlas som bevis för att alla övriga objekt saknar relevans.

## 7. Code Index och GTW som teknisk grund

GTW har redan mekanismer för inkrementell indexering, AST-analys, objekt, relationer, grafoperationer, historik, taggar, etiketter, strukturella regler, semantisk sökning och andra former av informationsurval.

Dessa funktioner finns delvis i separata komponenter.

En central undersökningsfråga är hur mycket som kan åstadkommas genom att förbättra och kombinera befintliga mekanismer, snarare än att bygga en helt ny motor.

### Förbättring av Code Index

Code Index bör kunna bli bättre på att:

- Identifiera och klassificera kodobjekt och relationer.
- Skilja faktiska beroenden från möjliga eller felaktigt upplösta kopplingar.
- Använda tillgänglig information om namnrymder, importer, typer och struktur.
- Redovisa varför en relation har etablerats.
- Behålla olösta kandidater utan att felaktigt presentera dem som verifierade.
- Identifiera vad som krävs för ytterligare upplösning.
- Exportera användbara beroende- och relationskartor.
- Stödja kombinerade frågor över kodstruktur, relationer och historik.

GTW ska kunna förbättra sin precision och analysförmåga med befintliga mekanismer, utan att vara beroende av AI, externa språkservrar eller ett framtida extra kodanalyslager.

Ett sådant extra lager kan senare bidra med ytterligare information och bättre upplösning, men ska vara ett komplement.

### Återanvändbar Rust-crate

Undersök möjligheten att göra Code Index eller dess återanvändbara delar till en fristående Rust-crate.

En första extraktion behöver inte samtidigt göra informationsmodellen generell.

Möjliga delade funktioner är lagring, objekt- och relationshantering, frågeoperationer, historik och indexuppdatering.

GTW kan fortsätta äga specialiserad kodanalys och AST-hantering.

En generell informationsmotor som behöver analysera kod ska kunna använda GTW:s befintliga förmågor i stället för att behöva duplicera dem.

Exakt uppdelning mellan crate, GTW och eventuell generell kärna är ännu inte beslutad.

RoaringBitmap, FTS och andra söktekniker är möjliga komponenter eller inspirationskällor, inte förutbestämda arkitekturval.

## 8. Förhållande till andra projekt

**GTW:** Utvecklar och använder kodspecifik indexering, relationsanalys, kodförståelse och andra undersökningsfunktioner. Ska också kunna dra nytta av generella mekanismer.

**Motor2:** Kan använda den generella informationsmotorn för bredare sökning, undersökning, kunskapskomplettering och assistans. Den ska kunna använda GTW för specialiserad kodanalys när det behövs.

**Gnawdriver:** Kan samla in, extrahera, kartlägga och navigera information från webbplatser och andra källor. Dess fångade navigeringsrepresentation kan vara en källa till eller användare av den bredare informationsmotorn.

**Framtida analyskomponenter:** Parser, regler, klassificerare och AI-modeller kan bidra med observationer, relationer och slutsatser genom tydliga gränssnitt.

Ingen integration ska förutsättas innan ansvar, beroenden och praktiska behov har undersökts.

## 9. Utvecklingsprinciper

- **Återanvänd det som finns:** Inventera befintliga förmågor innan nya komponenter skapas.
- **Förbättra utan att försvaga:** GTW:s nuvarande funktioner ska bevaras och helst stärkas.
- **Separera observation och slutsats:** Ett upptäckt mönster är inte automatiskt en verifierad relation.
- **Var tydlig med kunskapsgränser:** Redovisa både säkerhet och osäkerhet där det är relevant.
- **Undersök vidare när det tillför värde:** Okänd information är en möjlig arbetsuppgift, inte alltid en slutpunkt.
- **Undvik onödigt arbete:** Återanvänd analyser och fastställda slutsatser så länge de är giltiga.
- **Behåll spårbarhet:** Härledda uppgifter ska kunna kopplas till källor, analysmetoder och relevanta versioner.
- **Stöd flera informationsdomäner:** Gemensamma gränssnitt ska inte kräva identiska interna representationer.
- **Håll AI utbytbart:** Deterministiska metoder ska kunna användas självständigt.
- **Utveckla stegvis:** Testa gränssnitt och funktioner med små experiment innan större omstruktureringar beslutas.

## 10. Öppna frågor

### Befintliga förmågor

1. Vilka GTW-komponenter kan redan kombineras till en mer generell sök- och undersökningsmotor?
2. Vilka funktioner saknas faktiskt, och vilka finns men är svåra att kombinera?
3. Hur kan GTW:s egna verktyg förbättras för att analysera och kartlägga GTW självt?

### Kunskap och relationer

4. Hur ska objekt, relationer, observationer och klassificeringar representeras gemensamt?
5. Hur kan systemet skilja mellan säker identifiering, härledning, kandidater och olösta frågor?
6. Hur ska underlag, verifiering och motstridiga uppgifter hanteras?
7. Hur kan relationer bli självständigt undersökningsbara utan onödig komplexitet?

### Sökning och klassificering

8. Hur ska dynamiska filter kunna begära ny klassificering när befintlig information inte räcker?
9. Hur kan klassificeringar och delresultat återanvändas effektivt?
10. Hur kan systemet avgöra när vidare undersökning behövs respektive när underlaget är tillräckligt?
11. Hur ska olika sök- och analysmetoder samverka med bibehållen spårbarhet och korrekt hantering av ofullständiga resultat?

### Arkitektur och utveckling

12. Vilka delar av Code Index bör kunna bli en återanvändbar Rust-crate?
13. Vilka funktioner bör fortsätta vara GTW-specifika?
14. Hur ska en generell informationsmotor använda GTW utan att duplicera kodanalysen?
15. Vilka små experiment kan verifiera de viktigaste antagandena utan omfattande omstrukturering?

## Övergripande mål

En användarägd informationsmotor som inte bara besvarar frågan *”var finns det jag söker?”* utan också hjälper användaren att undersöka:

- *Vad är detta och vad hör ihop med det?*
- *Varför finns kopplingen och vad stöder den?*
- *Vad vet systemet säkert och vad är fortfarande osäkert?*
- *Vilken information saknas?*
- *Vad kan undersökas vidare för att förbättra förståelsen?*
- *När finns tillräckligt underlag för att gå vidare eller avsluta?*

Systemet ska stödja och förstärka användarens egen förståelse och arbetsförmåga genom sökning, navigering, klassificering, relationer och aktiv undersökning.

**Målet är inte att ersätta användarens bedömning, utan att göra information mer tillgänglig, begriplig, undersökningsbar och användbar – med så mycket automatisering som faktiskt hjälper.**
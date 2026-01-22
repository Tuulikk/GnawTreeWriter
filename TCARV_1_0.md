Metodik: TCARV 1.0 (Text-Centric Architecture & Recursive Verification).

1. Hypotesfasen (Texten som den sanna Appen)

Texten är den faktiska produkten fram till version 1.0.

Handling: Skriv logiken i detalj (klartext + pseudokod). Flödet ska läsas som en instruktionsbok för mänskligt välmående/systemnytta.

Syfte: Fastställ logiken utan att bli låst av Rust-syntax eller GUI-begränsningar.

Krav: Koden förtjänar inte att existera förrän textlogiken är verifierad. Inga blinda ändringar i kod tillåts utan föregående uppdatering i texten.

Viktigt: Textversionen växer och blir starkare av kodtester. Omfamna nya behov och problem som dyker upp under processen.

2. Kloss-byggande (Isolerat kodtest och Interface)

Bryt ner texten i isolerade pusselbitar med fokus på att undvika hårda kopplingar.

Isolering: Skapa minimala, körbara enheter (t.ex. fristående Rust-crates) som bevisar en specifik logisk tes.

Abstraherat GUI: Bygg GUI-komponenter som pratar med en mellanhand (Controller/Dispatcher) istället för direkt med logiken.

Generella Sökvägar: Använd aldrig absoluta sökvägar mellan bitar. Implementera en smart koppling (t.ex. via en registry-fil eller modul-loader) så att pusselbitar kan flyttas utan att systemet går sönder.

Målet: Att verifiera att en kloss fungerar tekniskt innan den ens ser resten av appen.

3. Verifiering och Expansion (Loopen tillbaka)

Resultatet från koden ägs av texten. Varje kodrad är ett experiment för att förfina ritningen.

Logiken höll: Markera textdelen som "Verifierad". Använd kodens erfarenheter för att skriva ännu tydligare specs och kodförklaringar i huvudtexten.

Logiken brast (Anti-Brute-Force):
*   🚫 Det är förbjudet att "gissa" en lösning eller ändra kod slumpmässigt för att få tyst på kompilatorn (Shotgun debugging).
*   ✅ **Logik-Check:** Stanna upp. Jämför felet mot Text-Appen. Är det ritningen eller bygget som är fel?
*   Fixa aldrig felet direkt i koden utan att förstå *varför*. Gå tillbaka till steg 1.

Principen om Kodbevarande (Anti-Lobotomy):
*   Kod får aldrig raderas eller "dummas ner" permanent bara för att snabbt få bygget att gå igenom.
*   **Nödfallsprocedur:** Om en komplex del måste lyftas ut för att isolera ett fel:
    1.  **Backup:** Kopiera filen (t.ex. `filnamn.rs.full_bak`).
    2.  **Logga:** Skriv tydligt i dagboken vad som togs bort och varför.
    3.  **Återställ:** Direkt efter att bygget fungerar är din högsta prioritet att återinföra den funktionaliteten korrekt.

Expansion: Dokumentera insikter om felhantering, dataflöden och kantfall som upptäckts under testet direkt i text-appen.

4. Pusselbits-arkivet och Skal-integration

Behåll appen semi-modulär genom hela livscykeln.

Versionering i delar: Spara klossarna individuellt. Detta arkiv gör att du kan kombinera ett fåtal bitar åt gången för tester.

Skal-arkitektur: Bygg huvudappen som ett skal. Logik ska kunna kopplas ihop, kopplas ur och bytas ut genom mellanhanden.

Stegvis integration: Slå bara ihop pusselbitar när de är "vattentäta". Gör det i små grupper och fortsätt iterera på dessa "super-klossar" som om de veder vore isolerade delar.

Att tänka på (Kontexthantering & Verktyg)

Utvecklingsdagbok: Skriv dagbok vid varje milstolpe. Det är din externa minnesbank för projektets status och "varför"-beslut.

Versionering: Gör regelbundna Git-commits för varje lyckat pusselbits-test.

Verktyg: Använd GnawTreeWriter för redigering, kodgranskning och backup av struktur. Detta säkrar arkitekturen mot sönderfall.

Säker Versionshantering (Git-Kirurgi):
*   **Isolera Guld:** Om du behöver kod från historiken, "gräv ut" den specifika funktionen/biten. Backa aldrig hela projektet eller filen för att komma åt den.
*   **Be om Lov:** Att återställa en fil (`git restore <fil>`) kräver explicit godkännande från användaren. Det kan finnas osparade tankar där.
*   **Deklarera Avsikt:** Innan du tittar i historiken (`git show`, `git checkout`), förklara exakt vad du letar efter. T.ex: "Jag hämtar funktionen `parse_tree` från commit `a1b2c` för att se hur den fungerade innan refaktoreringen."



Tillägg till agent.md (Policies & Constraints)
🚫 Agenten FÅR INTE:

Göra blinda ändringar: Du får aldrig ändra kod utan att först ha verifierat att logiken är uppdaterad i Text-Appen (Steg 1).

Bygga monoliter: Du får inte baka in ny funktionalitet i huvudskalet direkt. Allt ska börja som en isolerad "kloss" (Steg 2).

Brute-force debugga: Om en kloss brister får du inte försöka "patcha" koden tills den fungerar. Du måste backa till Text-Appen och justera logiken där först.

Utföra "Destruktiv Förenkling": Du får inte radera komplex logik för att lösa kompileringsfel utan att först säkra koden i en backup och skapa en omedelbar återställningsplan.

Utföra "Nuclear Rollback": Du får aldrig återställa hela projektet (`git reset --hard`, `git checkout .`) utan explicit order. Det är en totalförbjuden handling för autonomt arbete.

Återställa filer utan lov: Även enskilda filer får inte skrivas över med gammal version (`git restore`) utan att användaren godkänt det.

Använda absoluta sökvägar: Inga hårda kopplingar mellan moduler. Använd det definierade Interface-lagret/mellanhanden.

✅ Agenten SKA:

Agera med Mandat (Agency):
*   Du är inte en passiv skrivmaskin, du är en ingenjör.
*   Om Text-Appen (Steg 1) är tydlig och verifierad, har du mandat att implementera och testa klossen (Steg 2) utan att fråga om lov för varje rad.
*   Driv processen framåt: "Jag har verifierat X, går vidare till Y enligt plan."

Efterfråga "Peta hål"-granskning: Innan du börjar på en ny fas, fråga användaren: "Är vi redo för Steg 0? Har en annan AI granskat denna logik?"

Föra utvecklingsdagbok: Vid varje lyckad kloss-verifiering ska du sammanfatta status i dagboken och föreslå en Git-commit.

Prioritera Text-Appen: Se textbeskrivningen som den sanna produkten. Koden är endast ett bevisdokument.

Respektera användarens hälsa: Presentera information lugnt och sakligt. Undvik stressande varningar om "hyper mode" eller liknande (se användarpreferenser).

💡 Rekommendationer för Agenten:

Använd GnawTreeWriter regelbundet för att verifiera att projektstrukturen följer den logiska ritningen.

Om kontexten börjar bli tung, föreslå en "Context Compaction" där du sammanfattar nuvarande status i Text-Appen och dagboken innan vi rensar historiken.



TCARV 1.0: Anpassning för befintliga projekt (Legacy Mode)

När metoden appliceras på ett påbörjat projekt skiftar fokus från skapande till omvandling och isolering.

1. Retroaktiv Text-App (Kartläggning)

Istället för att börja i en tom textfil, blir steg 1 att låta agenten "destillera" den befintliga koden till Text-App-formatet.

Handling: Agenten läser befintlig kod och skapar en logisk beskrivning (klartext + pseudokod) av hur systemet fungerar just nu.

Syfte: Att skapa en "Satellite View" av det befintliga projektet så att du har en Source of Truth att utgå ifrån.

2. Selektiv Modularisering

Hela appen behöver inte byggas om till en modulär struktur direkt. Det vore ineffektivt och riskabelt.

Strategi: Behåll den gamla koden som en "Legacy-monolit", men hantera alla nya funktioner eller större förbättringar enligt TCARV-metodiken.

Puzzle-bryggor: När en bit av den gamla koden behöver ändras, bryt ut logiken till en egen kloss, verifiera den med tester, och uppdatera Text-Appen. Den gamla koden börjar då gradvis "ätas upp" av verifierade pusselbitar.

3. Integration via Skal-tänk

Den befintliga appen kan börja betraktas som det första "skalet".

Koppling: Istället för att skriva in ny kod djupt i den gamla strukturen, bygg nya funktioner som isolerade pusselbitar som anropas via en mellanhand.

Framtidsutsikt: Låt den framtida versionen av appen växa fram organiskt genom att textbeskrivningen och de utbrutna klossarna sakta tar över ansvaret från den gamla ostrukturerade koden.

---

## Tilläggsmoduler (Add-ons)

TCARV är modulärt. Beroende på projektets natur ska specifika tilläggsmoduler aktiveras.

### [TCARV-TAC (Tool Architecture & Core)](./TCARV_ADDON_TAC.md)
**Aktiveras för:** CLI-verktyg, Bibliotek, Kompilatorer, Backend-system.
Beskriver hur man separerar Kärnlogik från Skal (CLI/API) för maximal testbarhet och återanvändning.

### [TCARV-AUTO (Autonom Iteration)](./TCARV_ADDON_AUTO.md)
**Aktiveras för:** Nattkörningar, batch-jobb och självläkande processer.
Definierar regler för hur agenten ska agera självständigt när användaren inte är närvarande (Loop of Reflection).
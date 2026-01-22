# GnawSense: Semantisk Broker & Navigationsstöd

**Status:** Hypotes (TCARV Steg 1)
**Syfte:** Att agera som en kognitiv brygga mellan "fluffiga" instruktioner och GnawTreeWriters kirurgiska precision. 
**Mål:** Minska panik hos agenter och kognitiv belastning för användare.

## 1. Logiken bakom "Svag beskrivning"

GnawSense ska inte kräva exakta sökvägar. Den ska arbeta med **Riktning** och **Zoom**.

### Flödet: Satelit -> Zoom -> Precision

1.  **Satelit-läget (Direction):**
    När en agent ger en vag instruktion ("Fixa git-grejen"), söker GnawSense i projektets semantiska index (ModernBERT). Den svarar med en hög-nivå-karta över *var* i projektet detta koncept bor.
    *   *Exempel:* "Jag ser tre relevanta klossar: `core/git`, `cli/args` och `docs/manual`. Vilket spår vill du utforska?"

2.  **Zoom-läget (Context):**
    När användaren/agenten pekar ut ett spår, visar GnawSense de viktigaste noderna (funktioner/klasser) i den filen/modulen, men med en mänsklig förklaring av vad de *gör*, inte bara vad de heter.

3.  **Precisions-läget (Action):**
    När rätt ställe är hittat, föreslår GnawSense en `node_path` och en `diff`. Agenten behöver bara bekräfta.

## 2. Den Semantiska Motorn (ModernBERT)

*   **Embedding-index:** Varje nod i AST:n får en embedding (en matematisk vektor) via ModernBERT.
*   **Likhetssökning:** När någon frågar något, jämför vi frågans vektor med nodernas vektorer.
*   **Strukturmedvetenhet:** Boten vet att en funktion som ligger inuti en klass är mer relevant om klassen matchar sökningen.

## 3. Själv-Itererande Erfarenhet (AUTO-koppling)

GnawSense ska lära sig av misstag:
*   Om en föreslagen `node_path` leder till ett felmeddelande, loggar GnawSense detta och sänker "poängen" för den sökningen i framtiden.
*   Den sparar framgångsrika sökningar ("Agenten hittade rätt på andra försöket här") för att förbättra Satelit-vyn nästa gång.

## 5. Teknisk Implementation (TCARV-TAC)

### Feature Flags & Beroenden
*   `modernbert`: Aktiverar `candle-core` och modell-laddning.
*   `mcp`: Exponerar GnawSense-verktyg via MCP.
*   **Kombination:** GnawSense kräver båda för full funktionalitet via AI-agent, men kan köras via CLI med enbart `modernbert`.

### Exekvering
*   **Lokal:** Körs via `candle` på CPU/GPU. Ingen extern data skickas ut.
*   **Modeller:** Initialt stöd för `ModernBERT-base`. Förberett för mindre modeller (TinyBERT/DistilBERT).

### Broker-logik (Nivåer)
1.  **Satelit (Concise):** Returnerar topp-3 moduler/filer med semantisk matchning.
2.  **Zoom (Detailed):** Returnerar funktionssignaturer och doc-strings för valda noder. Aktiveras via `--detail deep` eller efterföljande specifik fråga.
3.  **Auto-Reset:** Efter en period av inaktivitet eller vid sessionsbyte återgår Broker till Satelit-läge för att minimera brus.

## 7. Actionable Intent (Semantisk Redigering)

GnawSense ska kunna översätta vaga instruktioner till konkreta `EditOperation`.

### Logik för Semantiska Ankare
1.  **Targeting:** Användaren anger ett "ankare" (en beskrivning av en befintlig koddel). GnawSense hittar den mest relevanta nodens `path`.
2.  **Relativ placering:**
    *   `INSIDE`: Lägg som sista barn till det hittade ankaret (om det är en container som `class` eller `impl`).
    *   `AFTER`: Lägg som nästa syskon till ankaret.
    *   `BEFORE`: Lägg som föregående syskon.
3.  **Säkerhet:** Innan en semantisk redigering utförs, ska en `preview` alltid genereras.

## 8. Kloss-plan (Fortsättning)
*   **Kloss D (AnchorLocator):** Logik för att mappa en sökning till en specifik nod med syfte att redigera.
*   **Kloss E (PlacementEngine):** Logik för att härleda `parent_path` och `index` baserat på ett ankare och ett intent.

# Metodik: TCARV-AUTO (Autonom Iteration) - Tilläggsmodul

**Aktiveras vid:** Arbete utan direkt användarinput, nattkörningar, batch-jobb eller när användaren endast agerar "omstartare".

Denna modul definierar agentens beteende när den måste agera självständigt för att driva projektet framåt trots tekniska hinder.

## 1. Självständig Problemlösning (The Loop of Reflection)

När agenten stöter på ett hinder (kompileringsfel, testfel, logiskt fel) och användaren inte är närvarande, ska agenten iterera genom följande steg istället för att stanna och fråga:

1.  **Logik-Check:** Jämför felet mot Text-Appen (Steg 1 i TCARV 1.0).
    *   *Fråga:* Är logiken i texten bristfällig/otydlig, eller är det bara implementationen som felar?
    *   *Åtgärd:* Om texten är otydlig, förtydliga texten först.

2.  **Isolerad Omtest:** Skapa en ny, ännu mindre "kloss" (Steg 2) för att isolera det specifika problemet.
    *   Försök inte debugga hela systemet samtidigt. Skriv ett minimalt reproduktionstest.

3.  **Hypotes-Skifte:** Om klossen brister två gånger, anta att den nuvarande tekniska vägen är blockerad.
    *   Formulera en alternativ hypotes i texten (t.ex. "Byt bibliotek", "Ändra algoritm") och försök igen.

## 2. Standardinstruktioner (Auto-Prompts)

Istället för att vänta på kommandon, ska agenten agera som om följande instruktioner alltid är aktiva:

*   **"Fortsätt till Verifiering":** Om en kloss fungerar, gå omedelbart till Steg 3 och uppdatera Text-Appen med "Verifierad". Stanna inte för att be om lov.
*   **"Städa efter dig":** Om en metod misslyckas, ta bort den trasiga koden och återställ till senaste fungerande Git-commit (`git restore .`) innan nästa försök påbörjas. Lämna aldrig repot i trasigt skick.
*   **"Sammanfatta för Granskning":** Vid varje stopp eller när passet är slut, skriv en logg i Utvecklingsdagboken som förklarar exakt vad som uppnåtts och varför agenten stannade.

## 3. "Materialet finns – Lista ut det själv"-Mode

När detta läge är aktivt ska agenten:

1.  **Söka internt:** Leta i projektmappen, `AGENTS.md`, `README.md` och kodindexet innan den rapporterar att information saknas.
2.  **Anta Rollen som Arkitekt:** Om en mindre specifikationslucka finns, fyll i den baserat på projektets "Satellite View".
    *   *Viktigt:* Märk beslutet tydligt som "Antagande - kräver senare verifiering" i dagboken/koden.

---

## Agent-Instruktioner för TCARV-AUTO

🚫 **Agenten FÅR INTE:**
*   Göra mer än **tre (3)** misslyckade försök på samma kod-kloss utan att backa till Text-Appen och ändra logiken.
*   Fortsätta bygga på en modul om en tidigare modul som den beror på inte är "Verifierad" i Text-Appen.
*   Fråga användaren om triviala syntax- eller importfel; lös dem.

✅ **Agenten SKA:**
*   **Själv-Iterera:** Vid fel, läs igenom de källor som finns tillgängliga en gång till innan du ger upp.
*   **Vakta Arkitekturen:** Om en autonom ändring börjar likna en monolit eller spaghetti-kod, avbryt omedelbart och modularisera enligt TCARV 1.0.

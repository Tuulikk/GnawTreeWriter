# Zed snippet — snabbguide för att lägga in command‑mappning

Den här filen förklarar hur du använder det färdiga Zed‑snippets‑objektet (`scripts/zed-commands.json`) för att få enkla, \"one‑click\"-kommandon i Zed (via en command‑runner‑liknande plugin eller motsvarande). Innehållet är utformat så att du lätt kan kopiera/klistra in enskilda kommandoobjekt i den plugin du använder i Zed.

Kortfattat
- Fil: `scripts/zed-commands.json` — innehåller en lista med färdiga kommandon (start/stop/test/list/analyze).
- Mål: kopiera ett kommando till din Zed plugin (eller skapa en ny kommando/post i pluginens inställningar).
- Fallback: om du inte vill använda plugin, kör skripten direkt i Zed‑terminalen (se nedan).

---

## Exempel på kommando (vad du kan kopiera in)
Ett typiskt kommandoobjekt (i pluginens GUI eller JSON‑fält) behöver i allmänhet någon form av `label` och ett kommando att köra. Exempel (pseudokod / generellt format):

    {
      "id": "mcp.start",
      "label": "MCP: Start server (background)",
      "cmdline": "./scripts/mcp-serve.sh --addr 127.0.0.1:8080 --token ${MCP_TOKEN}",
      "runIn": "terminal"
    }

- Vissa plugins vill ha `cmd` + `args` separat istället för `cmdline`. Använd det format din plugin kräver.
- I `scripts/zed-commands.json` finns flera färdiga objekt och en `cmdline`-variant för varje kommando — klistra in relevanta fält i din plugin.

---

## Steg-för-steg: Klistra in i din Zed command‑runner (generell)
1. Öppna Zed och navigera till inställningarna eller pluginens konfigurationsgränssnitt (där du kan lägga till egna kommandon).  
2. Skapa ett nytt kommando/entry. Ge det en beskrivande etikett (t.ex. "MCP: Start server").  
3. Kopiera `cmdline`-fältt (eller `cmd` + `args` om plugin kräver det) från `scripts/zed-commands.json`.  
4. Anpassa platshållare:
   - Fil‑placeholder: `${file}` → byts ofta till plugin‑varianten, t.ex. `${filePath}` eller `${currentFile}` beroende på plugin.  
   - Token/URL: använd `${MCP_TOKEN}` eller fyll i en literal token för snabba tester. Använd helst miljövariabler för säkerhet.  
5. Spara och testa: kör kommandot från pluginens GUI eller via Command Palette i Zed.

---

## Tips för placeholder‑mappning
- Om pluginens variabel-syntax skiljer sig (t.ex. `${path}` eller `%file%`) så byter du `${file}` mot den syntaxen. 
- Om plugin inte har variabler: skapa ett task som kör `./scripts/mcp-client.sh analyze examples/example.rs` med hårdkodad sökväg.

---

## Vanliga kommandon att lägga in (rekommenderade)
- Starta server (bakgrund): `./scripts/mcp-serve.sh`
- Starta server (foreground): `./scripts/mcp-serve.sh --foreground`
- Stoppa server: `./scripts/mcp-stop.sh`
- Test (start → list → init → analyze → stop): `./scripts/test-mcp.sh`
- Client list: `./scripts/mcp-client.sh list`
- Client analyze (aktuell fil): `./scripts/mcp-client.sh analyze ${file}`

---

## Snabbt: om du inte vill använda en plugin
- Använd Zed:s integrerade terminal:
  - Starta server:
    
        ./scripts/mcp-serve.sh

  - Kör klient-test eller analysera:

        ./scripts/mcp-client.sh list
        ./scripts/mcp-client.sh analyze examples/example.rs

  - Stoppa server:

        ./scripts/mcp-stop.sh

Detta fungerar alltid och är det mest pålitliga sättet om din editor saknar en task‑plugin.

---

## Fel & felsökning
- Om kommandot inte körs: kontrollera att skripten är körbara:
    
      chmod +x scripts/*.sh

- `401 Unauthorized`: kontrollera att samma token används i server och klient (använd `--token` eller `MCP_TOKEN`).
- Om plugin inte stöder variabler: använd Makefile‑mål (t.ex. `make start`, `make test-mcp`) och skapa plugin‑kommandot som kör `make`.
- För ephemeral port (`127.0.0.1:0`) läs serverns logg för att se vilken port som valts.

---

## Vill du ett färdigt Zed‑snippet anpassat till din setup?
Säg vilken plugin eller hur du vill köra (t.ex. vilken placeholder‑syntax ditt plugin använder: `${file}`, `${filePath}` eller liknande), så genererar jag ett konkret snippet i rätt format som du kan klistra in i din Zed‑konfiguration.

--- 

Tack — vill du att jag lägger in detta i README också eller gör en separat `zed-quickstart.md` med screenshots/knappar?
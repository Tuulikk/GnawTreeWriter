# Editor & GUI integration — köra MCP-servern och klientexempel

Syftet med denna sida är att snabbt visa hur du kör och testar GnawTreeWriter som MCP-server i olika utvecklingsmiljöer (Zed, VS Code, Gemini CLI, andra editors/GUI). Filen beskriver både terminal-first-workflow och hur du kan koppla knappar / tasks i editors för enkel one‑click-användning.

Innehåll
- Snabbstart (terminal)
- Skript och Makefile (lokala verktyg)
- VS Code: tasks och GUI
- Zed: hur du kör och integrerar (terminal + GUI)
- Gemini CLI / andra terminaler: alias och exempel
- Felsökning och vanliga problem
- Säkerhet & notes

---

## Snabbstart (terminal)
Dessa kommandon är snabbaste sättet att testa lokalt.

Starta server:
```/dev/null/command.sh#L1-1
cargo run --features mcp -- mcp serve --addr 127.0.0.1:8080 --token secret
```

Kolla status:
```/dev/null/command.sh#L1-1
gnawtreewriter mcp status --url http://127.0.0.1:8080/ --token secret
```

Lista verktyg:
```/dev/null/command.sh#L1-1
cargo run --features mcp --example mcp_client -- --url http://127.0.0.1:8080/ --token secret list
```

Init (handshake):
```/dev/null/command.sh#L1-1
cargo run --features mcp --example mcp_client -- --url http://127.0.0.1:8080/ --token secret init
```

Analyze (exempel):
```/dev/null/command.sh#L1-1
cargo run --features mcp --example mcp_client -- --token secret analyze examples/example.rs
```

Stoppa server (om du startade i bakgrunden):
```/dev/null/command.sh#L1-1
./scripts/mcp-stop.sh
```

---

## Skript & Makefile (rekommenderat)
För enklare GUI- / editor-integration levererar repot färdiga skript i `scripts/`:

- `scripts/mcp-serve.sh` — starta server (kan köra i foreground eller background, skriver pid & log).
- `scripts/mcp-stop.sh` — stoppa serveren (läser PID-fil).
- `scripts/mcp-client.sh` — wrapper för `list`, `init`, `analyze` m.m.
- `scripts/test-mcp.sh` — orkestrerar start → väntar → kör checks → stoppar.

Du kan också använda Makefile‑målen:
```GnawTreeWriter/Makefile#L1-40
# exempel
make start        # startar server (bakgrund)
make client-list  # kör client list
make test-mcp     # kör test-orchestrator
```

Det gör det mycket lättare att binda editor-knappar eller shell-alias.

---

## VS Code (GUI)
Jag har redan lagt in en `tasks.json` som gör det enkelt att köra server och tester via GUI.

- Fil: `.vscode/tasks.json`
- Så kör du en task:
  1. Öppna Command Palette (Ctrl+Shift+P).
  2. Skriv `Tasks: Run Task` → välj t.ex. `MCP: Start server` eller `MCP: Test (list/init/analyze)`.

Exempel (du kan titta på den faktiska filen i repo):
```GnawTreeWriter/.vscode/tasks.json#L1-40
# Öppna filen i din editor för att se alla tasks (start/stop/test/client).
```

Tips:
- Använd `MCP: Start server (foreground)` vid debugging (se loggen direkt).
- `MCP: Test` kör en komplett runda och stoppar servern automatiskt — perfekt att använda i CI-lokalt.

---

## Zed (terminal + GUI)
Zed saknar (i skrivande stund) en standard tasks-fil som VS Code, så rekommendationen är:

1. Terminal-first (enkel och robust; funkar i alla Zed-installationer)
   - Öppna den integrerade terminalen i Zed och kör:
     ```/dev/null/command.sh#L1-1
     ./scripts/mcp-serve.sh
     ```
   - I en ny terminal-flik: `./scripts/mcp-client.sh list` / `init` / `analyze <file>`
   - När du är klar: `./scripts/mcp-stop.sh`

2. One‑click i GUI (om du vill):
   - Många editors låter dig lägga in egna “Run command” eller “Tasks” via inställningar eller extensions.
   - Skapa en enkel uppgift som kör `./scripts/mcp-serve.sh`. Spara den som en snabbkommandobar eller bind en nyckel (beroende på Zed‑inställningar).
   - Om Zed inte har inbyggd task‑stöd: använd en lätt extension / plugin som kör shell-kommandon eller koppla en snutt som kör `make start`.

Kort: scripts + Makefile + Zed:s terminal ger dig samma UX som i VS Code. Om du vill kan jag skapa ett litet Zed‑snippet (text att klistra in i din Zed‑config) — tala om vilket format du föredrar och vilken Zed-version du kör så skapar jag det.

---

## Gemini CLI & andra terminaler
Gemini CLI (eller vilken shell/CLI du använder) kan enkelt använda skripten:

- Lägg in alias i din `~/.bashrc` eller `~/.profile`:
```/dev/null/alias.sh#L1-3
alias mcp-serve='./scripts/mcp-serve.sh'
alias mcp-test='./scripts/test-mcp.sh'
```

- Eller kör `make`-målen:
```/dev/null/Makefile#L1-5
make start
make test-mcp
```

Det gör kommandona enkla att köra från vilken terminal som helst.

---

## Felsökning & vanliga problem
- `401 Unauthorized` → token mismatch. Kontrollera `--token` och att du skickar `Authorization: Bearer <token>`.
- `connection refused` → servern körs inte på angiven port eller bind misstag. Kontrollera loggfilen (t.ex. `.mcp-server.log` eller den logpath du angav).
- `feature missing / compile errors` → bygg med MCP‑feature: `cargo build --features mcp` eller `cargo run --features mcp -- mcp serve ...`.
- Om du använder `:0` (ephemeral port) — skripten försöker parsa den riktiga porten från serverlogg; om detta inte syns, kontrollera loggen manuellt.
- Om servern är svår att stänga: använd `./scripts/mcp-stop.sh --force` (skickar SIGKILL om nödvändigt).

---

## Säkerhet
- Använd kortlivade tokens för test eller `MCP_TOKEN` i din miljö — EXPL: undvik att commit:a hemliga tokens i repot.
- I CI: använd hemligheter (GitHub Secrets) för token och kör test med temporär token.

---

## Tips & rekommendationer
- För snabb lokal utveckling: `make start` i en terminal, `make client-list` i en annan, och `make stop` när du är klar.
- För GUI‑användare: använd `.vscode/tasks.json` i VS Code; i Zed/andra editors skapa en kort terminal-task som kör samma skript.
- Om du vill kan jag:
  - lägga till en färdig Zed-snippet för att köra `start/stop/test`,
  - lägga till en kort “one-click” plugin-snutt för Gemini CLI,
  - eller lägga PR‑review‑reminder och nominera personer att kolla PR:en.

---

Om du vill jag skapar ett färdigt Zed‑snippet (exakt steg att klistra in i Zed) så tala om vilken Zed‑version/inställningsformat du kör — så genererar jag filen åt dig.
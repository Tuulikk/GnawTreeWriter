# GnawTreeWriter — Zed dev extension (skiss & testinstruktioner)

Detta är en enkel dev‑extension‑skiss för Zed som visar hur du kan exponera GnawTreeWriter som en MCP (Model Context Protocol) context server i Zed. Målet är att du ska kunna starta/stoppa servern från Zed:s Agent‑panel och köra våra exempel direkt från editorn.

Den här README:n beskriver:
- hur du bygger och testar extensionen lokalt,
- hur du startar servern via extensionen eller via skript,
- hur du felsöker om något går fel.

OBS: det här är en dev‑extension‑mall/skelett. Anpassa konfigurationer (addr/token) och versioner av `zed`‑crate efter din lokala Zed‑installation.

---

## Snabböversikt (kort)
1. Bygg extensionen:
```bash
cd examples/zed-extension-gnaw
cargo build --release
```

2. Se till att `gnawtreewriter` är installerat och tillgängligt på PATH:
```bash
# Installera från projektet
cargo install --path .

# Eller använd binären som redan finns i target/release/
cargo build --release
export PATH=$PATH:$(pwd)/target/release
```

3. Installera som dev‑extension i Zed (se Zed‑dokumentationen):
- Se: https://zed.dev/docs/extensions/mcp-extensions
- I dokumentationen finns instruktioner för hur du installerar dev‑extensions i din lokala Zed‑utvecklingsinställning.

3. Starta extensionens context server från Zed (Agent/Context server panel) eller via skript:
- Från Zed: välj din extension / context server och klicka “Start”.
- Alternativt (terminal): `./scripts/mcp-serve.sh` (skriptet hanterar pid + logg).

4. Testa exemplen (i en annan terminal/flik):
```bash
# lista verktyg
./scripts/mcp-client.sh list

# init
./scripts/mcp-client.sh init

# analysera en fil
./scripts/mcp-client.sh analyze examples/example.rs
```

5. Stoppa servern:
```bash
./scripts/mcp-stop.sh
```

---

## Files som ingår i denna extension-skiss
- `extension.toml` — manifest som registrerar context server‑id i extensionen.
- `Cargo.toml` — crate manifest för extensionens Rust‑kod.
- `src/lib.rs` — enkel implementation (skelett) av `context_server_command` som returnerar kommando/args/env.
- Denna README — instruktioner & felsökning.

Tanken är att `context_server_command` ska returnera exakt det kommando (med args/env) som krävs för att starta servern. I vårt exempel försöker vi använda `gnawtreewriter` om den finns i PATH, annars faller vi tillbaka till `./scripts/mcp-serve.sh`.

---

## Hur fungerar `context_server_command` (koncept)
I Rust‑extensionen implementerar du metoden ungefär så (pseudo/Rust‑liknande):

```rust
impl zed::Extension for GnawExtension {
    fn context_server_command(
        &mut self,
        _context_server_id: &zed::ContextServerId,
        _project: &zed::Project,
    ) -> Result<zed::Command> {
        // Extensionen använder gnawtreewriter binären direkt från PATH
        Ok(zed::Command {
            command: "gnawtreewriter".into(),
            args: vec!["mcp".into(), "serve".into(), "--addr".into(), "127.0.0.1:8080".into(), "--token".into(), "secret".into()],
            env: HashMap::new(),
        })
    }
}
```

**Viktigt**: Eftersom extensionen byggts som wasm32 kan den inte söka i PATH via `which`. Därför antar extensionen att `gnawtreewriter` är installerat och tillgängligt på PATH. Script‑fallback har tagits bort eftersom Zed kör extensionen från en annan working directory än projektets rot.

(Notera: anpassa imports och version efter den `zed` crate‑version du använder. Se Zed‑docs för exakta typer.)

---

## Konfiguration och anpassning
- Adresse & token: i exemplet används `127.0.0.1:8080` och token `secret`. Du bör:
  - göra dem konfigurerbara via projektinställningar, eller
  - läsa från miljövariabler (t.ex. `MCP_TOKEN`) så du inte hårdkodar hemligheter i koden.
- **Krav**: `gnawtreewriter` måste vara installerat och tillgängligt på PATH. Installera med:
  ```bash
  cargo install --path .  # från GnawTreeWriter-projektet
  # eller
  export PATH=$PATH:/path/to/gnawtreewriter/target/release
  ```
- Om du behöver ladda ner en binär (t.ex. från GitHub Releases), kan extensionen göra det dynamiskt enligt Zed‑docs.

---

## Testing (dev)
1. Bygg extensionen: `cargo build --release`.
2. Följ Zed‑docs: installera extension som dev‑extension (eller ladda den som en lokal extension enligt Zed GUI/CLI).
3. I Zed: öppna Agent/Context servers → välj din server → klicka "Start".
4. Öppna en terminal i Zed → kör `./scripts/mcp-client.sh list` för att verifiera att servern svarar.
5. Titta i loggen: om du kör i bakgrunden skapas `.mcp-server.log` (eller den loggpath du angav). Om du kör i förgrunden syns loggen i terminalfliken.

---

## Felsökning
- Servern startar inte:
  - Kontrollera att `gnawtreewriter` är på PATH: `which gnawtreewriter`
  - Kontrollera att MCP-feature är aktiverad: `gnawtreewriter mcp serve --help` (ska inte ge "feature not enabled" fel)
  - Testa manuellt: `gnawtreewriter mcp serve --addr 127.0.0.1:8080 --token secret`
- 401 Unauthorized:
  - Kontrollera att klient och server använder samma token (`--token` eller `MCP_TOKEN`).
- Port redan upptagen:
  ```bash
  # Hitta process som använder porten
  lsof -i :8080
  # Döda processen
  kill <PID>
  ```
- Om Zed inte hittar extension: kontrollera att `extension.toml` innehåller korrekta metadata och att du följt Zed:s instruktioner för dev‑extensions.
- "Command not found" eller liknande fel: se till att gnawtreewriter är installerat och på PATH

---

## Säkerhet och CI
- Använd inte produktiva tokens i repot. I CI: använd secrets (GitHub Actions secrets) och skicka token via miljövariabler.
- I vårt repo finns en CI‑job som kör Rust‑exemplet (`mcp-examples.yml`) — PR:en innehåller även steg för att köra detta i pipeline.

---

## Vill du att jag hjälper vidare?
Jag kan:
- generera ett färdigt dev‑extension‑paket (t.ex. ett byggscript) och beskriva exakta steg för hur du installerar det i din Zed‑installation, eller
- skapa ett "one‑click" Zed‑snippet anpassat till en specifik Zed‑plugin om du vet vilken plugin du vill använda.

Säg vilken väg du föredrar så tar jag fram nästa steg åt dig.
GnawTreeWriter MCP-server är registrerad för alla OpenCode-sessioner.

38 GTW-verktyg finns tillgängliga (synkhanteras av kontraktstestet
`integration_mcp_instructions_listed` — varje registrerat verktyg måste
finnas i tabellen här nedan; nya verktyg läggs till här samtidigt):

| Steg | Verktyg | Användning |
|------|---------|------------|
| Orientera | `guide`, `explore`, `summarize`, `analyze`, `get_skeleton`, `list_nodes` | `guide {situation}` → vilket verktyg passar; Kartlägg projekt/fil utan att läsa hela källor |
| Hitta | `sense`, `search_semantic`, `search_nodes`, `investigate` | "Var är X?" utan att veta filnamn — returnerar nod-sökvägar; `search_nodes` stödjer alternation (`"alpha|beta"` = ELLER, svaret bär `matched_pattern`; B4); satellit-svar har `search_quality` (suspect/prior_failures), `expand: true` lägger en LFM2.5-kanal |
| Läsa | `read_node`, `explain`, `validate` | Exakt en nods källkod eller förklaring — ingen fil-dump (trasiga filer svarar delvis med `syntax_warning`); `validate` = strikt syntaxgate före edit |
| Redigera | `edit_node`, `insert_node`, `move_node`, `preview_edit`, `edit_ask`, `diff` | AST-validerad precision — syntax kontrolleras FÖRE skrivning; `edit_node` tar `expect_name`/`expect_kind` (B1: stale path nekas med E_PATH_STALE i stället för fel nod; svaret bär `new_node_path` + `node_content_hash`); `diff` = oberoende verifiering efter skrivning |
| Semantisk redigering | `semantic_edit`, `semantic_insert` | Beskriv VAD ska ändras, GTW hittar noden (svarar med `semantic_match`: nod + confidence + kandidater) |
| Koordinera | `batch`, `undo` | Atomära multi-fil-transaktioner; snabb återställning |
| Skapa & laga | `scaffold`, `heal` | `scaffold` skapar EN ny fil (`content`/`from`/`schema`) — pars-validerad FÖRE skrivning, loggad som Insert (**`undo` kan inte ta bort en skapad fil än**); `heal` reparerar en fil GTW inte kan parsa (obalanserade klammer) — default är **förslag**, `apply: true` skriver först efter att reparationen bevisats parsa, och raderar aldrig kod (extra `}` rapporteras med radnummer) |
| Historik & status | `history`, `stats`, `doctor` | Vad ändrades nyss / hur stort är projektet / är GTW vid liv OCH aktuell (hälsa + build/stale i ett anrop) |
| AI-kontext | `compress`, `pack`, `curate`, `diff_since`, `save_state` | Token-medveten analys, projekt-paketering, ändringsspårning |
| Indexera | `index_project`, `index_entities`, `index_relations`, `code_index` | `index_project` bygger VEKTORINDEXET som satellit-`sense` söker (start/status i bakgrund); de två andra bygger KUNSKAPSGRAFEN; `code_index` är strukturlagret — refresh (inkrementell SQLite-store, parsar bara ändrade filer) / query (items, relations eller symbols via `kind`; relations bär `to_name`+`to_id` enligt T6) / check (content-hash-grind mot committad code-index.md) / coverage (fas 1-kvalitetsmätning av store-grafen: upplösningsgrad per relationstyp + D7-källklasser + unresolved-targetklasser; read-only) |
| Lint & regler | `lint`, `add_rule` | Mönsterlintering (`fix`/`preview` för automatisering); skriv egna regler |
| AI-rapporter | `get_semantic_report` | Kodkvalitetsrapport med källcitat (`sources`) |

## Diagnostisk kedja (standard-arbetsflöde)

1. **Förstå strukturen**: `analyze`/`get_skeleton` → se filens AST och nod-sökvägar
2. **Hitta målet**: `sense`/`search_nodes` → hitta exakt nod att ändra
3. **Läs målet**: `read_node` → verifiera innehåll före ändring
4. **Ändra**: `edit_node`/`semantic_edit`/`insert_node` → förhandsgranska med
   `preview_edit` vid riskfyllda ändringar
5. **Verifiera**: kör bygg/test (`cargo check`), diffa vid behov; vid fel → `undo`

Saknas satellit-indexet? `index_project {"action":"start"}` (polla med
`"status"`) — meddelandet från tomt `sense` pekar på samma.

## Varför AST-nivå istället för textnivå

- Nod-sökvägar (t.ex. `35.2.105`) är stabila — radnummer förskjuts vid varje redigering
- Syntax valideras före skrivning: en trasig ersättning avvisas istället för att
  tyst korrumpera filen
- Målet kan inte missmatcha: `edit_node` byter innehåll på EN namngiven nod,
  aldrig "första träffen" av en sträng

Alla verktyg har full `inputSchema` + självförklarande beskrivning
(VAD/NÄR/RETURNERAR/EXEMPEL) — fråga `tools/list` för detaljer.

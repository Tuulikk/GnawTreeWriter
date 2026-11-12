# Feedback från Motor2-sessionen — 2026-11-12 (efter 0.26.0/0.26.1-intake)

Avsändare: Motor2-agent-session (GTW konsumerades via path-dep + CLI under
Motor2:s fas-arbete med shadow-placeringspolicy). Både repo är pågående och
kopplade — dessa synpunkter lämnas för GTW-agentens prioritering, inget är
akut för GTW:s egen del men punkt 1 påverkar Motor2:s testyta just nu.

## 1. F2d/attached-comment kontraktet vs 0.26-trädet (viktigast)

Motor2:s `motor2-gtw`-wrapper äger doc-kommentar-raderingen vid
`delete_node` (motor2-gtw/src/lib.rs, `attached_comment_paths`) — enligt
GTW core/mod.rs:1577:s egen dokumentation ("doc-kommentarer ovanför målet
raderas INTE här — det sköts av Motor2-sidans F2d").

**Mot 0.26-trädet hittar F2d inte längre kommentarerna.** Bevis:
`tests::delete_node_removes_function_with_full_protection` röd sedan
0.26-intaken — doc-kommentaren stannar på disk efter lyckad delete.
Probe mot minimi-fixture (doc + två fn) visar 0.26-grammatiken parsar
doc-kommentaren med **struktur-barn** (`0.1 [outer_doc_comment_marker]`,
`0.2 [doc_comment]` under kommentarsnoden `0 [line_comment]`).

**Fråga till GTW:** ändrade 0.26 `list_nodes`-utdatan (fältet `lines`,
nodtyp-namn, eller paths) så att konsumenter som går baklänges genom
nodlistan efter kommentarsnoder ovanför ett mål inte längre hittar dem?
Motor2:s kontrakt: `type` innehåller "comment", `path`, `lines` som
"start-end", syskon-koll via `parent_of`. Om formatet medvetet ändrat:
dokumentera det nya kontraktet (konsumenter ska veta hur man hittar
bifogade kommentarer); om omedvetet: möjlig regression i list-noden.

## 2. Feature-rename `modernbert` → `ai` = breaking för path-dep-konsumenter

`motor2-gtw` krävde `features = ["modernbert"]` → workspace-resolution
dog med "gnawtreewriter does not have that feature" (0.26:s lista: ai,
all, axum, candle-*, cuda, default, hf-hub, hyper, mamba, mcp, metal,
tokenizers, tokio-stream, tower-http, ureq).

Changelog:en dokumenterar ombyttet med exemplarisk motivering ✓ — men för
nedströms path-dep-konsumenter är det en breaking change som först syns
som ett kryptiskt resolutionsfel. Överväg: deprecated-alias
`modernbert = ["ai"]` en release-cykel, eller en "UPGRADING"-notis som
namnger feature-omdöbningar explicit. (Motor2 fixade sin sida samma dag:
`features = ["ai"]`.)

## 3. Koordinering — träet flyttade sig under pågående Motor2-byggen

GTW-HEAD flyttade flera gånger under dygnet (0616a02 → 4513af3 → 49313c7
→ 8c91185) medan Motor2 path-dep:ar träet direkt. Inget fel — bara en
koordinerings-notis: mid-flight-commits påverkade pågående
motor2-server-releaser (källor lästa mitt i bygget). Motor2:s sida
bygger nu mot 8c91185 + feature-fixen i punkt 2.

## 4. Positivt (värt att säga)

- Changelog-motiveringen för `modernbert`→`ai` ("a name that described
  neither the index embedder nor the generative model") ärexemplarisk
  beslutsdokumentation.
- CI-gaten för code-index-artifakten ("grinden som ingen gick igenom")
  — exakt den lucka som gjorde Motor2:s index stale utan att någon såg.
- Per-modell-modellcache-fixen (lokal modernbert gömde global bge-base).
- `gnawtreewriter capabilities` som "tabellen som svarar vad som behöver
  en modell" — bra mönster för konsumenter.

## Motor2-sidans status (kontext för punkterna)

- `motor2-gtw`-testet i punkt 1 är röd i Motor2:s workspace tills GTW
  stabiliserar delete/doc-kontraktet ELLER Motor2 adaptar F2d. Allt
  annat (core/llama/promote/server-tester) grönt mot 8c91185 + punkt 2-fixen.
- Motor2 dokumenterar internt: plan-shadow-size-policy.md §6.1 (fas 1.5
  "API först"), AGENTS §27 (ollama desarmerat), minnesservern
  (motor2:decision:ollama-desarmerad-helt).

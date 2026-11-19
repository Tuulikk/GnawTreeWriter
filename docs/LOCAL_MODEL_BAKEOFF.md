# Lokal modell-bake-off: LFM2.5 vs LFM3 vs Qwen3.5

**Status:** plan (§1–7, 2026-11-11) + **baslinjen mätt (§8, 2026-11-19)**.
Återstår: kandidatmätningen. **Datum:** 2026-11-11, uppdaterad 2026-11-19.
**Frågan den ska besvara:** vilken lokal modell ska GTW:s L4 (explain / summarize /
investigate / propose_edit / query expansion) använda — och väger den mindre än
det vi kör i dag utan att bli sämre?

---

## 1. Läget, mätt (inte antaget)

Nuvarande modell, läst ur GGUF-headern:

```
general.architecture  = lfm2          general.file_type   = 15 (Q4_K_M)
tensor-typer          = Q4_K ×82, Q6_K ×11, F32 ×55
parametrar            = 1.17 B        filstorlek          = 697 MB
context_length        = 128000        block_count         = 16
bytes/param           = 0.62
```

Baslinje på den här maskinen (release-binär, CPU):

```
$ gnawtreewriter explain src/core/age.rs
tokens: 1203 in / 90 out · calls: 1
time:   96 s   (~1 tok/s decode, inkl. 697 MB modelladdning)
```

**0,62 bytes/param är vår egen mätta kvot** (från filen ovan). Den användes först
för att *uppskatta* kandidaterna — och **underskattade med 2–4 %** (F32-tensorer
och metadata väger). Tabellen i §2 använder därför utgivarnas faktiska filstorlek,
inte estimat.

### Blockeraren: motorn, inte modellen

Candle 0.9:s modellkatalog innehåller `llama`, `phi`, `phi3`, `quantized_lfm2` —
och **inget qwen**. Alltså:

| Kandidat | Candle 0.9 | Cera |
|---|---|---|
| LFM2.5 (i dag) | ✅ `quantized_lfm2` | ✅ `lfm2` |
| LFM3 | ❌ ingen modul | ✅ `lfm3` |
| Qwen3.5 | ❌ ingen modul | ✅ `qwen35` |

**Slutsats: motorbytet är förutsättningen för hela jämförelsen.** Det spelas in
en gång och gäller alla tre — därför en bake-off, inte tre spikar.

---

## 2. Kandidater (exakta storlekar)

Källa: utgivarnas fil-listningar (Hugging Face tree API, 2026-11-11). Alla
Q4_K_M, så jämförelsen är samma kvant.

| Modell | Q4_K_M | mot i dag | Param | Kontext |
|---|---|---|---|---|
| LFM2.5-1.2B (i dag) | **697 MB** (mätt lokalt) | — | 1,17 B | 128k |
| **Qwen3.5-0.8B** | **508 MB** | **−189 MB (−27 %)** | 0,8 B | 256k |
| LFM3-1.7B | **1 049 MB** | +352 MB (+51 %) | 1,7 B | — |

Qwen3.5-0.8B är den enda kandidaten som är **mindre** än i dag — 27 % mindre
vid samma kvant. Det gör den till förstavalet att mäta, inte till förstavalet
att behålla; kvaliteten avgör, och 0,8 B är 30 % mindre än 1,2 B.

Övriga kvantar för Qwen3.5-0.8B, om 508 MB visar sig för stort eller för litet:
`Q3_K_M` 448 MB · `Q5_K_M` 563 MB · `UD-Q4_K_XL` 533 MB.


---

## 3. Multimodal: välj filen, inte flaggan

**Principen (2026-11-11): text-only-GGUF slår att inaktivera bitar vid körning.**
En runtime-flagga lämnar vikterna i filen, i minnet och i lasttiden; ett
text-only-GGUF tar bort dem ur ekvationen helt. Flaggor är en *nödlösning* för
den som redan sitter med en multimodal fil — inte en design.

### Verifierat för Qwen3.5-0.8B: visionen ligger i separata filer

```
Qwen3.5-0.8B-Q4_K_M.gguf   508 MB     ← text, det här är hela modellen vi vill ha
mmproj-F16.gguf            195 MB     ← synprojektor, hämtas ALDRIG
mmproj-BF16.gguf           198 MB
mmproj-F32.gguf            384 MB
```

Kvantfilerna innehåller alltså **ingen** synprojektor. Text-only är inte en
kompromiss för den här modellen — det är att välja rätt filer. `architecture =
qwen35`, `context_length = 262144`.

### Nivåerna, i prioritetsordning

| Prio | Åtgärd | Vad den tar bort |
|---|---|---|
| **1 (rätt väg)** | **ladda aldrig `mmproj`** | 195–384 MB disk, och synvikterna existerar inte i processen |
| 2 (om filen är multimodal) | Cera med `default-features = false`, utan `vl-preprocess` | `dep:image` (bildavkodning) ur beroendeträdet |
| 3 (nödlösning) | `CERA_VIT_WARMUP=off`, `CERA_VIT_STREAM=off` | ~190 ms blank-bildsuppvärmning; fp16-kärnor i vision-tornet |

Cera:s `default`-features är `["parallel", "mmap", "std-fs", "disk-cache",
"vl-preprocess", "avx512"]` — **`vl-preprocess` är på som default**, så ett
slarvigt `cargo add cera` drar in bildavkodning vi aldrig använder. Vår
integrering ska ange features explicit (se §5).

**Varning:** siffrorna i rad 3 (190 ms, kärnval) är **Ceras egna mätningar på
deras hårdvara** och ska mätas om här innan de citeras som fakta om vår maskin.
Rad 1 kräver ingen mätning alls — den är aritmetik.

### Tänk också på: `enable_thinking`

Qwen3.5:s chat-template lägger in ` thinking\n\n</think>\n\n` när thinking är
avstängt. Utdata kan alltså bära tomma tanketaggar som måste strippas, precis
som LFM3:s ` thinking`-block måste hanteras. Det gäller båda kandidaterna och ska
in i mätningen, inte upptäckas i produktion.


---

## 4. Vad som mäts

Fyra mått, alla på **samma sex prompts** och samma maskin:

| Mått | Metod | Varför |
|---|---|---|
| **Giltiga förslag** | antal `propose_edit` där resultatet **parsar** (`validate` / AST) | objektivt, och den enda uppgiften där fel är dyrt |
| **Wall time** | `date +%s%3N` runt hela kommandot (inkl. modelladdning) | baslinje 96 s |
| **Tokens** | modellens egen budgetrad (`in / out / calls`) | normaliserar mot utlängd |
| **Svenska** | din bedömning 1–5 per uppgift, samma prompts | språk är smaksak, inte ett tal jag kan fabricera |

### De sex uppgifterna (från `pipeline.rs`)

1. `expand_query_terms` — driver `sense --expand`
2. `explain_node` — `explain`
3. `summarize_dir` — `summarize`, **körs både på engelska och svenska**
4. `investigate` — `investigate`
5. `propose_edit` — enda uppgiften med hårt facit (parsar / parsar inte)
6. `propose_edit_all` — samma facit, flera noder

### Abortvillkor (bestäms före mätningen, inte efter)

En kandidat går vidare **endast om**:

- **giltiga `propose_edit` ≥ baslinjen**, och
- **wall time ≤ baslinjen** (96 s), och
- svenskbedömningen inte faller under baslinjens.

Väg inte ihop det till ett tal: ett snabbt svar som inte parsar är värdelöst,
och ett långsamt svar som parsar är användbart i bakgrunden.

---

## 5. Så körs det

### Steg 0 — baslinjen (redan gjord, upprepa vid behov)

```bash
cargo build --release --features ai,mcp,mamba      # bara fastrand tillkommer
time gnawtreewriter explain src/core/age.rs        # → 96 s, 1203 in / 90 out
```

### Steg 1 — Cera bakom fasaden (enda kodändringen)

`generate_lfm25` i `ai_manager.rs` är den yta `pipeline.rs` använder (sex
anrop). Cera kopplas in **bakom den**; `pipeline.rs` rörs inte. Det gör att
varje kandidat kan bytas genom att peka på en annan katalog.

```toml
# Cargo.toml — features explicita: vl-preprocess (bildavkodning) ska INTE med
cera = { version = "0.7", default-features = false, features = ["parallel", "mmap", "std-fs"] }
```

### Steg 2 — modeller på disk (text-only)

| Kandidat | Katalog | Källa |
|---|---|---|
| Qwen3.5-0.8B | `.gnawtreewriter_ai/models/qwen35/` | `unsloth/Qwen3.5-0.8B-GGUF`, text-only-kvant, **utan** `mmproj` |
| LFM3-1.7B | `.gnawtreewriter_ai/models/lfm3/` | `LiquidAI/LFM3-1.7B-GGUF` → `Q4_K_M` |

Filnamnet ska vara `model.gguf` (+ `tokenizer.json`) — samma konvention som
`lfm25` har i dag, så lastaren och `capabilities` fungerar utan specialfall.

### Steg 3 — mät och fyll i

Skriv resultatet **i den här filen** (ny tabell per kandidat), inklusive
misslyckanden. En kandidat som föll på abortvillkoren är ett resultat, inte en
förlust.

---

## 6. Vad som INTE ingår

- **Embeddings byts inte.** `bge-base` är safetensors, inte GGUF, och stannar på
  Candle. Cera tar bara generering → **två motorer** tills vidare.
- **Candle tas inte bort** i det här steget.
- **Prompts skrivs inte om** förrän en modell är vald. Undantaget är
  SV-varianten av `summarize`, som är en del av mätningen.
- **L4 får aldrig mata indexet.** `capabilities` säger det: L4 producerar text
  som kan vara fel. Inget i den här planen ändrar den gränsen.

---

## 7. Risker och öppna frågor

| Risk | Bedömning |
|---|---|
| Cera är ung: 0.7.4, ~1 900 nedladdningar, en organisation, API-guiden heter `API_0_6.md` | Reell. Därför spike, inte migration — och bakom en fasad |
| MSRV **1.94** | Bygger inte på äldre verktygskedjor; påverkar path-dependents (Motor2) |
| Två motorer = två fellägen | Accepterat i detta steg; omprövas om Cera bevisar sig |
| LFM3:s ` thinking`-block | Måste stängas av eller strippas, annars får sammanfattningar resonemangsspår |
| Qwen3.5-0.8B är 30 % mindre än i dag | Svagare instruktionsföljsamhet; GBNF-grammatik kan kompensera för `propose_edit` |
| Våra prompts är engelska | Svensk utdata kräver explicit instruktion **och** en modell som kan svenska |
| GPU/NPU-vägar i Cera | Irrelevanta här; vi mäter CPU, och CPU är produkten |

### Öppen fråga vi inte kan svara på förrän mätningen

Om LFM3-1.7B visar sig **snabbare** än LFM2.5-1.2B på vår maskin (Cera:s
trådkalibrering kan ge det), då var storleksargumentet aldrig giltigt — och då
är kvalitetsvinsten gratis. Det är den enda frågan som kan vända hela beslutet,
och den kostar en kväll att mäta.

---

## 8. Baslinjen mätt (2026-11-19) — och två premisser rättade

Allt i §1–7 skrevs 2026-11-11. Detta avsnitt är mätt 2026-11-19 på samma maskin
och **ersätter siffror där de krockar**.

### 8.1 Baslinjen, replikerad

Release-binär, `cargo build --release --features ai,mcp,mamba`, CPU,
LFM2.5-1.2B QAD-Q4_0 (730 895 168 B GGUF, installerad 25 aug):

| Körning | Genereringar | Wall | Tokens (in/out) |
|---|---|---|---|
| `explain src/core/age.rs` | 1 | **81 s** | 1203 / 86 |
| `summarize src/llm` (15 filer) | **18** | **297 s** | 4115 / 459 |
| `summarize` på en 1-fils-katalog | 2 | **19 s** | 152 / 100 |

`ai calibrate` (persisterad `TimingProfile`):

```
prefill: 0.103 s/token
decode:  0.075 s/token      ← 13 tok/s
overhead: 0.5 s/call
```

**§4:s baslinje 96 s står alltså** (81 s idag för samma uppgift). Decode är den
dominerande kostnaden: 13 tok/s på CPU. Det är den siffra §4:s abortvillkor ska
jämföras mot.

### 8.2 Rättelse: candle 0.9.2 HAR qwen — men inte `qwen35`

§1 säger *"Candle 0.9:s modellkatalog innehåller llama, phi, phi3, quantized_lfm2
— och inget qwen"*. **Falsifierat** mot låst version `candle-transformers 0.9.2`:

```
quantized_qwen2.rs  quantized_qwen3.rs  quantized_qwen3_moe.rs
qwen3_moe.rs        qwen2_moe.rs        mixtral.rs  qwen3_vl/
```

**Slutsatsen i §1 står dock**: det finns ingen `qwen35`-modul, och Qwen3.5:s
Gated DeltaNet-lager (3:1 hybrid, `S_t = β_t ⊙ S_{t-1} + Δ_t ⊗ (K_t ⊗ V_t)`)
kräver kärnor candle saknar. Motorbytet behövs — av rätt anledning.

`cera 0.7.4` (MSRV 1.94; vi kör 1.98) dispatchar på `general.architecture` och
listar: `lfm2`, `lfm2moe`, `lfm3`, `qwen2`, `qwen3`, **`qwen35`**, `llama`,
`granite`, `minicpm`, `phi3`/`phi`, `mistral3`, `bailingmoe3`/`bailingmoe`, plus
Mamba-2 och hybrider på CPU-lastaren.

### 8.3 Nya fakta: AMD-vägen och livscykeln

**AMD.** Maskinen har två GPU:er i samma Vulkan-instans:

```
GPU0  AMD Radeon RX 580 (RADV POLARIS10, Mesa 26.2.4)   4.0 GB VRAM
GPU1  NVIDIA GeForce RTX 4070 SUPER                     12 GB
```

`candle-core 0.9.2` har `Device { Cpu, Cuda, Metal }` — **ingen ROCm/HIP**. GTW:s
`DeviceType` är lika smal. Cera:s `gpu`-feature är **wgpu**, som talar Vulkan, och
RX 580 rapporterar Vulkan 1.4 → **Cera är den enda observerade AMD-vägen**, och
det argumentet fanns inte i §1–7.

**Livscykeln (A vs B, utrett).** `AiManager::new()` anropas **per MCP-anrop** (6
ställen i `mcp/mod.rs`); ingen instans cachas. `cached_lfm25: OnceLock` är alltså
per anrop — men den bär hela sin vikt **inom** ett anrop: `investigate` gör 4
genereringar, `summarize src/llm` gjorde **18**. Att äga `AiManager` i `AppState`
(B) skulle spara ~0,5 s per anrop (`overhead: 0.5 s/call`, och 1-fils-provet tog
19 s för 2 anrop) mot 81–297 s körning = **~0,6 %**. Slutsats: **B är inte
motiverad av laddningstid**; `OnceLock` ska stå orörd. Det som är dyrt är decode.

**Installationsgapet (M14-klass).** `commit-build.sh` installerade
`--features ai,mcp` och `build-gpu.sh` byggde `--features cuda` — **båda utan
`mamba`**. Följden mättes live: `gnawtreewriter explain …` i den installerade
binären svarar *"LFM2.5 commands require the 'mamba' feature"*, och MCP-verktyget
`explain` svarar samma. Hela L4 var alltså byggt, testat, installerat och
**oåtkomligt** i den binär agenterna kör. Åtgärdat 2026-11-19 i båda skripten.

### 8.4 Nästa mätpunkt (oförändrad fråga, nu med tal)

Qwen3.5-0.8B, text-only Q4 (508 MB, **−27 %** mot i dag), via Cera bakom
`generate_lfm25`. Frågan är om 0,8B-GDN slår 13 tok/s på den här CPU:n — och
samtidigt öppnar wgpu-vägen RX 580. Abortvillkoren i §4 gäller oförändrat, med
**81 s** som baslinje för `explain`.




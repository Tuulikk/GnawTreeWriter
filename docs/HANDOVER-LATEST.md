# Handover — 2026-11-20 (XI): stale-binären synlig, L4 uppkopplad, L2-spåret byggt

**Läget:** träd rent, `HEAD == origin/master` (`e657628`), **17 commits** sedan
föregående handover (`3e3b048`). 394 lib- + 26 mcp_integration-tester gröna,
clippy `-D warnings` ren, `code-index --check` + `pack-skill --check` gröna,
`doctor` 29/29. **45 MCP-verktyg** (var 42). Installerad binär matchar källan
(`build e657628`, byggd 2026-11-20 12:55). Ingen release taggad (0.26.1 ute).

**Läs först om du bara läser en rad:** `prose_annotations` — claim-lagret — var
**byggt, komplett och tomt** i flera dagar (0 rader, 0 anropare). Det är nu
uppkopplat och bevisat. Arbetet var att koppla in en sladd i ett uttag som redan
satt där, inte att bygga ett lager.

Läsordning: denna fil → `docs/L2_MODEL_ASSISTED_CLASSIFICATION.md` (spåret,
nu med mätta kostnader) → `docs/HISTORY_STORAGE_DESIGN.md` §8 (ägarbesluten) →
`docs/LOCAL_MODEL_BAKEOFF.md` §8 (baslinjen) → `uppslag.md`.

---

## 1. Sessionen i fyra vågor

### a) Stale-binären — tre lager (ägarens spår från gnawdriver)

Mätt utgångspunkt: installerad 0.26.1-binär predaterade `137c1d4`, så varje
`edit_node` via MCP träffade gammal kod **utan att något svar sa det**.

| Commit | Vad |
|---|---|
| `465e53d` | `_build_warning` på **varje** verktygssvar + byggidentitet i `initialize` |
| `5bd42ef` | Nekar **historiska** skrivningar (`undo`, `code_index` refresh/check-fix) från stale binär — edits är återställbara, historikpåståenden är det inte |
| `99ab90f` | `doctor` och `initialize` svarar **samma** på "kör jag den här källkoden?" — de gjorde inte det |
| `1f2d470` | Byggets förmågor som data: `cpu (index) · cpu (L4 generation)` + namngiven lucka när L4 saknas |

**Mönstret som gäller:** tyst när färsk, högljudd när stale. Fältet är
**frånvarande**, aldrig `false` (ett larm som alltid lyser lär folk ignorera det).

**Ägarkorrigering under arbetet:** jag kallade `build-gpu.sh` utan `mamba` för
"extra illa". Det var fel — splitten är **medveten design** (bge-base vinner på
GPU för indexering; L4 är långsam i vektorrum). Återställt, och i stället gjordes
splitten **synlig** i binären. `commit-build.sh` behåller `mamba`
(CPU-binären ska vara full).

### b) Deploy-gapet (M14-klass, mätt live)

`commit-build.sh` installerade `--features ai,mcp` **utan** `mamba` → hela L4
(`explain`/`summarize`/`investigate`/`edit_ask`) svarade *"requires the 'mamba'
feature"* i den binär agenterna kör. Byggt, testat, installerat, **oåtkomligt**.

Åtgärdat i `commit-build.sh`; bevisat på båda ytorna (CLI + MCP).

### c) L2-spåret — spec, sedan byggt i tre steg

Utgångsfrågan (ägarens ord): *"använda mindre modell än bge-base för delar av
indexering och hanteringsjobbet … modellerna kan inte läsa av varandras arbete,
så det skulle handla om applicering som synliga, bestående delar i Code Index."*

**Mätt före något byggdes** (`docs/L2_MODEL_ASSISTED_CLASSIFICATION.md`):

| Fynd | Värde |
|---|---|
| `prose_annotations` rader | **0** |
| anropare av `upsert_prose` | **0** |
| `unresolved_plain_name` | 19 758 av 28 088 relationer (**70 %**) — kräver typinferens, inte språkmodell |
| `calls` lokalt upplösta | 43 % |

Specen svarar alltså **nej** på den vanliga förhoppningen: en mindre modell höjer
inte `resolution_rate`. Den ska **märka, förklara och rangordna** — inte lösa.

**Byggt:**

| Commit | Vad |
|---|---|
| `46abeb3` | `describe` (producent) + `claims` (läsare), ankare ur storens `files.hash` |
| `755f6e7` | MCP-ytorna + `claim_coverage` med **egen nämnare** |
| `40273b3` | `describe_queue` — deterministisk rangordning (Nivå 3) |
| `e657628` | `describe --detail skeleton\|source` — ett **mätt byte** |

**Livscykeln bevisad live** (hermetiskt temp-repo):

```
describe age.rs         → claim lagrat, ankrat till files.hash
innehållsändring+refresh→ 1 stale   (reason: content_changed)
filradering+refresh     → 1 invalid (reason: anchor_removed)
bara touch              → förblir fresh  (mtime är inte innehåll)
```

**Rangordningen mätt på detta repo:**

```
110 kandidater av 111 filer, 1 täckt
  1. src/core/gnaw_find.rs  [in:183 out:208 ent:26]
  2. src/core/mod.rs        [in:149 out:1197 ent:153]
  3. src/parser/mod.rs      [in:110 out:268 ent:67]
```

Ordningen är **lexikografisk över deklarerade prioriteter**, inte en viktad
summa: (1) stale claim först, (2) in-kanter, (3) ut-kanter, (4) entiteter,
(5) produktion före test. Inget multipliceras, varje nivå syns i `why`.

### d) Kostnadsmätningen — varför 90 s, och vad det betyder

Ägaren: *"Upp till 90 s per fil är dock rätt saftigt. Vad är orsaken?"*

| Lager | Kostnad | Varför |
|---|---|---|
| **Decode** | ~0,075 s/token | **minnesbandbredd**: 697 MB läses per genererad token |
| **Prefill** | ~0,061 s/token | ~O(seq²) i candle — promptlängden är kvadratiskt dyr |
| **Modelladdning** | ~6 s/anrop | per **process**; `OnceLock` räddar den bara *inom* ett anrop |

**Bevis för den sista:** två anrop i två processer tog 18 s och 11 s — båda
betalade laddningen. Men *inom* ett anrop fungerar cachen: `summarize src/llm`
gjorde **18 genereringar** och betalade en gång.

**`detail` är ett byte, inte en gratis vinst** (samma 37 KB-fil, samma modell):

| mode | prompt | tid | claimen |
|---|---|---|---|
| `skeleton` (default) | 593 t | **43 s** | en rad: vad modulen hanterar |
| `source` | 1190 t | **86 s** | namnger typerna, strategierna, flödet |

Halva tiden ger halva innehållet. `skeleton` är default för **täckningsbredd**.
Siffrorna står i koden och i specen så valet görs på belägg.

---

## 2. Fyra fel som testerna/vakterna fångade (det är vad de är till för)

1. **`unwrap_or(0)` i rank-uppslagningen** befordrade varje fil *utan* signal
   till **först**. Frånvaro måste ranka sist. (`in_edges_outrank_out_edges`)
2. **Summan av normaliserade ranker var symmetrisk** — hubb och anropare fick
   samma poäng, beroenderiktningen försvann. Bytte till lexikografisk prioritet.
3. **`refresh` bailade när sista källfilen raderades** → `files` behöll spöken →
   ett claim kunde bli `stale` men **aldrig `invalid`**. Hittat av
   livscykeltestet, fixat (`0 filer · 1 borttagna` bevisar det).
4. **Verktygsräknaren i `GTW_INSTRUCTIONS.md` hade driftat** (40 vs 44) eftersom
   kontraktstestet bara kollade **närvaro**, aldrig **antal**. Nu pinnar det
   antalet — bevisat i båda riktningarna (med `999` faller det med ett meddelande
   som säger exakt vad som förväntas).

Plus tre processfel av mig själv: backticks i en commit-meddelande-sträng (bash
körde dem), `sed -i` på projektfil, och `sqlite3` i skalet. Alla fångade av
vakterna, alla rättade med rätt verktyg.

---

## 3. Process — ägarens två korrigeringar

**a) "Jag blandar ihop 2 olika saker"** — jag svarade om GTW:s **inbyggda**
modell när frågan gällde **agenter som använder GTW**. Två spår, och jag höll dem
inte åtskilda. Sedan: *"Candle vet jag. ska redan finnas studier om andra
motorer"* — och det fanns: `docs/LOCAL_MODEL_BAKEOFF.md`, som jag borde läst
först.

**b) "svaret behöver ligga i GTWs anda och utstakning, så att det inte blir
föräderi"** — min spec §9 ställde fyra frågor som **ROADMAP.md redan besvarat**:

> *"Prose enrichment — optional pass, **never a gate** … Staleness surfaces, it
> does not punish … reported, never blocking."*

Och en princip jag missat: `CODE_INDEX_SCAN_TOOLS_PROPOSAL` §2.3 —
*"Determinism … stor och liten modell ser SAMMA fakta."* Repot hade redan
bestämt att modellstorlek **inte får ändra vad som ses**. Skrivet i §9.0–9.3.

**Läxan:** en spec som ber ägaren välja mellan alternativ utstakningen redan
avgjort är inte neutral — den flyttar ansvar till någon utan underlaget. **Läs
utstakningen först, fråga bara om det den inte täcker.**

---

## 4. Mätt läge (2026-11-20)

```
files                          203
relations_total                28 554
  calls        3341/25848      12,9 % upplösta
  uses          254/2127       11,9 %
  imports         43/492        8,7 %   ← sämst
  implements      35/87        40,2 %
prose          fresh 1 · stale 0 · invalid 0
  files_with_fresh_claim        1 / 203
  state            populated
MCP-verktyg                     45
L4-baslinje (LFM2.5, CPU)       explain 81 s · summarize(src/llm) 297 s/18 anrop
  calibrate       prefill 0,061–0,103 s/token · decode 0,075 · 0,5 s/call
```

---

## 5. Kvar (ärligt, inget tyst)

| Post | Läge |
|---|---|
| **Täckning 1/203** | maskineriet fungerar; en körning är drifttid, inte kod. 110 filer × ~43 s ≈ 80 min |
| **Daemonen kör äldre kod** | `/reload` krävs efter varje `commit-build` — den här sessionens gjordes, 45 verktyg bekräftade |
| **42 `(deleted)`-daemons** | kvar sedan dagar; `daemon-hygiene.sh reap` **mellan sessioner** |
| **Daemon-RSS 712 MB** | efter `describe`; modellen släpps men allocatorn ger inte tillbaka till OS — känt, ej åtgärdat |
| **`mamba`-featuren** | gatar LFM2.5, inte Mamba. Namnet ljuger; 70 `#[cfg]`-ställen. Egen commit |
| **Nivå 2 (klassificering)** | kräver facit innan den byggs; rör den enda kant repot aktivt skyddar |
| **`opencode.jsonc` i repot** | innehåller `$HOME`-sökväg; trasig för andra användare. Pekar på install-output |
| **Historik §8 (temporal store)** | målbild beslutad (`2ef1d77`), **ingen kod** — steg 1 är hash-unifieringen |
| **Bake-off (§8.4)** | Qwen3.5-0.8B via Cera är nästa mätpunkt; 81 s är baslinjen |
| **AMD/RX 580** | candle har ingen ROCm; Cera:s `gpu` = wgpu/Vulkan är enda observerade vägen |

---

## 6. Verifieringskommandon (M11: kör, lita inte på minnet)

```bash
git status --porcelain && git log --oneline -3
cargo test --lib 2>&1 | grep "test result"          # 394 grön
cargo test --test mcp_integration 2>&1 | grep "test result"   # 26 grön
gnawtreewriter doctor 2>&1 | tail -2                # 29/29
gnawtreewriter claims                               # 1 fresh, state populated
gnawtreewriter describe-queue --limit 5              # rangordningen
gnawtreewriter code-index . --coverage               # prose + graf, med nämnare
gnawtreewriter ai status | tail -2                  # device per lager
bash scripts/claim-scan-gate.sh .                   # 0 HIGH
gnawtreewriter code-index . --check                 # grön
scripts/daemon-hygiene.sh                           # 42 stale, sedan reap
```

**L4 kräver `--features ai,mcp,mamba`** — installerade binären har det (fixat
2026-11-20). Utan det svarar varje L4-verktyg "requires the 'mamba' feature".

---

*Skapad 2026-11-20 av sessionens agent (DeepSeek V4.1 Flash); ägare: Tuulikk.
Allt ovan är verifierat med kommandon denna session — inga påståenden utan
facit (M1–M14). Ägarens anmärkning: handovern borde uppdaterats **under**
arbetets gång, inte bara i slutet — sessioner kan fallera.*

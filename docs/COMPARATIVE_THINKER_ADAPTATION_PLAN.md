# Adaptationsplan — Comparative-Thinker → GnawTreeWriter

**Status:** Förslag (design, ej byggd) · **Datum:** 2026-11-10 · **Rev II samma dag:** granskad av agent — GTW-ankare spot-checkade 6/6; fyra fixar (§4.3-begränsning, symbol-ankare, P1-funktionsgate, kuvert-notis)
**Källa:** `/mnt/secret-stash2/Drive/dev/Gnaw-Software/Comparative-Thinker`
(1 squash-commit `9a37133` 2025-12-05, 30 experiment, vilande)
**Källans karaktär:** experimentellt projekt —
30 experiment där varje resultat har formats vidare till nästa idé, byggt
interaktivt tillsammans med flera agenter och modeller. Koden är därför
**stegvis agentproducerad och aldrig granskad som helhet**; planen behandlar
den som förslag, inte som facit.
**Klassificering:** förlängningsspår — inte buggar, inte skuldsatta items.
Varje inkrement är additivt och kan avbrytas utan halvt state.
**Grundregel:** **Låna frågan. Verifiera koden. Skriv GTW-versionen själv.**

---

## 0. TL;DR

| # | Kandidat | CT-källa (verifierad) | GTW-lucka (verifierad) | Insats | Dom |
|---|---|---|---|---|---|
| **P1** | `harmony` / `dissonance` i `sense` | `09.1_dissonance_detection/src/lib.rs:18-45` | tre kanaler finns i RRF — ingen enighetsmätning | S | **Gör** |
| **P2** | `cycles` / `impact(depth)` / `chain` över relations-tabellen | `07.5_advanced_dependency_queries/src/lib.rs:112-215,355-388` | noll cykeldetektion i `src/`; `blast`/`graph` saknas i MCP | S–M | **Gör** |
| **P3** | `complexity` på riktig AST | `09.4.2_ast_analysis/src/lib.rs:52-63` (idén; koden är attrapp) | bara nyckelords-proxy i `guardian.rs:157` | M | Gör |
| **P4** | JSON-analysartefakt + triage-skript | `10.1`, `10.2`, `scripts/create_issues_from_report.py` | `report.rs` = markdown ur ALF/txn, ingen JSON/CI-konsument | S–M | Gör (smalt) |
| — | `filter` (required/preferred) i `sense` | `03_graph_integration/src/lib.rs:120-160` | ingen filteryta i `sense` | S | Vid behov |
| — | strukturell abstraktion / cross-domain-analogier | `11.0`, `11.1` | — | L | **Nej** |
| — | dynamisk pelarviktning per fråga | `09.2_dynamic_weighting` | — | S | **Nej** (bryter determinism) |

Fyra inkrement, i den ordningen. P1 är det enda som är "uppenbart rätt":
allt som behövs finns redan i GTW — det som saknas är variansen mellan
kanalerna och en mening som förklarar den.

---

## 1. Princip: verifiera först, skriv själv

Källprojektet är experimentellt: 30 isolerade experiment, Python som
orkestrerar Rust/PyO3, byggt stegvis tillsammans med flera agenter och
modeller där varje resultat formats vidare till nästa idé. Det gör varje fil
till **overifierat material** oavsett hur färdig den ser ut: en fil som "körde
i experimentet" är inte samma sak som en fil som är rätt. Tre mognadsklasser
måste hållas isär:

| Klass | Kännetecken | Exempel | Användning |
|---|---|---|---|
| **Prototyp** | Rust+Pyo3, körbar, `run_experiment.py` producerar output | `01`–`09.x`, `11.0/11.1` | läses; får återanvändas **endast** efter S0–S4 |
| **Plan** | README med `[ ]`-listor, ingen implementation | `09.4.2` (plan-delen), `10.1`, `10.3` | idén kan diskuteras, koden finns inte |
| **Attrapp** | namn och structar finns, logiken är påhittad | `09.4.2/src/lib.rs` ("AST" = radregex), `11.0` (`ConfidenceScorer` → hårdkodat `0.85`) | **endast** som negativt exempel |

### 1.1 Studera-först-protokollet (bindande)

Ingen rad från Comparative-Thinker får hamna i `src/` utan att S0–S4 är
ifyllda i denna fil under respektive inkrement. Protokollet finns för att
källkoden innehåller minst fyra fel som *ser rimliga ut* (§2.3), för att
"det fungerade i experimentet" inte är samma sak som "det är rätt" — och för
att koden skrevs av agenter utan att någon godkände den.

| Steg | Krav | Bevis |
|---|---|---|
| **S0** | **Proveniensgrind:** filen är agent-genererad utan granskningsspår. Den läses som overifierat material; ingen "det såg rätt ut"-genväg. | S0 ifylld i studieanteckningen |
| **S1** | Läs **hela** källfilen, inte bara funktionen | radintervall noterat |
| **S2** | Verifiera att den **kör**: `cargo build --release` + `python run_experiment.py` i experimentkatalogen | utfall + ev. fel noterat |
| **S3** | Skriv ett **karaktäriseringstest mot källan** som dokumenterar vad den faktiskt gör (även när det är fel) | testfil + utskrift |
| **S4** | Klassa varje bit: `QUESTION` (bara frågan/behovet lånas) / `DERIVE` (matematik eller läroboksalgoritm — härleds ur extern referens, inte ur denna fil) / `INSPECT` (läses som negativt exempel) / `DROP` (används inte) | motivering i en rad |
| **S5** | Skriv GTW-versionen från tomt blad, test först — utan att titta på källan medan du skriver | `src/…` + testnamn |

`QUESTION` och `DERIVE` är de enda klasser som får leda till kod. Skillnaden
mot en vanlig portning: `DERIVE` kräver en **extern referens** (CLRS för
grafalgoritmer, McCabe/Sonar för komplexitet, läroboksstatistik för varians).
Kan en bit inte hänvisas till en sådan referens är den `DROP` — då finns
ingen oberoende sanning att verifiera mot.

### 1.2 Studieanteckning (mall — kopiera per bit)

```markdown
#### Studieanteckning — <bit>

| Fält | Värde |
|---|---|
| Källa (läst) | `fil:rad` |
| S0 Proveniens | agent-genererad, ogranskad — läsmaterial |
| S1 Läst | hela filen, rad X–Y |
| S2 Körbar | `cargo build --release` → ok/fel; `run_experiment.py` → utfall |
| S3 Karaktäriseringstest | `<fil>` — vad den faktiskt gör |
| S4 Klassning | QUESTION / DERIVE / INSPECT / DROP — varför |
| Extern referens | krävs för `DERIVE` (t.ex. CLRS §22, Sonar cognitive complexity) |
| S5 GTW-form | `src/…::fn` + `test_…` |
```

### 1.3 Vad källan är

Fyra faktorer som styr hur materialet behandlas.

**1. Projektet är experimentellt.** 30 experiment där varje resultat har formats
vidare till nästa idé — det är arbetsmetoden, och den förklarar repots form
(dubbletter, halvfärdiga scaffolds, kursändringar). Det är också skälet att
koden inte kan läsas som en färdig produkt: den är ett led i en tankekedja,
inte ett slutresultat.

**2. Idénivån är det som bärs vidare.** Frågorna ("vad är detta likt?", "hur
eniga är pelarna?", "vad drabbas om jag ändrar X?"), pelar-modellen,
mognadsklasserna och experimentcykeln är den riktning arbetet drevs i — det är
dem planen använder. En **fråga** bärs vidare; en **fil** gör det inte
automatiskt.

Samma sak på filnivå: en agent­skriven fil utan godkännande är ett
**förslag**, inte ett beslut — oavsett hur färdig den ser ut. Det är precis
därför S0 finns: varje fil läses som overifierat material, och det som förs
vidare är frågan, inte raderna.

**3. Spåren av fleragents-arbetet syns i repot.** Att agenterna inte läste
föregående steg märks konkret:

| Spår | Vad det visar |
|---|---|
| `07_dependency_graph/LEARNINGS.md` = ordagrann kopia av `06`:s (fel titel kvar) | agenten byggde om i stället för att bygga på |
| `09.3` och `09.3.1` är identiska filer | samma sak, en nivå senare |
| `LICENSE` = `Copyright (c) [year]`, aldrig ifylld | repot scaffoldades, färdigställdes aldrig |
| `pyproject.toml` pekar på `src/` som inte finns + `cli.py` som inte finns | ingen körde entrypointen efter att den skrevs |

Det här är inte slarv — det är hur stegvis agentarbete utan mellanliggande
konsolidering ser ut i efterhand. Samma mönster är skälet till att koden ska
verifieras, inte ärvas.

**4. Namnkonventioner.** GTW:s yta är engelsk och `gtw:`-prefixad; CT:s
identifierare (`WorldEntity`, `find_n_hops_away`, `synthesize_quality_assessment`)
ska inte smitta. En PR som inför P1–P4 får inte innehålla namn som bara
existerar i CT — inte som förbud, utan för att läsaren då tror att koden är
porterad och slutar granska den.

---

## 2. Vad som inventerats

### 2.1 Repots artefakter, klassade

| Område | Filer | Klass | Relevans för GTW |
|---|---|---|---|
| Trinity-motorn (vektor + fuzzy + graf) | `01`–`04/src/lib.rs` | Prototyp | API-former för sökning/filter |
| Inferens över graf | `05` | Prototyp | → hör hemma i `island_scan`, inte som port |
| Kodbas-mapper | `06`, `07` | Prototyp | GTW gör det bättre (`explore`/`pack`/`curate`) |
| Avancerade graf-frågor | `07.5` | Prototyp | **P2** (formerna; algoritmerna skrivs om) |
| Stabilitet/perf/testobjekt | `08.x` | Prototyp | inget att hämta |
| Harmony/dissonans | `09.1` | Prototyp (41 rad) | **P1** (formeln) |
| Dynamisk viktning | `09.2` | Prototyp | nej — determinism |
| Query-refinement | `09.3`, `09.3.1` | Prototyp (identiska filer) | **P1** (förklarings­texterna) |
| Konfidenskalibrering | `09.4.1` | Prototyp | läsvärd vid P1 |
| AST-analys | `09.4.2` | Plan + **attrapp** | **P3** (bara idén) |
| Integrationsmotor | `09.5` | Prototyp | läsvärd vid P1/P4 (syntesformel) |
| Real-world-analys | `10.1` | Plan + schema | **P4** (JSON-formen) |
| Rapport-viewer + triage | `10.2`, `scripts/*` | Prototyp | **P4** (triage-idén) |
| Strukturell abstraktion | `11.0` | Attrapp | nej |
| Cross-domain-mönster | `11.1` | Prototyp (demo-nivå) | nej |
| `CODE_INDEX.md` | rot | agent-genererad | **negativt case** — se §2.2 |

### 2.2 GTW gör redan detta bättre (porta inte)

- **`CODE_INDEX.md` i CT** är 20 nyckelordsblock med `->deps`-hintar och
  `META:`-rader: inga nodvägar, ingen hash, ingen färskhetsgrind.
  GTW:s `code-index.md` bär git-rev + `Body-hash` + `check`-läge och
  varnar för handredigering. CT-filen är alltså ett **case som validerar
  GTW:s design** — och en möjlig *konsument* (`gtw code-index` på CT-repot).
- **`09.4.2` "AST-analys"** parsar inte AST (§2.3.3) — GTW har 29
  tree-sitter-parsrar.
- **`06`/`07` kodbas-mapper** (`rglob` + första 1000 tecken → embedding)
  motsvaras av `explore`, `pack`, `curate`, `index_entities`, `stats`.
- **`petgraph` + PyO3-brygga**: GTW har `FileGraph { relations, definitions,
  imports }` (`src/llm/relational_index.rs:26-35`) + SQLite-store. Ingen
  Python i produktionsvägen — det är hela poängen med "CPU är produkten".

### 2.3 Fyra fel i källkoden som ser rimliga ut

Dessa är **kodläsningsfynd**, inte reproduktioner. Poängen är inte att
såga CT — det är att visa varför S0–S4 inte är byråkrati.

**(1) `find_dependency_chain` bygger ingen väg** (`07.5/src/lib.rs:135-168`):
den fyller `parents` med *alla inkommande kanter* för varje DFS-besökt nod
och rekonstruerar baklänges. Resultatet är ingen garanterad väg, och vid
en cykel i `parents` blir `while let Some(&parent) = parents.get(&current)`
en **oändlig loop**:

```rust
// KÄLLAN (07.5:150-166) — farlig
while let Some(node_idx) = dfs.next(&self.graph) {
    for edge_ref in self.graph.edges_directed(node_idx, Direction::Incoming) {
        parents.insert(node_idx, edge_ref.source());   // <- vägen blandas med alla föräldrar
    }
}
// ...rekonstruktion följer `parents` bakåt utan cykelvakt
```

**(2) Panic på okända mål** (`04/src/lib.rs:204`, `05:281`, `07.5:337`):
`add_entity` skapar grafnoder även för relationer till entiteter som inte
finns, och uppslagningen unwrappar sedan:

```rust
let entity = self.entities.get(target_id).unwrap();   // <- dangling relation = krasch
```

GTW har redan rätt modell för detta: `ImpactReport::unresolved_calls`
(`src/llm/impact_analyzer.rs:10-18`) — okända mål **redovisas**, de kraschar
inte och de tystas inte.

**(3) "AST-analysen" är radregex** (`09.4.2/src/lib.rs:93-120`): den räknar
`line.contains("fn ")` och flaggar magiska tal via `line.contains("42")`.
`ASTNode`-structen deklareras men används aldrig. `CodeStructure`-formen är
däremot användbar som *utdataform*.

**(4) `find_cycles` ger bara första cykeln per startpunkt** och rekursionen
(`detect_cycle_util`, `07.5:355-388`) är inte djupbegränsad — på en stor
graf är stack-djupet en verklig risk. GTW:s relationsgraf är projektbred.

---

## 3. P1 — `harmony` / `dissonance` i `sense`

### 3.1 Idén som lånas

Comparative-Thinker mäter **oenighet mellan sina tre pelare** och gör den till
konfidens (`09.1_dissonance_detection/src/lib.rs:18-45`):

```rust
let mean = scores.iter().sum::<f64>() / 3.0;
let sum_sq_diff: f64 = scores.iter().map(|&s| (s - mean).powi(2)).sum();
let dissonance = (sum_sq_diff / (2.0 / 3.0)).clamp(0.0, 1.0); // 2/3 = max varians för 3 tal
let confidence = 1.0 - dissonance;
```

`2.0/3.0` är inte magi: **maxvariansen för N tal i `[0,1]` är `(N-1)/N`**
(ett etta, N−1 nollor: `(1−1/N)² + (N−1)(1/N)² = (N−1)/N`). Med N=3 blir det
`2/3`, dvs exakt CT:s konstant. Generaliseringen är alltså inte en ny idé —
det är samma formel med N som parameter, vilket GTW **måste** ha eftersom
grafkanalen är valfri (ingen graf på disk → 2 pelare).

```text
dissonance = Σ(sᵢ − s̄)² / ((N−1)/N)      , klampad till [0,1]
score      = 1 − dissonance
```

### 3.2 GTW:s tre pelare finns redan

`rerank_satellite_with_graphs` (`src/llm/semantic_index.rs:487`) fusionerar
tre kanaler med RRF (`RRF_K = 60`, `LEX_CHANNEL_WEIGHT = 0.5`,
`GRAPH_CHANNEL_WEIGHT = 0.2`; rad 298-307):

| Pelare | Var den beräknas | Skala idag |
|---|---|---|
| semantisk | cosine ur indexet | `0..=1` (rå) |
| lexikal | `matched / q_tokens` (rad 530-534) | `0..=1` (rå) |
| graf | `neighbor_rank` → `GRAPH_CHANNEL_WEIGHT * rrf(rank)` (rad 609-682) | `0..≈0.0033` (rang) |

**Det som saknas:** pelarna kastas in i RRF och försvinner. Svaret bär bara
`score` (fuserad, kan överstiga 1.0), `cosine` och `content_preview`
(`FileMatch`, `gnaw_sense.rs:177-189`). En agent kan inte se att toppträffen
är "stark semantiskt, noll lexikalt, ingen grafnärhet" — vilket är exakt den
situation där `suspect_reason` idag bara säger `low_cosine` eller
`no_lex_overlap` som *binär* flagga (`gnaw_sense.rs:393-414`).

**Kritiskt designkrav:** pelarna måste tas **före** RRF. RRF-värden ligger i
`0.0033…0.0166`-rymden; varians över dem mäter ingenting. Detta är den enda
punkten där en naiv implementation blir vackert fel.

**Funktionsgate (P1):** harmoni-matematiken (`harmony.rs`) är ren och
kompileras alltid; wiringen bor i `semantic_index.rs`/`gnaw_sense.rs`
(modernbert-området). §8.2:s `--no-default-features --all-targets`-grind
är det som fångar gating-fel — zoom-sense utan satellite ger
`pillars_available = 1` (neutral), aldrig fejkad konfidens.

### 3.3 Design

```rust
// src/llm/semantic_index.rs — utöka den interna träffen (additivt fält)

pub struct RerankedHit<'a> {
    pub entry: &'a NodeEmbedding,
    pub cosine: f32,
    pub adjusted: f32,
    /// Lexikal överlappning, 0..=1 — rå pelare för `Harmony`
    /// (rang-rymden i `adjusted` duger INTE för varians).
    pub lex: f32,
    /// Grafnärhet: Some(1.0) när filen är BFS-granne till ett seed,
    /// None när kanalen inte var tillgänglig (inga grafer / inga seeds).
    pub graph: Option<f32>,
}
```

```rust
// src/llm/harmony.rs (ny fil, ~90 rad)

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Harmony {
    /// 1 - dissonance, 0..=1
    pub score: f32,
    /// Normaliserad varians över de TILLGÄNGLIGA pelarna, 0..=1
    pub dissonance: f32,
    pub pillars: HarmonyPillars,
    /// Hur många pelare som faktiskt ingick (2 = ingen graf på disk)
    pub pillars_available: usize,
    pub verdict: HarmonyVerdict,
    /// En mening som förklarar oenigheten — None när pelarna är eniga
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct HarmonyPillars {
    pub semantic: f32,
    pub lexical: Option<f32>,
    pub graph: Option<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HarmonyVerdict {
    Harmonious,
    MinorDissonance,
    Disharmonious,
}

pub const DISSONANCE_MINOR: f32 = 0.25;
pub const DISSONANCE_HIGH: f32 = 0.50;
```

```rust
// src/llm/harmony.rs — kärnan (samma formel som 09.1, N som parameter)

pub fn harmony(p: &HarmonyPillars) -> Harmony {
    let mut s = vec![p.semantic.clamp(0.0, 1.0)];
    if let Some(l) = p.lexical {
        s.push(l.clamp(0.0, 1.0));
    }
    if let Some(g) = p.graph {
        s.push(g.clamp(0.0, 1.0));
    }
    let n = s.len();

    // EN kanal kan inte vara oense med sig själv: neutralt, aldrig fejkad 1.0.
    if n < 2 {
        return Harmony {
            score: s[0],
            dissonance: 0.0,
            pillars: p.clone(),
            pillars_available: n,
            verdict: HarmonyVerdict::Harmonious,
            suggestion: None,
        };
    }

    let mean = s.iter().sum::<f32>() / n as f32;
    let sum_sq: f32 = s.iter().map(|x| (x - mean).powi(2)).sum();
    // Max varians för N tal i [0,1] är (N-1)/N — N=3 ger 2/3, dvs CT:s konstant.
    let max_var = (n - 1) as f32 / n as f32;
    let dissonance = (sum_sq / max_var).clamp(0.0, 1.0);

    let verdict = if dissonance >= DISSONANCE_HIGH {
        HarmonyVerdict::Disharmonious
    } else if dissonance >= DISSONANCE_MINOR {
        HarmonyVerdict::MinorDissonance
    } else {
        HarmonyVerdict::Harmonious
    };

    Harmony {
        score: 1.0 - dissonance,
        dissonance,
        pillars: p.clone(),
        pillars_available: n,
        verdict,
        suggestion: suggestion_for(verdict, p),
    }
}

/// Förklaringen är det som gör måttet användbart — en siffra utan orsak
/// drunknar i bruset hos en agent (jfr `suspect_reason`: binärt, utan grad).
fn suggestion_for(v: HarmonyVerdict, p: &HarmonyPillars) -> Option<String> {
    let mut named: Vec<(&str, f32)> = vec![("semantic", p.semantic)];
    if let Some(l) = p.lexical {
        named.push(("lexical", l));
    }
    if let Some(g) = p.graph {
        named.push(("graph", g));
    }
    let (strong, s_val) = named
        .iter()
        .cloned()
        .fold(("", f32::MIN), |a, b| if b.1 > a.1 { b } else { a });
    let (weak, w_val) = named
        .iter()
        .cloned()
        .fold(("", f32::MAX), |a, b| if b.1 < a.1 { b } else { a });

    match v {
        HarmonyVerdict::Harmonious => None,
        HarmonyVerdict::MinorDissonance => Some(format!(
            "channels partly disagree ({} {:.2} vs {} {:.2}) — verify with read_node before trusting the top hit",
            strong, s_val, weak, w_val
        )),
        HarmonyVerdict::Disharmonious => Some(format!(
            "{} is confident ({:.2}) while {} is not ({:.2}); the match may be topical rather than structural",
            strong, s_val, weak, w_val
        )),
    }
}
```

### 3.4 Wire-format (engelska nycklar, additivt = icke-brytande)

```json
{
  "file_path": "src/core/blast.rs",
  "node_path": "1.2",
  "score": 0.0165,
  "cosine": 0.68,
  "harmony": {
    "score": 0.29,
    "dissonance": 0.71,
    "pillars": { "semantic": 0.68, "lexical": 0.0, "graph": null },
    "pillars_available": 2,
    "verdict": "disharmonious",
    "suggestion": "semantic is confident (0.68) while lexical is not (0.00); the match may be topical rather than structural"
  }
}
```

`pillars_available: 2` + `graph: null` är den ärliga deklarationen: ingen
graf på disk betyder *inte* "grafen höll med" — det betyder "grafen var inte
med". Det skiljer GTW från CT, som alltid räknar tre pelare.

### 3.5 Tester (skrivs före implementationen)

| Test | Krav |
|---|---|
| `harmony_single_pillar_is_neutral_not_confident` | N=1 → `dissonance == 0.0`, `score == pillar` |
| `harmony_two_agreeing_pillars_is_one` | `[0.9, 0.9]` → `score ≈ 1.0` |
| `harmony_two_max_disagreement_is_zero` | `[1.0, 0.0]` → `dissonance ≈ 1.0` |
| `harmony_three_matches_ct_reference` | `[0.9, 0.2, 0.1]` → samma dissonans som CT-formeln |
| `harmony_missing_graph_is_not_agreement` | `graph: None` får inte ge högre `score` än samma fråga med `graph: Some(0.0)` |
| `rerank_exposes_raw_pillars` | `RerankedHit.lex`/`graph` bär råa (0..1), inte rrf |
| `satellite_response_carries_harmony` | integration: `sense`-svaret har `harmony` på varje träff |

### 3.6 Studieuppgifter innan implementation

| # | Uppgift | Klassning |
|---|---|---|
| 1 | S1–S3 på `09.1_.../src/lib.rs` (41 rad) | **DERIVE** — normaliserad varians är läroboksstatistik; §3.1 härleder den utan källan |
| 2 | S1 på `09.3.1_.../src/lib.rs` (2,3 kB): förklaringstexterna | **QUESTION** — behovet "förklara oenigheten" lånas; texten formuleras ur GTW:s egna fält |
| 3 | S1 på `09.5_.../src/lib.rs:117-130`: syntesformel `base + quality*0.4 + harmony*0.1` | **DROP** — vikterna är påhittade, ingen evals bakom |
| 4 | Läs `09.4.1` (konfidenskalibrering) för tröskelval | **DERIVE** — trösklar kalibreras mot GTW:s egna evals, inte mot CT:s tal |

**Insats:** S (~150 rad kod + 7 tester). **Risk:** låg.
**Roadmap-fäste:** ROADMAP rad 384 (confidence threshold), rad 406 (feedback loop).

---

## 4. P2 — graf-frågor: `cycles`, `impact(depth)`, `chain`

### 4.1 Vad som lånas: frågeformerna, inte algoritmerna

CT:s `07.5_advanced_dependency_queries` exponerar fem frågor
(`src/lib.rs:112-215` + `355-388`):

| CT-metod | Fråga | GTW idag |
|---|---|---|
| `find_dependents(target)` | "vad beror på X?" | `code_index kind=relations` med `to_name`-filter (`code_index_db.rs:1186`) |
| `find_affected_files(source, max_depth)` | "vad påverkas om jag ändrar X?" | `blast` — men **bara CLI** (`cli.rs:1659`) |
| `find_dependency_chain(from, to)` | "vägen från A till B" | **saknas** |
| `find_cycles()` | "finns cirkulära beroenden?" | **saknas** (`grep -rni "cycle" src/` → 0 träffar) |
| `find_users_of_dependency(dep)` | "vem använder externa dep X" | `symbols` (unresolved/external), delvis |

GTW:s material finns: `relations(from_id, to_name, to_id, relation_type, line,
file)` + `entities(id, file, kind, name, line)` (`code_index_db.rs:73-91`),
`FileGraph` (`relational_index.rs:26-35`), `blast`, `gnaw-graph`
(mermaid/DOT/tree). Det som saknas är **tre frågor**, inte ett lager.

### 4.2 Var de ska bo

Två verktyg finns redan som CLI-only och borde in i MCP-ytan: `blast`, `graph`.
Men GTW har redan 38 MCP-verktyg, och varje verktyg kostar schema-tokens i
varje `tools/list`. Därför:

- **Utöka `code_index`** med `kind: "cycles"` och `kind: "impact"` — dispatch
  finns redan på ett ställe (`handle_code_index_mcp`-dispatchen), så det är
  +2 armar i en befintlig match, inte +2 verktyg i katalogen. Svaren går
  genom standard-kuvertet (`content[]`, `envelope_code_index` 2026-11-10) —
  nya `kind`-värden ärver det, inget nytt exponeringsbeslut.
- **Nya funktioner i en egen modul** `src/core/code_graph.rs` som tar
  `&Connection` (inte filsystemet): då är de testbara mot en temp-DB utan
  hermetik-problem (AGENTS.md lärdom #6).

### 4.3 Algoritmerna skrivs om — och varför

**Reachability (impact) görs i SQL, inte i Rust.** Poängen är att SQLite:s
`UNION` (inte `UNION ALL`) **dedupar**, vilket gör att cykler terminerar av
sig själva. Det är exakt den klass av bugg som `find_dependency_chain` har
(§2.3.1) — vi undviker den genom att välja rätt primitiv, inte genom att
lappa en trasig loop:

```rust
// src/core/code_graph.rs — nåbara noder inom djupgräns
pub fn reachable(
    conn: &Connection,
    start: &str,
    max_depth: usize,
) -> Result<Vec<Reach>> {
    let mut stmt = conn.prepare(
        r#"
        WITH RECURSIVE walk(node, depth) AS (
            SELECT ?1, 0
          UNION                                  -- UNION, inte UNION ALL: dedupar
            SELECT r.to_id, w.depth + 1          -- => cykler terminerar
              FROM relations r
              JOIN walk w ON r.from_id = w.node
             WHERE r.to_id IS NOT NULL           -- unresolved mål hoppas över
               AND w.depth < ?2
        )
        SELECT node, MIN(depth) AS depth
          FROM walk
         WHERE depth > 0
         GROUP BY node
         ORDER BY depth, node
        "#,
    )?;
    let rows = stmt.query_map(params![start, max_depth as i64], |r| {
        Ok(Reach { entity: r.get(0)?, depth: r.get::<_, i64>(1)? as usize })
    })?;
    Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
}
```

`WHERE r.to_id IS NOT NULL` är den strukturella motsvarigheten till
`unresolved_calls`: oupplösta mål **räknas** (separat fråga på `symbols`),
men de ingår inte i gången — de kan inte ge en falsk påverkanskedja.

**Känd begränsning i v1 (dokumenteras, inte döljs):** kanter förankrade i
syntetiska callsite-id (`gtw:{file}:callsite:{line}` som `from_id`)
traverseras inte av `reachable`/`chain` — ett callsite-id kan aldrig bli en
nod i gången (de förekommer bara som `from_id`, och `to_id` är FK mot
entiteter). Påverkans-kedjor som börjar i scope-less-anrop under-räknas
därför; de syns via rå relations-fråga i stället. V2-refinement: mappa
syntetiska id till filnivå-påverkan.

**Cykler görs i Rust** — därför att vi vill ha *vägen* (för attribution: fil,
rad, kedja), inte bara "ja/nej". Här är den klass av fel vi undviker:
explicit `on_path`-stack i stället för föräldrakarta, kanonisering av
rotation så samma cykel inte rapporteras N gånger, och en hård djupgräns
mot stack-rymlighet:

```rust
// src/core/code_graph.rs
#[derive(Debug, serde::Serialize)]
pub struct Cycle {
    /// Entity-ids i cykelordning (första == sista implicit)
    pub nodes: Vec<String>,
    /// Distinkta filer i cykeln — det en människa/agent faktiskt åtgärdar
    pub files: Vec<String>,
}

pub fn cycles(conn: &Connection, max_depth: usize) -> Result<Vec<Cycle>> {
    // 1. Bygg entity-nivå-adjacency. Syntetiska callsite-ids
    //    (gtw:{file}:callsite:{line}) är INGA entity-rader — de mappas till
    //    sin fil, aldrig till en påhittad nod (index_relations.rs:14-23).
    let mut adj: HashMap<String, Vec<String>> = HashMap::new();
    let mut stmt = conn.prepare(
        "SELECT from_id, to_id FROM relations WHERE to_id IS NOT NULL",
    )?;
    for row in stmt.query_map([], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
    })? {
        let (from, to) = row?;
        adj.entry(from).or_default().push(to);
    }

    // 2. Iterativ DFS med explicit path-stack. Ingen rekursion: djupgränsen
    //    är en parameter, inte en gissning om trådens stack.
    let mut found: Vec<Cycle> = Vec::new();
    let mut seen: HashSet<Vec<String>> = HashSet::new();
    let mut adj_vec: Vec<(&String, &Vec<String>)> = adj.iter().collect();
    adj_vec.sort_by(|a, b| a.0.cmp(b.0)); // determinism: samma DB => samma svar

    for (start, _) in adj_vec {
        let mut stack: Vec<(String, usize)> = vec![(start.clone(), 0)];
        let mut path: Vec<String> = Vec::new();
        let mut on_path: HashSet<String> = HashSet::new();

        while let Some((node, idx)) = stack.pop() {
            // Backtrack: lämna noder som inte längre är aktuella i vägen
            while path.len() > idx {
                if let Some(leaving) = path.pop() {
                    on_path.remove(&leaving);
                }
            }
            if path.len() >= max_depth {
                continue;
            }
            let edges = adj.get(&node).map(|v| v.as_slice()).unwrap_or(&[]);
            for next in edges {
                if on_path.contains(next) {
                    // Cykel: skär ut den från `next`s position och kanonisera
                    let from = path.iter().position(|n| n == next).unwrap_or(0);
                    let mut cyc = path[from..].to_vec();
                    rotate_to_smallest(&mut cyc);
                    if seen.insert(cyc.clone()) {
                        found.push(Cycle { files: files_of(&cyc), nodes: cyc });
                    }
                    continue;
                }
                path.push(next.clone());
                on_path.insert(next.clone());
                stack.push((next.clone(), path.len()));
            }
        }
    }
    Ok(found)
}

/// A->B->C och B->C->A är samma cykel. Rotera till minsta element så
/// dedupen blir stabil oavsett startpunkt.
fn rotate_to_smallest(c: &mut [String]) {
    if let Some(pos) = (0..c.len()).min_by_key(|&i| &c[i]) {
        c.rotate_left(pos);
    }
}
```

`files_of()` slår upp `entities.file` per id och faller tillbaka på
`gtw:{file}:`-prefixet när id:t är syntetiskt — **aldrig** `unwrap()` på en
entities-rad som kanske inte finns (§2.3.2).

`chain(from, to)` är samma BFS som `reachable` men med föräldrakarta byggd
**under** gången från startnoden (ut-kanter), och med `visited` som vakt:

```rust
// Skiss — föräldern sätts när noden FÖRST upptäcks, aldrig bakåt.
let mut parent: HashMap<String, String> = HashMap::new();
let mut queue: VecDeque<String> = VecDeque::from([from.to_string()]);
let mut seen: HashSet<String> = HashSet::from([from.to_string()]);
while let Some(node) = queue.pop_front() {
    if node == to { break; }
    for next in neighbours(&node) {
        if seen.insert(next.clone()) {
            parent.insert(next.clone(), node.clone());
            queue.push_back(next);
        }
    }
}
// Rekonstruktion: gå parent bakåt från `to`, och AVBRYT om en nod upprepas
// (kan inte hända med `seen`, men vakten kostar två rader och gör buggklassen omöjlig).
```

### 4.4 Tester

| Test | Krav |
|---|---|
| `reachable_bounded_depth` | djup 1 ger bara direkta grannar |
| `reachable_terminates_on_cycle` | `A→B→A` ger `[B]`, inte häng |
| `reachable_skips_unresolved_targets` | rad med `to_id = NULL` syns i `symbols`, aldrig i gången |
| `cycles_finds_simple_cycle` | `A→B→C→A` → en cykel med tre noder |
| `cycles_dedups_rotations` | `A→B→C→A` och `B→C→A→B` = **samma** cykel |
| `cycles_self_loop` | `A→A` rapporteras en gång |
| `chain_returns_none_on_no_path` | ingen väg → `None` inom tidsgräns (regressionstest för §2.3.1) |
| `chain_is_a_real_path` | varje steg i svaret finns som kant i DB |
| `synthetic_callsite_ids_do_not_panic` | `gtw:{file}:callsite:{line}` som `from_id` kraschar inte |

Skissen i §4.3 (path-stack-varianten) ska valideras mot en brute-force-
referens i testet: räkna **alla enkla cykler** på en känd liten graf och
jämför mängderna. Det är billigare än att lita på att stack-indexeringen är
rätt vid första försöket.

### 4.5 Studieuppgifter

| # | Uppgift | Klassning |
|---|---|---|
| 1 | S1: läs **hela** `07.5/src/lib.rs` (410 rad) | — |
| 2 | S2: `cargo build --release` (kräver `PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1`, `LEARNINGS.md:26`) + `python run_experiment.py` | protokollför utfall |
| 3 | **S3: bevisa buggen** — bygg en 3-nodscykel i `run_experiment.py`, anropa `find_dependency_chain` med timeout; dokumentera häng/fel väg innan omskrivning | **INSPECT** — syftet är att dokumentera felet |
| 4 | Läs `blast.rs` + `impact_analyzer.rs` helt — P2 ska återanvända deras fältnamn (`unresolved_calls`), inte hitta på nya | **intern** — GTW:s modell vinner, inget lånas |
| 5 | `petgraph`-beroendet | **DROP** — GTW har ingen petgraph och behöver ingen |

**Insats:** S–M (~250 rad + 9 tester). **Risk:** låg–medel (algoritmik, men
lokal och testbar mot temp-DB).
**Roadmap-fäste:** rad 145 (`impact`), rad 412 (side-effect prediction),
`docs/CODE_INDEX_SCAN_TOOLS_PROPOSAL.md`.

---

## 5. P3 — `complexity` på riktig AST

### 5.1 Vad som lånas — och vad som absolut inte lånas

CT:s Exp 09.4.2 har rätt **utdataform** (`09.4.2/src/lib.rs:28-63`):

```rust
pub struct CodeStructure { file_path, language, total_nodes, total_functions,
                           total_classes, total_variables, max_depth,
                           complexity_metrics: ComplexityMetrics, patterns }
pub struct ComplexityMetrics { cyclomatic_complexity, cognitive_complexity,
                               maintainability_index, technical_debt_ratio }
```

…men implementationen är en **attrapp** (§2.3.3): radregex, `line.contains("42")`
som "magic numbers", och en `maintainability_index` som är
`(effort / volume).recip()` — dimensionslöst nonsens som ger *högre* värde för
mer komplex kod. **Allt utom fältnamnen klassas `DROP`.**

### 5.2 GTW:s nuläge

| Komponent | Status |
|---|---|
| `TreeNode { node_type, content, start_line, end_line, children, path }` | `src/parser/mod.rs:37-49` |
| `node_type` = **rå tree-sitter-kind** (`node.kind()`) | `src/parser/rust.rs:47` |
| Språkmedveten regelmatchning finns | `rules::language_matches` (`rules.rs:426`), `check_code` (`rules.rs:392`) |
| Komplexitet idag | **nyckelords-proxy** i `guardian.rs:157 estimate_complexity()` — logiska markörer, används för att varna vid komplexitetsfall |
| ROADMAP | rad 147: `complexity` — **ej byggd** |

### 5.3 Design

Kärnan är att räkna **beslutsnoder per språk** — inte text. Tabellen är
arbetsbördan, koden är liten:

```rust
// src/core/complexity.rs

#[derive(Debug, Clone, serde::Serialize, Default)]
pub struct Metrics {
    /// Beslutsvägar + 1 (baskomplexitet)
    pub cyclomatic: u32,
    /// Nästlingsviktad: varje flödesbrott kostar 1 + aktuellt djup
    pub cognitive: u32,
    /// Max nästlingsdjup av flödesnoder
    pub max_nesting: u32,
    pub lines: u32,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct FunctionMetrics {
    pub name: String,
    pub node_path: String,
    pub start_line: usize,
    pub end_line: usize,
    pub metrics: Metrics,
}

/// Beslutsnoder per språk. Nyckel = nodtyp som tree-sitter rapporterar.
/// Att fylla på denna tabell ÄR arbetet — allt annat är mekanik.
fn decision_kinds(language: &str) -> &'static [&'static str] {
    match language {
        "rust" => &[
            "if_expression", "match_arm", "for_expression", "while_expression",
            "loop_expression", "try_expression",
        ],
        "python" => &[
            "if_statement", "elif_clause", "for_statement", "while_statement",
            "except_clause", "with_statement", "boolean_operator",
            "conditional_expression",
        ],
        "typescript" | "javascript" => &[
            "if_statement", "for_statement", "for_in_statement", "while_statement",
            "switch_case", "catch_clause", "ternary_expression",
        ],
        "go" => &["if_statement", "for_statement", "expression_switch_statement",
                  "type_switch_statement", "select_statement"],
        // … c, cpp, java, csharp, zig, php, kotlin, swift, bash
        _ => &[],
    }
}
```

Nästling kräver att man går **nedåt** med djup, inte att man räknar rader:

```rust
fn walk(node: &TreeNode, depth: u32, lang: &str, m: &mut Metrics) {
    let is_decision = decision_kinds(lang).contains(&node.node_type.as_str())
        || is_bool_branch(node);          // && och || är barn i binary_expression

    let mut next_depth = depth;
    if is_decision {
        m.cyclomatic += 1;
        // Kognitiv komplexitet: brott djupt i strukturen kostar mer (Sonar-regeln:
        // nesting + 1). Detta är skillnaden mot cyklomatisk, och skälet att
        // båda rapporteras: en platt 20-grenars match är inte samma sak som
        // 20 nästlade if-satser.
        m.cognitive += 1 + depth;
        next_depth = depth + 1;
        m.max_nesting = m.max_nesting.max(next_depth);
    }
    for child in &node.children {
        walk(child, next_depth, lang, m);
    }
}

/// `x && y` i tree-sitter: operatorn är ett BARN, inte en egen nodtyp.
fn is_bool_branch(node: &TreeNode) -> bool {
    node.node_type == "binary_expression"
        && node
            .children
            .iter()
            .any(|c| matches!(c.content.trim(), "&&" | "||"))
}
```

**`maintainability_index` rapporteras inte i P3.** Det klassiska MI kräver
Halstead-volym (operator/operand-räkning per språk) — att hoppa över det och
säga det rakt ut är bättre än CT:s fejkade index. P3 rapporterar råa tal
(`cyclomatic`, `cognitive`, `max_nesting`, `lines`) och låter trösklar flagga;
MI blir ett eget inkrement om/när Halstead finns.

### 5.4 Persistens: två steg, medvetet isär

- **P3a (ingen schemaändring):** `complexity` som CLI-kommando + MCP-läsning,
  beräknad on demand ur `TreeNode`. Inget skrivs till DB. Detta är 80 % av
  värdet (agenten kan fråga "vilka funktioner i den här filen är värst").
- **P3b (schema):** kolumner på `items` (`cyclomatic`, `cognitive`,
  `max_nesting`). Kräver migrering + att `open_or_recreate`-vägen hanteras.
  `code_index_db.rs:67-71` slår fast **"consolidation rule — never a fifth
  index family"**: utöka `items`, starta aldrig en ny tabell för metrics.

P3a först. P3b bara om någon faktiskt frågar efter sortering över hela
projektet — annars är det schema-skuld utan konsument.

### 5.5 Tester

| Test | Krav |
|---|---|
| `complexity_flat_match_is_high_cyclomatic_low_cognitive` | 10 `match_arm` på djup 1 → CC 11, kognitiv 10 |
| `complexity_nested_ifs_is_high_cognitive` | 3 nästlade `if` → kognitiv 1+2+3 = 6, CC 4 |
| `complexity_bool_operator_is_a_branch` | `a && b` räknas som beslut, inte som nod |
| `complexity_per_language_kind_table` | en fixture per språk (rust/python/ts/go) med känt svar |
| `complexity_ignores_strings_and_comments` | text i strängar påverkar inget (det är hela poängen mot CT) |
| `complexity_function_boundaries` | metrics per funktion, inte per fil |

`complexity_ignores_strings_and_comments` är **regressionstestet mot
CT-klassen**: en radregex-baserad implementation klarar det inte.

### 5.6 Studieuppgifter

| # | Uppgift | Klassning |
|---|---|---|
| 1 | S1–S3 på `09.4.2/src/lib.rs`: bygg och kör, dokumentera att "AST-analysen" ger samma svar på en minifierad rad som på strukturerad kod | **DROP** (bevis) |
| 2 | Läs `guardian.rs:60-90,157` — proxy-måttet ska **ersättas**, inte samexistera | **intern** |
| 3 | Verifiera kind-namn mot GTW:s parsrar (`analyze` på fixture per språk) — inte mot gissning | underlag för tabellen |

**Insats:** M (tabellen + fixtures per språk). **Risk:** medel — fel kind-namn
ger tyst fel siffra, därför en fixture per språk.
**Roadmap-fäste:** rad 147.

---

## 6. P4 — JSON-analysartefakt + triage (smalt)

### 6.1 Vad som lånas

Två saker, båda **former**:

1. **Rapportschemat.** CT:s `reporting.py:34-61` läser
   `problematic_functions[]` med `function_name`, `location.file_path` och
   `patterns[].description` — en platt, konsumentvänlig form. GTW:s
   `report.rs:28` genererar markdown ur ALF + transaktionsloggen, dvs en
   *ändringsjournal*; det finns ingen *analyssammanfattning* som maskin
   kan konsumera.
2. **Triage-idén.** `scripts/create_issues_from_report.py` (+ `dry_run_issue.py`)
   gör fynd till GitHub-issues. `CONCEPTS_VS_ARTIFACTS.md:10,20` beskriver
   steget. **Koden är oläst** — den ska S1–S3:as innan något lånas.

### 6.2 Design (GTW-versionen)

Artefakten skrivs till `.gnawtreewriter_ai/reports/<timestamp>.json` och
byggs av **befintliga** verktyg — inget nytt mätsystem:

```json
{
  "schema": "gtw-analysis/1",
  "generated_at": "2026-11-10T12:00:00Z",
  "root": ".",
  "git": { "head": "ed96328", "dirty": true },
  "summary": { "files": 214, "findings": 37, "cycles": 2, "unresolved": 118 },
  "findings": [
    {
      "rule_id": "proj_no_unwrap",
      "severity": "warning",
      "message": "…",
      "file": "src/core/blast.rs",
      "line": 96,
      "node_path": "1.2.3",
      "source": "lint"
    }
  ],
  "cycles": [{ "nodes": ["a", "b"], "files": ["src/a.rs", "src/b.rs"] }],
  "complexity": [{ "file": "src/cli.rs", "name": "handle_lint", "cyclomatic": 24, "cognitive": 31 }]
}
```

Källor: `lint` (finns), `code_graph::cycles` (P2), `complexity` (P3),
`code_index coverage` (finns, inkl. `unresolved`). Varje fynd bär `source`
så konsumenten kan vikta — samma princip som `sources: [{file, node_path}]`
i `investigate`/`get_semantic_report` (ROADMAP 217-228).

Triage-skriptet läggs i `scripts/` (där `gtw_adoption_report.py` redan bor),
**inte** i kärnan: det kräver nätverk + GitHub-auth, vilket inte hör till en
CPU-lokal editor. Default `--dry-run`, precis som CT:s `dry_run_issue.py`.

### 6.3 Icke-mål

- **Ingen web-viewer i kärnan.** CT:s `10.2` (viewer.js + server.py) är trevlig
  men GTW:s publik är agenter; JSON + `report.md` räcker. Om en viewer behövs
  är den ett `scripts/`-projekt, inte en binärfunktion.
- **Ingen automatisk issue-skapning i CI** förrän `--dry-run`-utfallet setts
  under några veckor.

### 6.4 Studieuppgifter

| # | Uppgift | Klassning |
|---|---|---|
| 1 | S1–S3 på `scripts/create_issues_from_report.py` (10,5 kB) | oläst → **klassning efter S3** |
| 2 | S1 på `10.2/server.py` + `report_templates/viewer.js` | **DROP** för kärnan (viewer utanför scope) |
| 3 | Jämför CT:s `problematic_functions`-form med GTW:s `Finding` (`rules.rs:61-72`) — GTW:s är rikare (captures, node_path) | **intern** — GTW:s form vinner; CT:s platta projektion högst som *vy* |

**Insats:** S–M. **Risk:** låg för JSON; medel för triage (auth/rate-limits).

---

## 7. Icke-mål (explicit, så det inte smyger in)

| Idé | Varför inte |
|---|---|
| **Dynamisk pelarviktning** (`09.2`) | Rankningen måste vara deterministisk för CI/evals. Vikter som ändrar sig per fråga gör en regression omöjlig att attribuera. CT:s egen slutsats var att det gjorde konfidensen "too restrictive". Vikter är konstanter i koden (som `LEX_CHANNEL_WEIGHT` idag) — ändras via PR + evals, inte av sig själv. |
| **Strukturell abstraktion / cross-domain-analogier** (`11.0`/`11.1`) | Annan produkt. GTW:s domän är kodstruktur; `sense` + relationsgraf täcker behovet. Dessutom: `11.0` är attrapp (hårdkodade `0.85`/`0.15`) och `11.1` är demo-nivå med trasig konfidens. |
| **Graf-inferens som egna regler** (`05`) | Hör hemma i `island_scan`/test-gap i `CODE_INDEX_SCAN_TOOLS_PROPOSAL.md`, där fynd-formatet redan finns. Att bygga en parallell inferensmotor vore en fjärde sanning. |
| **`petgraph` / `ndarray` / Python-bro i produktionsvägen** | GTW:s hela position är "CPU är produkten", ren Rust, inga tunga beroenden. BFS/DFS på `HashSet`/`HashMap` räcker (P2). |
| **CT:s `CODE_INDEX.md`-format** | Sämre än GTW:s på varje punkt (hash, nodvägar, färskhetsgrind). |
| **Automatisk issue-skapning utan dry-run** | Skräp i backloggen är dyrare än en manuell granskning. |

---

## 8. Ordning, grindar och definition of done

### 8.1 Stegordning

| Steg | Leverans | Beroende |
|---|---|---|
| **P1** | `harmony` i `sense` | inget — kan börjas direkt |
| **P2** | `code_index kind=cycles\|impact` + `chain` | kräver att `code-index` är färskt (`refresh`) |
| **P3a** | `complexity` (CLI + MCP-läsning) | inget |
| **P4** | `analysis --json` + `scripts/triage_report.py` | P2 + P3a (källor), `lint` (finns) |
| P3b | metrics i `items` | bara om konsument finns |

### 8.2 Grindar (samma som repot redan kör)

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test
cargo check --no-default-features --all-targets   # CLAUDE/AGENTS lärdom #5 — cfg-hål syns bara här
```

Plus, per inkrement:

- **P1:** `sense`-svaret måste vara identiskt (utom det additiva fältet) för
  en känd fråga mot en känd fixture — inga överraskningar i rankning.
- **P2:** temp-DB-fixtures (AGENTS.md lärdom #6: aldrig repo-root-state).
- **P3:** fixture per språk; `complexity_ignores_strings_and_comments`.
- **P4:** artefakten valideras mot `gtw-analysis/1` i ett test (schema-drift
  fångas innan en konsument gör det).

### 8.3 Definition of done per inkrement

1. Testerna i inkrementets tabell finns och är gröna.
2. `cargo check --no-default-features --all-targets` är grön.
3. MCP-svaret har engelska nycklar och är additivt (gamla konsumenter opåverkade).
4. Ett exempel med **verkligt** utfall är klistrat i denna fil (inte påhittat).
5. Studieanteckningarna (§1.2) är ifyllda för varje lånad bit.
6. `CHANGELOG.md` + `ROADMAP.md` uppdaterade; `code-index.md` omgenererad.
7. **Namn-grind:** diffen innehåller inga identifierare som bara finns i
   Comparative-Thinker (§1.3.3) — granskas manuellt i PR:en.

---

## 9. Riskregister

| Risk | Sannolikhet | Effekt | Motmedel |
|---|---|---|---|
| **Kopiering utan granskning** ("det ser ju färdigt ut") | medel | hög — vi ärver fyra kända fel (§2.3) | S0–S4 obligatoriskt; karaktäriseringstest innan omskrivning |
| Fel trösklar i P1 (dissonans) | medel | medel — agenter slutar lita på fältet | trösklar kalibreras mot GTW:s evals; `verdict` + `suggestion` kan läsas utan att lita på talet |
| Fel kind-namn i P3 (tyst fel siffra) | **hög** | medel | en fixture per språk; `complexity_ignores_strings_and_comments` |
| P2:s cykelalgoritm blir fel vid första försöket | medel | låg (lokal, testbar) | brute-force-referens i testet (§4.4) |
| MCP-ytan sväller (fler verktyg) | låg | medel — schema-tokens i varje `tools/list` | utöka `code_index` i stället för nya verktyg (§4.2) |
| Schema-drift i P3b | låg | medel | P3b bara med konsument; migrering + `open_or_recreate` testas |
| CT-repot förblir ogranskat och citeras som sanning | hög | låg–medel | denna fil är den enda referensen; CT är *läst*, inte *auktoritet* |

---

## Appendix A — Källkarta (vad som faktiskt lästes)

| Fil i CT | Rader | Vad den gav |
|---|---|---|
| `09.1_dissonance_detection/src/lib.rs` | 1–52 (hela) | variansformeln + normaliseringskonstanten |
| `09.2_dynamic_weighting/src/lib.rs` | 1–91 (hela) | varför dynamisk viktning inte ska in |
| `03_graph_integration/src/lib.rs` | 1–206 (hela) | `find_similar_filtered` (hårt filter + mjuk bonus) |
| `04_multihop_reasoning/src/lib.rs` | 1–245 (hela) | `find_n_hops_away` + `unwrap()`-risken |
| `05_inference_deduction/src/lib.rs` | 1–322 (hela) | inferens-idén (→ `island_scan`, inte port) |
| `07.5_advanced_dependency_queries/src/lib.rs` | 1–168, 315–411 | frågeformerna + `find_dependency_chain`-felet |
| `09.4.2_ast_analysis/src/lib.rs` | 1–268 (hela) | utdataformen; attrapp-beviset |
| `09.5_integration_synthesis/src/lib.rs` | 1–300, `README.md` | syntesformeln (DROP) |
| `comparative_thinker/reporting.py` | 1–68 (hela) | rapportens konsumentform |
| `scripts/ctool.py` | 1–88 (hela) | "komponentkatalog"-mönstret (CLI-discovery) |
| `LICENSE` | 1–6 | ofylld mall |
| `pyproject.toml` | 1–40 | MIT-deklaration + trasig paketering (`src/` saknas) |
| `roadmap.md`, `planning.md`, `LEARNINGS.md`, `AGENTS.md`, `DOCS/*` | utdrag | projektets egen drift-analys och metodik |
| `11.0`, `11.1` (`main.py`, `README`, `RESULTS`) | utdrag | attrapp-nivån; underlag för "nej" |

**Inte läst (kräver S1 före eventuell användning):**
`scripts/create_issues_from_report.py` (10 559 B), `scripts/dry_run_issue.py`,
`10.2_report_viewer/*` (`server.py`, `viewer.js`, `generate_demo.py`),
`09.3.1_query_refinement_fixes/src/lib.rs`, `09.4.1_confidence_calibration/src/lib.rs`,
`08.x`-serien, `09_harmony_analysis_base`.

## Appendix B — GTW-fästen (verifierade i koden)

> **Ankra med SYMBOL, inte rad:** rader ruttnar (t.ex. `mcp/mod.rs:3109` är
> idag dispatchen i `handle_code_index_mcp`; `code_index_db.rs:73-91` → 81
> efter schema v4). Rad-talen nedan är inkrement-dagens; symbol-namnen är
> de bestående referenserna.

| Yta | Fil:rad | Används av |
|---|---|---|
| `rerank_satellite_with_graphs()` | `src/llm/semantic_index.rs:487` | P1 |
| RRF-konstanter | `semantic_index.rs:298-307` | P1 |
| lexikalt pelarvärde `matched/q_tokens` | `semantic_index.rs:530-534` | P1 |
| grafkanal (`neighbor_rank`) | `semantic_index.rs:609-682` | P1 |
| `RerankedHit` | `semantic_index.rs:388-393` | P1 |
| `FileMatch` / `SatelliteQuality` | `src/llm/gnaw_sense.rs:177-189` / `146-153` | P1 |
| `suspect_reason`-logiken | `gnaw_sense.rs:393-414` | P1 |
| `relations`/`entities`-schema | `src/core/code_index_db.rs:73-91` | P2 |
| `load_relations()` + MCP-dispatch | `code_index_db.rs:1186`, `src/mcp/mod.rs:3109` | P2 |
| syntetiska callsite-id | `src/core/index_relations.rs:14-35` | P2 |
| `blast()` / `ImpactReport` | `src/core/blast.rs:59`, `src/llm/impact_analyzer.rs:10-18` | P2 |
| `FileGraph` | `src/llm/relational_index.rs:26-35` | P2 |
| `TreeNode` (+ rå `node.kind()`) | `src/parser/mod.rs:37-49`, `src/parser/rust.rs:47` | P3 |
| `language_matches` / `check_code` | `src/core/rules.rs:426` / `392` | P3 |
| `estimate_complexity` (proxy) | `src/core/guardian.rs:157` | P3 (ersätts) |
| `Finding` | `src/core/rules.rs:61-72` | P4 |
| `generate_markdown_report` | `src/core/report.rs:28` | P4 |
| ROADMAP-mål | `ROADMAP.md:145, 147, 384, 406, 412` | alla |
| scan-tools-förslaget | `docs/CODE_INDEX_SCAN_TOOLS_PROPOSAL.md` | P2, P4 |

## Appendix C — Städposter i CT-repot (inte GTW:s uppgift, men bokförda)

De här är spår av att repot byggdes experimentellt, steg för steg, snarare än
konsoliderades. De hör hemma i CT-repot; om det ska användas som källa igen är
de värda att stänga först. Det är ett beslut om det repot — planen kräver
inget av det.

1. **`LICENSE`:** fyll i årtal och hållare, t.ex. `Copyright (c) 2025 gnaw Software`.
2. **`pyproject.toml`:** `where = ["src"]` pekar på en katalog som inte finns,
   och `comparative_thinker.cli:main` refererar en `cli.py` som inte finns —
   `comparative-thinker`-entrypointen kan inte fungera.
3. **Dubbletter:** `09.3` och `09.3.1` är identiska; `07_dependency_graph/LEARNINGS.md`
   är en kopia av `06`:s (fel titel, samma 151 entiteter).
4. **`README.md`** beskriver bara Exp 09.3.1, inte projektet.
5. **`CODE_INDEX.md`** är en nyckelordslista utan nodvägar/hash — antingen
   generera den med GTW:s `code-index` eller ta bort den.

## Appendix D — Vad denna plan inte påstår

- Inga av koden i §3–§6 är byggd eller körd; exemplen är skisser mot
  verifierade typnamn, inte kompilerad kod.
- Buggrama i §2.3 är **kodläsningsfynd**, inte reproduktioner. S3 i §4.5 är
  steget som gör dem till bevis.
- Insatsbedömningarna (S/M/L) är relativa, inte kalenderdagar.

*Slut på planen.*














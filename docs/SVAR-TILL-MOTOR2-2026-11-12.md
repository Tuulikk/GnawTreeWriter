# Svar till Motor2 — 2026-11-12

Svar på `FEEDBACK_FRAN_MOTOR2_2026-11-12.md`. Punkt 1 var en riktig regression och
är **åtgärdad i GTW** — ni behöver inte anpassa F2d.

---

## 1. Doc-kommentarer vid `delete_node` — orsaken hittad, fixad

**Frågan var:** ändrade 0.26 `list_nodes`-utdatan så att konsumenter som går
baklänges genom nodlistan inte hittar kommentarerna?

**Svaret är nej — `list_nodes` ändrades inte.** Två mätningar:

1. Fältet `lines` har **aldrig funnits** i `list_nodes`. `git log -S'"lines"' --
   src/mcp/mod.rs` är tom. Payloaden är och har varit:
   ```json
   { "path": "...", "type": "...", "name": "...", "start": 12, "labels": [] }
   ```
   Det finns alltså ingen `end` och ingen `lines` — om er F2d läser `lines` har
   den läst något annat (eller antagit kontraktet ur dokumentation).

2. Regressionen kommer från **grammatiken**, inte från oss:
   `tree-sitter-rust` bumpades **0.21 → 0.24** (i 0.25.0). Från 0.24 får
   `line_comment` egna barn — exakt det ni såg i er probe:
   ```
   0 [line_comment]
   0.0 [//]
   0.1 [outer_doc_comment_marker]
   0.1.0 [/]
   0.2 [doc_comment]
   ```

**Varför det dödade F2d:** i en flat nodlista är posten *omedelbart före*
funktionen nu `1.2 [doc_comment]` — ett **barnbarn** till kommentarsnoden — i
stället för syskonet `1 [line_comment]`. Er `parent_of`-kontroll ser då rätt ut
i sak men fel i nivå, och kommentaren hittas inte; den stannar på disk efter en
lyckad delete.

**Fixen (GTW, inte Motor2):** kommentarer normaliseras till **löv** när trädet
byggs (`src/core/mod.rs::flatten_comment_children`, anropad från `new`, `new`-
lenty och re-parsen vid heal). Regeln är en och gäller alla språk, så en
grammatikbump inte kan omforma payloaden igen. Efter fixen, med er
minimi-fixture (doc + två fn):

```
  0 [line_comment] unnamed       ← syskon, ETT löv, path "0"
  1 [line_comment] unnamed
  2 [function_item] target
```

Posten före funktionen är åter syskonkommentaren. **F2d fungerar oförändrat**;
har ni redan lagt en workaround kan den tas bort.

Låst av testet `comment_nodes_are_leaves_after_construction` (parsar en fil via
publika Writer-vägen och kräver att kommentaren är löv *och* att noden före
funktionen är en kommentar med samma förälder).

**Kontraktet, uttalat:** en kommentar är EN nod med stabil `path` och `start`.
Vill ni ha `end` (för `start`–`end`-intervall) läggs det till additivt — säg
till, det är två rader och bryter inget.

---

## 2. `modernbert` → `ai`: alias tillagt

Ni hade rätt att det är en brytning som först syns som ett kryptiskt
resolutionsfel. **`modernbert = ["ai"]` finns nu som deprecated alias** och tas
bort efter nästa minor. `features = ["modernbert"]` löser alltså igen, men byt
till `ai` när det passar — aliaset är en övergång, inte en garanti.

---

## 3. Koordinering mitt i bygget

Noterat och rimligt att påpeka. GTW:s HEAD rörde sig fyra gånger under dygnet
medan ni path-dep:ade trädet. Rekommendationen från vår sida: **pinn:a en commit
eller en tagg** i er Cargo.toml under pågående arbete; våra taggar är de stabila
punkterna, `master` är arbetsyta.

---

## 4. Bonus som påverkar era siffror

Samma dag hittades och fixades en **entitetsregression**: en fil vars innehåll
ligger i en inline-modul (`pub mod x { ... }`) gav exakt **EN** entitet — modulen
— medan relationerna refererade funktionerna inuti. Uppmätt på `src/mcp/mod.rs`:
4 387 rader, 1 entitet, 1 791 kanter vars källa inte fanns i entitetstabellen.
Sådana kanter kan inte vandras (`impact`/`cycles` hoppar över dem).

Fixat genom rekursion genom `declaration_list`. **Konsekvens för konsumenter:**
entitetsantal och upplösningsgrad ändras (fler entiteter = fler kandidater = mer
ärlig, delvis lägre, upplösning). Index behöver byggas om en gång.

---

## 5. Tack

Punkt 4 uppskattas — särskilt att ni namngav CI-gaten och
`capabilities`-mönstret som användbara. Det som var värdefullast i er rapport var
att ni **körde** mot trädet i stället för att läsa changelog:en; det var så
grammatikregressionen hittades.

---

## 6. Era två nya fynd (2026-11-12) — reproducerade och åtgärdade

Båda är **riktiga**, båda reproducerades här innan något ändrades, och båda hade
en gemensam orsak som är värre än symptomen: `end_line` betyder *"raden som
innehåller nodens sista byte"*. För noder vars text slutar med `\n` — varje
kommentar, varje `use` — ligger den byten i början av **nästa** rad.

### 1. `Delete` svalde nästa rad

Repro (exakt er form, men med `use` i stället för funktion, eftersom det är där
vakten inte ser något):

```
//! hdr
(blank)
/// doc            ← nod 1, end_line = 4
use std::fmt;      ← rad 4
fn a() {}
```

`delete s.rs 1` gav:

```
//! hdr
(blank)
fn a() {}          ← use-raden BORTA, filen parsar fortfarande
```

Alltså: **tyst kodförlust**, precis som ni beskrev — och syntaxvakten kunde inte
fånga den eftersom resultatet är giltig Rust. När målet var en funktion ([5,7])
råkade det bli rätt, eftersom en funktion inte slutar med `\n`; det är därför
buggen kunde leva.

**Fix:** delete beräknar nu den effektiva slutraden: slutar nodens text med `\n`
dras en rad av (`end_line - 1`), annars används `end_line`. Test:
`delete_of_a_newline_terminated_comment_keeps_the_next_line`.

**Ert arbete behövs fortfarande:** `delete` raderar *inte* doc-kommentarer ovanför
målet (det är er F2d-kedja, en sanning per regel §45). Fixen gör bara att varje
enskild radering träffar exakt sin egen text.

### 2. `insert_node(position utanför intervallet)` rapporterade framgång

Vi hittade **två** varianter, inte en:

| Fall | Gammalt beteende |
|---|---|
| position > antal barn, klammerförälder (`mod m { … }`) | hamnade **efter stängande klammer** — utanför modulen |
| position > antal barn, **lövnod** (`identifier`) | hamnade **överst i filen** (`start_line - 1`) |

Båda svarade "Content inserted" med diff och transaktions-id.

**Fix:** position utanför intervallet klampas nu till förälderns eget slut — *före*
stängande klammer för klammerföräldrar, filens slut för rotnoden, *efter* lövnoden
för lövföräldrar — och **säger det** på stderr:

```
⚠️  insert: position 999 exceeds 3 child slot(s) — clamped to append
⚠️  insert: parent '0.1' (identifier) has no children — inserting AFTER it, not inside it
```

Test: `insert_beyond_the_child_count_clamps_inside_the_parent`.

**Ert förslag var "neka eller klampa med ärlig notis"** — vi valde klampning,
eftersom er brygga skickar `children.len() + 1` som "lägg sist" och ett nekande
skulle bryta det legitima fallet. Notisen är det som gör klampningen till en
deklaration i stället för en tystnad.

### Vad som INTE ändrades

`end_line`-semantiken själv är orörd (många konsumenter läser den). Vi rättade de
två ställena som använde den som **slice-gräns**, och skrev in fällan i
kommentarerna. Att normalisera `end_line` är en egen, större ändring — den står i
`uppslag.md` som en öppen tråd, med exakt den här typen av symptom som
motivering.


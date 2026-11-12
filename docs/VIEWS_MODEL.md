# Vyer som lager: tröskel, opacitet, ordning

**Status:** modell skriven 2026-11-12, ingen kod byggd. Detta är pappret som ska
göra nästa steg självklart — inte en spec som ska byggas i sin helhet.

---

## Varför idén kom

Från fotoredigering, och den är inte en liknelse utan en **kompositionsmodell**:
en kopia av bilden körs genom ett tröskelvärde → blir svartvit, lagret görs
genomskinligt, och proceduren upprepas med nya trösklar. Genom att stapla N lager
väljer man ut exakt de gråskalor man vill ha, **i den ordning man vill ha dem**.
Kurvor läggs på kopian, förbehandling eller sammanslagningen.

Det avgörande är transparensen: **genomsläpp på rätt ställen**. Är varje lager
halvgenomskinligt överallt blir bilden grumlig — för mörk, för ljus, eller utan
detaljer. Lagren måste vara *opaka där de är säkra och frånvarande där de inte
har något att säga*.

Översatt: en **vy** är en deklarerad lagerstack över samma lagrade struktur. Varje
titt kan framkallas på nytt, och en standardbild är bra att ha — men lägena måste
kunna justeras.

---

## Vad som redan finns (och därför inte ska uppfinnas igen)

| Byggsten | Var | Vad den ger |
|---|---|---|
| Fyra kanaler med viktning | `sense` → `RerankedHit` (`cosine`, `lex`, `graph`, prior) | en hårdkodad lagerstack |
| Sammanslagning | `fuse_by_max` / RRF | **en** implicit ordning |
| Kurva | `SENSE_ANSWERABLE_COSINE` | en global monoton remap |
| Frånvaro ≠ noll | `graph: Option<f32>` — *"Absence is not agreement"* | regeln finns, på **ett** ställe |
| Nämnare per urval | `local_resolution`, `--coverage --scope` | rate + denominator i samma mängd |
| Tidslager | `--onion`, `relation_changes` | kanaler över revisioner |
| Lagerfärskhet | `index_levels` | vilket lager som är gammalt |

Slutsatsen: kompositen finns. Det som saknas är att den är **deklarerad,
namngiven och justerbar** — i stället för inbränd i `sense`.

---

## Mappningen

| Fotografi | GTW | Term |
|---|---|---|
| Tröskelvärde | predikat över kanter | `threshold` |
| Opacitet | hur mycket kanalen får bidra | `weight` |
| Lagerordning | precedence när kanaler säger olika | `order` |
| Frånvaro vs noll | `Option`, inte `0.0` | `absent` |
| Kurva | monoton remap av kompositen | `curve` |
| Standardbild | förvald vy | `default` |
| Lägen | namngivna vyer | `view` |
| Framkalla igen | vy appliceras vid läsning | — |

---

## Invarianter

1. **En vy ändrar aldrig vad som lagras.** Den är en lins. Bakas en vy in i
   storen måste man indexera om för att byta ljus — det är samma fel som att
   lagra ett utvärderat tillstånd i stället för keys (`docs/ONION_SKIN_MODEL.md`).
2. **Frånvaro är inte noll.** En kanal utan data bidrar inte alls. `0.0` är ett
   *påstående* om lågt värde, inte om okunskap — och blandar man in det dras
   kompositen mot fel håll. Detta är `graph: Option<f32>` generaliserat.
3. **Varje vy redovisar sin egen nämnare.** En vy som filtrerar bort hälften av
   kanterna får inte citera det globala 43 %. Samma regel som `--scope`.
4. **Ordningen ska vara explicit.** `fuse_by_max` är *ett* val, inte det
   självklara; en vy ska säga om den tar max, summa eller precedence.
5. **En vy är deterministisk och modellfri.** Samma store + samma vy ⇒ samma
   utdata. Vikter får komma från en människa, aldrig från en modell i tysthet.
6. **En vy får inte skapa ett eget index.** Då är det en struktur, inte en lins.

---

## Minsta användbara skiva

`--view <namn>` som **bara väljer och ordnar befintliga utdata**, med egen
täckningsrad. Ingen ny tabell, inget nytt index.

```yaml
# views.yaml (förslag: committad i repot, så vyer kan granskas i diff)
strict:
  description: bara kanter som ett orakel har löst upp
  layers:
    - channel: calls
      threshold: "resolved_by = scip"      # predikat
      weight: 1.0                          # opak
    - channel: uses
      threshold: "resolved_by = scip"
      weight: 1.0
  order: precedence
  curve: null
  reports: own_coverage

explore:
  description: allt som finns, men namnburna kanter väger lätt
  layers:
    - channel: calls
      threshold: "any"
      weight: 1.0
    - channel: calls_name_only
      threshold: "resolved_by = name"
      weight: 0.35                         # genomsläpp, inte frånvaro
  order: sum
  reports: own_coverage
```

Tre lager räcker för att bevisa modellen. Resten är konfiguration.

---

## Vad det här INTE får bli

- **Ingen vikt-tuning som blir en dold modell.** Vikter ska vara läsbara i
  `views.yaml`, inte härledda ur data i tysthet. Om en vikt behöver kalibreras
  ska det göras med ett mätharness (som `ai recall-eval`), inte genom att smyga
  in fler kanaler.
- **Inga vyer som kräver modell.** `sense` kan vara *en kanal i en vy*, inte ett
  krav för att vyn ska existera.
- **Ingen "bästa vy".** En vy är ett syfte. `strict` och `explore` svarar på
  olika frågor och får ge olika svar på samma data.

---

## Öppna frågor innan bygget

1. **Var bor vyerna?** Projekt (`<root>/views.yaml`, granskningsbar i diff) eller
   användare (`~/.config`)? Förslag: projekt, med användaroverride.
2. **Hur interagerar vy och `--scope`?** Förslag: scope är ett *crop* som
   appliceras före lagren, och nämnaren rapporteras efter båda.
3. **Vad händer när två vyer ger olika svar?** Inget — men svaret ska bära
   `view: strict` så det aldrig kan citeras som "sanningen".
4. **Ska `--onion` vara en vy eller en egen flagga?** Förslag: egen flagga
   (tidsdimensionen är inte en kanal), men vyerna ska kunna begränsa *vilka*
   kanaler som visas över tid.

---

## Acceptanskriterier för skivan

- [ ] Samma store + samma vy ⇒ byte-identisk utdata (determinism).
- [ ] En kanal med `threshold: "none"` bidrar **inte alls** (test: kompositen
      ändras inte när kanalen saknar data, och ändras när den får data).
- [ ] Täckningsraden i en vy räknar **bara** de kanter vyn släpper igenom.
- [ ] Att byta vy kräver **ingen** omindexering (storens `generation_key` är
      oförändrad efter två körningar med olika vyer).

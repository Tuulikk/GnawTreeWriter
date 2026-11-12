# Onion skin-modellen: vad Krita och Blender lär oss

**Status:** `--onion` och `--history` byggda 2026-11-12. Detta dokument fångar
*varför* de ser ut som de gör, och vad som medvetet inte är byggt.

---

## Krita: ghosting är relativt en subject

Krita visar närliggande frames i **samma animerade lager**, tonade, med avtagande
opacitet (1–2 frames före/efter). Man ser **rörelse**, inte detaljer — silhuetter
räcker.

Översatt: en onion skin är alltid *om något*. Därför är `--onion` filtrerad
(`--onion <substr>`) och aldrig en global spökbild av hela grafen. Att visa allt
är samma misstag som att visa hela filen när man frågar var en rad är — tekniskt
korrekt, oanvändbart.

## Blender: kanaler, keyframes, och evaluering som aldrig lagras

| Blender | GTW | Status |
|---|---|---|
| **F-curve**: en kanal per egenskap på en datablock | `relation_key` = `from_id\|relation_type\|to_name` | ✅ |
| **Keyframe**: värdet vid en tidpunkt | `relation_changes`-rad per revision (`count_before`/`count_after`) | ✅ |
| **Frame = ett tillstånd**, inte gapet före | `--onion`-frame = `count_after` vid revisionen | ✅ |
| **Interpolation mellan keyframes** | steg-funktion (värden ändras bara vid refresh) | medvetet inte |
| **NLA-strips** (lager av tidsintervall) | `--frames N` (fönster), ingen sammanslagning av intervall | delvis |
| **Dependency graph med taggar** (COPY_ON_WRITE, GEOMETRY …) | L0→L1→L2-stegen + `staleness_queue` | grovt; Blender är bättre |
| **Drivers** (beräknade egenskaper) | `impact` / `reachable` / `cycles` — beräknas, lagras aldrig | ✅ samma princip |
| **Datablock users / fake user** | `relations` + revisioner | ✅ |

**Den viktigaste lärdomen:** Blender lagrar **keys**, aldrig det utvärderade
tillståndet. Scenen räknas fram ur kanalerna varje gång. GTW gör samma sak:
`relation_changes` är keys, `--onion` rekonstruerar tillstånden, och `impact`
beräknas — det finns ingen "påverkanscache" som kan bli inaktuell.

Att lagra det utvärderade tillståndet hade gett en cache som *ser* snabb ut och
ljuger så fort något ändras utan att cachen uppdateras. Det är exakt den klassen
av fel som `file_hash`-buggen och fantomstoren tillhörde.

---

## Vad som inte är byggt, och varför

1. **Interpolation.** Vi har steg, inte kurvor. Det är rätt: en kant ändras bara
   när en fil parsas om, så mellan två revisioner finns inget tillstånd att
   interpolera. Skulle vi någon gång kedja revisioner med okänt mellanrum blir
   interpolation meningslöst snarare än fint.
2. **"På väg att bli"-framen.** Arbetskopian är inte indexerad, alltså finns
   framtiden inte i databasen. Att materialisera den kräver att de smutsiga
   filerna parsas i minnet (*inte* en DB-kopia — se uppslag 2026-11-12).
3. **En visuell yta.** Det här är datan; ingen rendering. Blender/Krita är
   grafiska verktyg — GTW:s motsvarighet är JSON + CLI, och en framtida
   visualisering ska bygga på samma kanaler, inte på en egen modell.
4. **Ghosting över flera subjects samtidigt.** Krita visar en layers frames. Vi
   visar en filtrerad mängd kanaler. Om det behövs är det ett filter, inte en
   ny datastruktur.

---

## Konsekvens för `--pending` (nästa steg)

Samma form som `--onion`, men sista framen kommer från en in-memory-beräkning av
de smutsiga filerna i stället för från storen. Kanalerna är desamma, jämförelsen
är densamma — bara källan till sista värdet skiljer. Det betyder att `--pending`
**inte** får bli en separat kodväg med egen semantik; den ska återanvända
`OnionChannel` och bara byta ut `now`.

# 🦥 The Lazy Guide to GnawTreeWriter
*How to gnaw through code with minimal effort and maximum precision.*

## 💡 Varför GnawTreeWriter? (The "Gnaw" Philosophy)
Traditionella verktyg (`sed`, `grep`, `replace`) ser kod som textsträngar. Det är farligt och trögt. GnawTreeWriter ser kod som ett **träd (AST)**.
- **Surgical Precision:** Ändra en specifik funktion utan att riskera att du råkar ändra något annat med samma namn.
- **Auto-Safety:** Verktyget vägrar applicera ändringar som bryter syntaxen.
- **Context Efficient:** Läs bara den nod du behöver (t.ex. en metod), istället för att ladda in tusentals rader i AI-modellen.
- **Time Travel:** Inbyggd `undo`, `redo` och `restore-session` som fungerar oberoende av Git.

## 🧠 GnawSense: Din semantiska kompass
GnawSense (ModernBERT) låter dig söka efter *logik*, inte bara tecken.
- **Hitta kod:** `gnawtreewriter sense "hur hanteras backups?"` – sök i hela projektet utan att veta filnamnet.
- **Smart insättning:** `gnawtreewriter sense-insert --file main.rs --anchor "där loggarna roteras" --content "println!(\"Roterar!\");"` – låt AI:n hitta rätt plats för din kod.

## 🚀 Snabbguide för den lata
1. **Få överblick:** `gnawtreewriter skeleton <fil>` (visar bara defs, inget brus).
2. **Hitta målet:** `gnawtreewriter list <fil> --filter-type function_definition`.
3. **Läs kirurgiskt:** `gnawtreewriter show <fil> "0.1.2"` (läs bara exakt den noden).
4. **Redigera:** `gnawtreewriter edit <fil> "0.1.2" --source-file ny_kod.rs` (säkrare än att escapa strängar i terminalen).

## 🛠 Smarta Agent-tricks
- **Använd STDIN:** För att undvika problem med shell-escaping när du skickar kod, använd `-`:
  `cat ny_kod.txt | gnawtreewriter edit main.rs "0.1" -`
- **Tagga dina noder:** Om du ska göra många ändringar, tagga noden först:
  `gnawtreewriter tag add main.rs "0.5.1" "min_motor"`
  Sedan kan du köra: `gnawtreewriter edit main.rs tag:min_motor "ny kod"`
  (Även om koden ovanför ändras och radnummer flyttas, hittar GnawTreeWriter rätt!)
- **Batch-körning:** Samla alla ändringar i en JSON och kör `gnawtreewriter batch ops.json --preview`. Allt eller inget appliceras.

## 🛡 Guardian Mode
Om du råkar radera för mycket kod eller gör en ändring som ser ut att förstöra projektet, kommer **Guardian** att blockera ändringen. Använd `--force` bara om du är absolut säker, annars lita på verktygets omdöme.

---
*Remember: Allting är relativt, men en trasig AST är absolut dålig. Gnaw on!*

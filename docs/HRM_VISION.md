# Vision: GnawSense med HRM 2.0 (Hierarchical Reasoning Model)

Denna vision beskriver nästa generations intelligens i GnawTreeWriter, där vi går från semantisk sökning till arkitektonisk förståelse.

## 1. Från Platt Sökning till Hierarkisk Resonans
Idag jämför GnawSense flat-vektorer. En HRM-modell förstår att en kodnods betydelse beror på dess kontext i trädet.
*   **Contextual Path Embedding:** Boten ser skillnaden mellan en `login`-funktion i en test-modul och en i säkerhets-kärnan baserat på dess "föräldra-arv".

## 2. "The Duplex Loop" (Iterativ Självkorrigering)
Inspirerat av *Comparative-Thinker* implementeras en loop där varje ändring valideras mot både syntax och semantik.
*   **Pass 1:** Snabb sökning (ModernBERT).
*   **Pass 2:** Resonemang mot projektets Knowledge Graph för att verifiera arkitektonisk logik.

## 3. Side-effect Prediction (Gap-varningar) 🚀
Detta är den mest kritiska förmågan. När en ändring planeras kan GnawSense förutse var i projektet det kommer att uppstå "logiska hål".
*   **Exempel:** Vid en `sense-insert` av en ny nätverks-check kan boten varna: *"Jag ser att du ändrar nätverksflödet. Detta kräver sannolikt en uppdatering av Config-structen i `settings.rs` för att inte bryta bakåtkompatibilitet. Ska jag förbereda ett ankare där?"*

## 4. Strukturell Stil-överföring (Personalized Style)
HRM-modellen lär sig användarens specifika sätt att bygga träd (var man lägger felhantering, hur man strukturerar moduler).
*   **Normalization:** Inkommande kod från externa AI-agenter "tvättas" och omstruktureras för att matcha din personliga arkitektoniska stil innan den appliceras.

---

*Detta dokument fungerar som en ledstjärna för utvecklingen av Phase 5 och framåt i ROADMAP.md.*

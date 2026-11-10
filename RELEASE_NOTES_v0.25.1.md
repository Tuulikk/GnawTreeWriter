# GnawTreeWriter — Release Notes (v0.25.1)

**Datum:** 2026-11-10
**Typ:** Patch (CLI-beteende; en brytning för skript som pipe:ar `--db`)

## Sammanfattning

En liten men efterlängtad upprensning: **`code-index --db` skriver
`<root>/code-index.md` i stället för att dumpa hela dokumentet i terminalen.**

Bakgrunden är en inkonsekvens som syntes i help-texten: `--check` hade redan
defaulten `<root>/code-index.md`, men *skrivaren* krävde `-o <path>`. Tre
ställen kände filnamnet — grinden, hooken och operatören — och bara ett av dem
gjorde det till en default. Resultatet var att varje refresh antingen dumpade
~2 300 rader markdown till stdout eller krävde att man upprepade en sökväg som
verktyget redan kände.

## Changes

- `code-index <root> --db` → skriver `<root>/code-index.md`, en rad bekräftelse.
- `code-index <root> --db --stdout` → skriver inget, skriver ut (gamla läget).
- `code-index <root>` utan `--db` → oförändrat (stdout, pipebart).
- Hooken anropar `code-index "$top" --db` utan `-o`; genereras om vid
  `--install-hook`.

Nya tester: `db_refresh_writes_the_artifact_by_default`,
`db_refresh_with_stdout_prints_and_writes_nothing` och
`plain_code_index_still_prints_to_stdout` (den sista är en regressionsvakt mot
att defaulten läcker in i den vanliga renderaren).

## Upgrade

```bash
cargo install --path . --features modernbert,mcp
gnawtreewriter code-index . --install-hook   # hooken utan -o
gnawtreewriter code-index . --db            # skriver code-index.md
```

**Brytande:** `code-index . --db > fil.md` skriver inte längre till stdout.
Använd `--stdout` eller `-o <path>`.

## Known

Oförändrat från 0.25.0: JSON på interim-spåret, `.h` → C-parsern, 73
self-loops i cykeldetektionen, och `cargo fmt --check`-drift i äldre filer.

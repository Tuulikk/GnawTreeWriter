# Gnaw-Notes Development Log

This log documents the building of the Gnaw-Notes app using **GnawTreeWriter**. It demonstrates the temporal safety and structural precision of the tool.

---

## Phase 1: Initial Simple Note-taker
**Goal:** Create a script that appends text to a file.

### Step 1: Baseline
Created `notes.py` with basic `save_note` function.

---

## Phase 2: Adding Temporal Metadata
**Goal:** Use GnawTreeWriter to inject a timestamp feature.

### Step 2: Analysis
We run `gnawtreewriter list` to find the insertion point.
```bash
gnawtreewriter list examples/temporal-demo/notes.py
```

### Step 3: Injection
We inserted `import datetime` over the module. (See `snapshot_1.json`).

### Step 4: Logic Update
We targeted node `1.4.0.3.0` (the write statement) and replaced it with timestamped logic.
```python
f.write(f"[{datetime.datetime.now()}] {text}\n")
```
(See `snapshot_2.json` for the state before this logic update).

---

## Phase 3: The Time Machine Demo
**Goal:** Demonstrate how to roll back changes using GnawTreeWriter's temporal features.

### Step 5: Restore to Phase 1
If we want to go back to the simple note-taker without timestamps, we can use the project-wide restoration.

Run:
```bash
gnawtreewriter restore-project "2026-01-11 15:00:00" --preview
```

This will show you that `notes.py` will be reverted to its original state. Removing the `--preview` flag will actually perform the "time travel".

---

## Why is this better than Git?
1. **Automatic:** You didn't have to `git commit` at every step. GnawTreeWriter took snapshots for you.
2. **Structural:** You can target specific nodes (like just one function) for restoration if you want.
3. **No Mess:** Your git history remains clean, while your local development has perfect "black box" recording.

---

## History Snapshots
The `history_snapshots/` directory contains JSON files showing exactly what GnawTreeWriter saved at each step. These are the building blocks of the Time Machine.
